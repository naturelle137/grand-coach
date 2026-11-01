// SPDX-License-Identifier: EUPL-1.2

//! Sharded, resumable storage of self-play records (#70).
//!
//! A run directory holds `manifest.json` and one shard per range of
//! [`Manifest::shard_deals`] deals: `shard-NNNNN.jsonl` (one
//! [`DecisionRecord`] per line, deals in order) and `shard-NNNNN.done`
//! ([`ShardInfo`] with the SHA-256 of the data). A shard counts as complete
//! only if its `.done` file matches the data file; it is written to a
//! temporary file and renamed, and the `.done` file comes last, so an
//! interrupted shard is never read as valid and is redone on restart.
//!
//! Every run has a wall-time and a disk cap and a CPU budget: game workers
//! times analysis threads per worker never exceed the declared number of
//! CPUs ([`skat_analysis::with_max_threads`]).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use skat_core::RuleConfig;
use skat_nn::{ENCODER_VERSION, sha256};

use crate::selfplay::{
    DecisionKind, DecisionRecord, RECORD_FORMAT, RULES, RunSeed, TeacherProfile, play_deal,
};

/// Version of the directory layout and manifest.
pub const DATASET_FORMAT: u32 = 1;

/// Everything that determines the records of a run. A restart must use the
/// same manifest; caps and the CPU budget may change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// [`DATASET_FORMAT`].
    pub dataset_format: u32,
    /// [`RECORD_FORMAT`].
    pub record_format: u32,
    /// Encoder version at generation time (informational: records are
    /// raw, encoding happens at training time).
    pub encoder_version: u16,
    /// Engine commit (`git describe`), `-dirty` if the tree had changes.
    pub engine_commit: String,
    /// Teacher profile name.
    pub profile: String,
    /// The full teacher profile (`Debug` form).
    pub profile_detail: String,
    /// Run seed (bookkeeping; never a model input).
    pub seed: u64,
    /// How deal and teacher seeds derive from the run seed.
    pub seed_scheme: String,
    /// Rules.
    pub rules: RuleConfig,
    /// Deals of the run: `0..deals`.
    pub deals: u64,
    /// Deals per shard.
    pub shard_deals: u64,
    /// Split rule.
    pub split: String,
}

impl Manifest {
    /// The manifest of a run.
    pub fn new(
        engine_commit: &str,
        teacher: TeacherProfile,
        seed: u64,
        deals: u64,
        shard_deals: u64,
    ) -> Manifest {
        Manifest {
            dataset_format: DATASET_FORMAT,
            record_format: RECORD_FORMAT,
            encoder_version: ENCODER_VERSION,
            engine_commit: engine_commit.to_string(),
            profile: teacher.name.to_string(),
            profile_detail: format!("{:?}", teacher.profile),
            seed,
            seed_scheme: "deal: mix(mix(seed, DEAL_KEY), deal); teacher: \
                          mix(mix(mix(seed, TEACHER_KEY), deal), seat); SplitMix64 finaliser"
                .to_string(),
            rules: RULES,
            deals,
            shard_deals: shard_deals.max(1),
            split: "by deal seed: train 80 %, validation 10 %, test 10 %".to_string(),
        }
    }

    /// Number of shards.
    pub fn shards(&self) -> u64 {
        self.deals.div_ceil(self.shard_deals)
    }

    /// Deals of shard `index`.
    pub fn shard_range(&self, index: u64) -> std::ops::Range<u64> {
        let start = index * self.shard_deals;
        start..(start + self.shard_deals).min(self.deals)
    }
}

/// Contents of a `.done` file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardInfo {
    /// First deal.
    pub first_deal: u64,
    /// One past the last deal.
    pub end_deal: u64,
    /// Number of records.
    pub records: u64,
    /// Size of the data file in bytes.
    pub bytes: u64,
    /// SHA-256 of the data file, hexadecimal.
    pub sha256: String,
}

/// Why a shard cannot be read.
#[derive(Debug)]
pub enum ShardError {
    /// No `.done` file: the shard was never finished.
    Incomplete,
    /// The data does not match its `.done` file.
    Corrupt(String),
    /// Reading failed.
    Io(io::Error),
}

impl std::fmt::Display for ShardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardError::Incomplete => write!(f, "shard incomplete"),
            ShardError::Corrupt(why) => write!(f, "shard corrupt: {why}"),
            ShardError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for ShardError {
    fn from(e: io::Error) -> ShardError {
        ShardError::Io(e)
    }
}

/// Path of the data file of shard `index`.
pub fn shard_path(dir: &Path, index: u64) -> PathBuf {
    dir.join(format!("shard-{index:05}.jsonl"))
}

fn done_path(dir: &Path, index: u64) -> PathBuf {
    dir.join(format!("shard-{index:05}.done"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The verified `.done` information and data of shard `index`.
fn verified(
    dir: &Path,
    manifest: &Manifest,
    index: u64,
) -> Result<(ShardInfo, Vec<u8>), ShardError> {
    let done = match fs::read(done_path(dir, index)) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(ShardError::Incomplete),
        Err(e) => return Err(e.into()),
    };
    let info: ShardInfo =
        serde_json::from_slice(&done).map_err(|e| ShardError::Corrupt(e.to_string()))?;
    let range = manifest.shard_range(index);
    if (info.first_deal, info.end_deal) != (range.start, range.end) {
        return Err(ShardError::Corrupt(
            "deal range differs from the manifest".into(),
        ));
    }
    let data = match fs::read(shard_path(dir, index)) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(ShardError::Corrupt("data file missing".into()));
        }
        Err(e) => return Err(e.into()),
    };
    if data.len() as u64 != info.bytes || hex(&sha256(&data)) != info.sha256 {
        return Err(ShardError::Corrupt("checksum mismatch".into()));
    }
    Ok((info, data))
}

/// Reads the run's manifest.
pub fn read_manifest(dir: &Path) -> io::Result<Manifest> {
    let bytes = fs::read(dir.join("manifest.json"))?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

/// Reads the records of a complete shard; incomplete or corrupt shards are
/// rejected.
pub fn read_shard(dir: &Path, index: u64) -> Result<Vec<DecisionRecord>, ShardError> {
    let manifest = read_manifest(dir)?;
    let (info, data) = verified(dir, &manifest, index)?;
    let records = parse_records(&data)?;
    if records.len() as u64 != info.records {
        return Err(ShardError::Corrupt("record count differs".into()));
    }
    Ok(records)
}

fn parse_records(data: &[u8]) -> Result<Vec<DecisionRecord>, ShardError> {
    data.split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).map_err(|e| ShardError::Corrupt(e.to_string())))
        .collect()
}

/// State of a run directory ([`verify`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verified {
    /// Complete shards whose records all pass
    /// [`check_record`](crate::selfplay::check_record).
    pub complete: u64,
    /// Missing or incomplete shards.
    pub incomplete: u64,
    /// Records in complete shards.
    pub records: u64,
}

/// Reads every shard of the run in `dir` and checks its records. Corrupt
/// shards and unusable records are errors; incomplete shards are counted.
pub fn verify(dir: &Path) -> Result<Verified, ShardError> {
    let manifest = read_manifest(dir)?;
    let mut out = Verified::default();
    for index in 0..manifest.shards() {
        match read_shard(dir, index) {
            Ok(records) => {
                for r in &records {
                    if !manifest.shard_range(index).contains(&r.deal) {
                        return Err(ShardError::Corrupt(format!(
                            "deal {} in shard {index}",
                            r.deal
                        )));
                    }
                    crate::selfplay::check_record(r)
                        .map_err(|e| ShardError::Corrupt(format!("shard {index}: {e}")))?;
                }
                out.complete += 1;
                out.records += records.len() as u64;
            }
            Err(ShardError::Incomplete) => out.incomplete += 1,
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// Serialises records as JSON lines.
pub fn to_jsonl(records: &[DecisionRecord]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in records {
        serde_json::to_writer(&mut out, r).expect("records serialise");
        out.push(b'\n');
    }
    out
}

/// Caps and CPU budget of one invocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// Wall-time cap: no deal starts after it; the shard in progress is
    /// dropped and redone on restart.
    pub wall_time: Duration,
    /// Disk cap for the run directory in bytes: no deal starts once the
    /// directory plus the shards in progress reach it.
    pub disk_bytes: u64,
    /// Declared CPU budget: game workers times analysis threads.
    pub cpus: NonZeroUsize,
    /// Game workers (shards played side by side); at most `cpus`.
    pub workers: NonZeroUsize,
}

impl Limits {
    /// Analysis threads per worker: `cpus / workers`, at least one; with
    /// at most `cpus` workers their product stays within the budget.
    pub fn threads(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.cpus.get() / self.workers.get()).unwrap_or(NonZeroUsize::MIN)
    }
}

/// Why a run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Every shard is complete.
    Finished,
    /// The wall-time cap was reached.
    WallTime,
    /// The disk cap was reached.
    Disk,
}

/// What one invocation did.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Shards that were already complete and skipped.
    pub skipped: u64,
    /// Shards written now.
    pub written: u64,
    /// Deals played in written shards.
    pub deals: u64,
    /// Records in written shards.
    pub records: u64,
    /// Bytes of written shards.
    pub bytes: u64,
    /// Decisions per kind in written shards.
    pub kinds: BTreeMap<DecisionKind, u64>,
    /// Decisions with an incomplete teacher analysis.
    pub incomplete: u64,
    /// Wall time of the invocation.
    pub elapsed: Duration,
    /// Why it stopped.
    pub stop: Stop,
}

/// Size of all files in `dir`.
fn dir_bytes(dir: &Path) -> io::Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let meta = entry?.metadata()?;
        if meta.is_file() {
            total += meta.len();
        }
    }
    Ok(total)
}

/// Writes `bytes` to `path` through a temporary file and a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    {
        use std::io::Write;
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// Generates or resumes the run in `dir`. A new directory gets
/// `manifest`; an existing one must hold the same manifest. Complete
/// shards are skipped; missing, incomplete and corrupt ones are played and
/// written. Records do not depend on `limits`.
pub fn generate(
    dir: &Path,
    manifest: &Manifest,
    teacher: TeacherProfile,
    limits: Limits,
) -> io::Result<Report> {
    let start = Instant::now();
    if manifest.profile != teacher.name
        || manifest.profile_detail != format!("{:?}", teacher.profile)
    {
        return Err(io::Error::other("manifest and teacher profile differ"));
    }
    if limits.workers > limits.cpus {
        return Err(io::Error::other(
            "more game workers than CPUs in the budget",
        ));
    }
    fs::create_dir_all(dir)?;
    match read_manifest(dir) {
        Ok(existing) if existing == *manifest => {}
        Ok(existing) => {
            return Err(io::Error::other(format!(
                "{} holds another run: {existing:?}",
                dir.display()
            )));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut bytes = serde_json::to_vec_pretty(manifest).map_err(io::Error::other)?;
            bytes.push(b'\n');
            write_atomic(&dir.join("manifest.json"), &bytes)?;
        }
        Err(e) => return Err(e),
    }

    let mut todo = Vec::new();
    let mut skipped = 0;
    for index in 0..manifest.shards() {
        match verified(dir, manifest, index) {
            Ok(_) => skipped += 1,
            Err(ShardError::Io(e)) => return Err(e),
            Err(_) => todo.push(index),
        }
    }

    let seed = RunSeed(manifest.seed);
    let base_bytes = dir_bytes(dir)?;
    let pending = AtomicU64::new(0);
    let next = AtomicU64::new(0);
    let stop = Mutex::new(None);
    let halted = AtomicBool::new(false);
    let report = Mutex::new(Report {
        skipped,
        written: 0,
        deals: 0,
        records: 0,
        bytes: 0,
        kinds: BTreeMap::new(),
        incomplete: 0,
        elapsed: Duration::ZERO,
        stop: Stop::Finished,
    });
    let error: Mutex<Option<io::Error>> = Mutex::new(None);
    let halt = |why: Stop| {
        halted.store(true, Ordering::SeqCst);
        stop.lock().expect("lock").get_or_insert(why);
    };

    std::thread::scope(|scope| {
        for _ in 0..limits.workers.get() {
            scope.spawn(|| {
                skat_analysis::with_max_threads(limits.threads(), || {
                    loop {
                        if halted.load(Ordering::SeqCst) {
                            return;
                        }
                        let i = next.fetch_add(1, Ordering::SeqCst) as usize;
                        let Some(&index) = todo.get(i) else { return };
                        let mut records = Vec::new();
                        let mut size = 0u64;
                        for deal in manifest.shard_range(index) {
                            if start.elapsed() >= limits.wall_time {
                                halt(Stop::WallTime);
                            } else if base_bytes + pending.load(Ordering::SeqCst)
                                >= limits.disk_bytes
                            {
                                halt(Stop::Disk);
                            }
                            if halted.load(Ordering::SeqCst) {
                                pending.fetch_sub(size, Ordering::SeqCst);
                                return;
                            }
                            let deal_records = play_deal(seed, teacher.profile, deal);
                            let bytes = to_jsonl(&deal_records).len() as u64;
                            size += bytes;
                            pending.fetch_add(bytes, Ordering::SeqCst);
                            records.extend(deal_records);
                        }
                        let data = to_jsonl(&records);
                        let info = ShardInfo {
                            first_deal: manifest.shard_range(index).start,
                            end_deal: manifest.shard_range(index).end,
                            records: records.len() as u64,
                            bytes: data.len() as u64,
                            sha256: hex(&sha256(&data)),
                        };
                        let written = write_atomic(&shard_path(dir, index), &data).and_then(|()| {
                            let mut done = serde_json::to_vec(&info).map_err(io::Error::other)?;
                            done.push(b'\n');
                            write_atomic(&done_path(dir, index), &done)
                        });
                        if let Err(e) = written {
                            error.lock().expect("lock").get_or_insert(e);
                            halted.store(true, Ordering::SeqCst);
                            return;
                        }
                        let mut r = report.lock().expect("lock");
                        r.written += 1;
                        r.deals += info.end_deal - info.first_deal;
                        r.records += info.records;
                        r.bytes += info.bytes;
                        for rec in &records {
                            *r.kinds.entry(DecisionKind::of(&rec.view)).or_default() += 1;
                            r.incomplete += u64::from(matches!(
                                rec.teacher.status,
                                Some(s) if s != skat_ai::Status::Complete
                            ));
                        }
                    }
                });
            });
        }
    });

    if let Some(e) = error.into_inner().expect("lock") {
        return Err(e);
    }
    let mut report = report.into_inner().expect("lock");
    report.elapsed = start.elapsed();
    report.stop = stop.into_inner().expect("lock").unwrap_or(Stop::Finished);
    Ok(report)
}
