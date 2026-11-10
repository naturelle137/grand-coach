// SPDX-License-Identifier: EUPL-1.2

//! On-disk cache of loaded training data (`nn-train train --cache DIR`).
//!
//! Loading parses, checks and encodes every record (minutes for large
//! sets). The cache stores the result ([`Data`] with compact feature rows)
//! in one binary file per data set and target settings, read back in
//! seconds. Its name is a key over everything that determines the data:
//! [`CACHE_FORMAT`], the encoder version, every source run (manifest and
//! the shard hashes of its `.done` files, in the order given) and the
//! hyperparameters the targets use. Changed or added shards give a new
//! key; the source run directories are only read, never written.
//!
//! A hit trusts the `.done` hashes (as [`crate::shards::read_shard`] does
//! after checking the data once); a cache file that does not parse is
//! rebuilt. Bump [`CACHE_FORMAT`] whenever the file layout or the target
//! computation of [`Examples`] changes.

use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use skat_nn::{ACTIONS, ENCODER_VERSION, FEATURES, sha256};

use crate::features::FeatureCodes;
use crate::selfplay::{Player, Roles};
use crate::shards::{read_manifest, shard_digest};
use crate::train::{
    Coverage, Data, DataSource, Examples, Hyper, TRAIN_FORMAT, hex_hash, load_with,
};

/// Version of the cache file layout and of the cached target computation.
pub const CACHE_FORMAT: u32 = 2;

const MAGIC: &[u8; 8] = b"GCDATA\0\x01";

/// How [`load_cached`] got the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheUse {
    /// Read from the cache file.
    Hit,
    /// Loaded from the runs and written to the cache.
    Written,
}

/// What determines the cached data.
#[derive(Serialize)]
struct KeyInput<'a> {
    cache_format: u32,
    train_format: u32,
    encoder_version: u16,
    sources: &'a [DataSource],
    soft_weight: u32,
    soft_temperature: u32,
    value_mix: u32,
    beliefs: bool,
}

/// The sources of `dirs` from their manifests and `.done` files, without
/// reading shard data; equal to the sources [`load_with`] reports.
pub fn sources(dirs: &[PathBuf]) -> io::Result<Vec<DataSource>> {
    let mut out = Vec::new();
    for dir in dirs {
        let manifest = read_manifest(dir)?;
        let canonical = serde_json::to_vec(&manifest).map_err(io::Error::other)?;
        let mut hashes = Vec::new();
        let mut shards = 0;
        for index in 0..manifest.shards() {
            let digest = shard_digest(dir, index)
                .map_err(|e| io::Error::other(format!("{}: {e}", dir.display())))?;
            if let Some(d) = digest {
                hashes.extend_from_slice(&d);
                shards += 1;
            }
        }
        out.push(DataSource {
            manifest,
            manifest_sha256: hex_hash(&canonical),
            shards,
            data_sha256: hex_hash(&hashes),
        });
    }
    Ok(out)
}

/// The cache key of `sources` loaded with the targets of `hyper`.
pub fn key(sources: &[DataSource], hyper: &Hyper, beliefs: bool) -> String {
    let input = KeyInput {
        cache_format: CACHE_FORMAT,
        train_format: TRAIN_FORMAT,
        encoder_version: ENCODER_VERSION,
        sources,
        soft_weight: hyper.soft_weight.to_bits(),
        soft_temperature: hyper.soft_temperature.to_bits(),
        value_mix: hyper.value_mix.to_bits(),
        beliefs,
    };
    let json = serde_json::to_vec(&input).expect("key serialises");
    sha256(&json).iter().map(|b| format!("{b:02x}")).collect()
}

/// [`load_with`] through the cache in `cache` (created if missing).
pub fn load_cached(
    dirs: &[PathBuf],
    hyper: &Hyper,
    beliefs: bool,
    cache: &Path,
) -> io::Result<(Data, CacheUse)> {
    let sources = sources(dirs)?;
    let key = key(&sources, hyper, beliefs);
    let path = cache.join(format!("{key}.gcdata"));
    match read(&path, &key) {
        Ok(data) if data.sources == sources => return Ok((data, CacheUse::Hit)),
        Ok(_) => eprintln!("cache {}: other sources, rebuilding", path.display()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("cache {}: {e}, rebuilding", path.display()),
    }
    let data = load_with(dirs, hyper, beliefs)?;
    if data.sources != sources {
        return Err(io::Error::other(
            "shard hashes differ from their .done files or changed while loading",
        ));
    }
    fs::create_dir_all(cache)?;
    write(&path, &key, &data)?;
    Ok((data, CacheUse::Written))
}

#[derive(Serialize, Deserialize)]
struct Header {
    key: String,
    sources: Vec<DataSource>,
    coverage: Vec<Coverage>,
    splits: Vec<SplitHeader>,
}

#[derive(Serialize, Deserialize)]
struct SplitHeader {
    rows: usize,
    entries: usize,
    /// Bit patterns of each feature's table.
    tables: Vec<Vec<u32>>,
    /// Wide features (exact columns follow the codes).
    wide: Vec<usize>,
    belief: bool,
}

/// Elements converted per write or read.
const CHUNK: usize = 1 << 16;

fn put<T: Copy, const N: usize>(
    w: &mut impl Write,
    values: &[T],
    bytes: impl Fn(T) -> [u8; N],
) -> io::Result<()> {
    let mut buf = Vec::with_capacity(CHUNK * N);
    for chunk in values.chunks(CHUNK) {
        buf.clear();
        for &v in chunk {
            buf.extend_from_slice(&bytes(v));
        }
        w.write_all(&buf)?;
    }
    Ok(())
}

fn get<T, const N: usize>(
    r: &mut impl Read,
    len: usize,
    value: impl Fn([u8; N]) -> T,
) -> io::Result<Vec<T>> {
    let mut out = Vec::with_capacity(len);
    let mut buf = vec![0u8; CHUNK * N];
    let mut left = len;
    while left > 0 {
        let n = left.min(CHUNK);
        r.read_exact(&mut buf[..n * N])?;
        out.extend(buf[..n * N].as_chunks::<N>().0.iter().map(|&c| value(c)));
        left -= n;
    }
    Ok(out)
}

fn player_code(p: Player) -> u8 {
    Player::ALL.iter().position(|&q| q == p).expect("in ALL") as u8
}

fn player(code: u8) -> io::Result<Player> {
    Player::ALL
        .get(usize::from(code))
        .copied()
        .ok_or_else(|| io::Error::other("bad player code"))
}

fn write_examples(w: &mut impl Write, e: &Examples) -> io::Result<()> {
    w.write_all(e.features.codes())?;
    for (_, column) in e.features.wide() {
        put(w, column, f32::to_le_bytes)?;
    }
    put(w, &e.offsets, |x| (x as u64).to_le_bytes())?;
    put(w, &e.legal, u32::to_le_bytes)?;
    put(w, &e.target, f32::to_le_bytes)?;
    put(w, &e.chosen, u32::to_le_bytes)?;
    put(w, &e.value, f32::to_le_bytes)?;
    put(w, &e.has_options, |b| [u8::from(b)])?;
    put(w, &e.option_score, f32::to_le_bytes)?;
    put(w, &e.option_win, f32::to_le_bytes)?;
    w.write_all(&e.belief)?;
    put(w, &e.player, |p| [player_code(p)])?;
    put(w, &e.roles, |r| {
        [
            player_code(r.declarer),
            player_code(r.defenders[0]),
            player_code(r.defenders[1]),
        ]
    })
}

fn read_examples(r: &mut impl Read, h: &SplitHeader) -> io::Result<Examples> {
    let bad = |why: &str| io::Error::other(format!("cache corrupt: {why}"));
    let (rows, entries) = (h.rows, h.entries);
    let mut codes = vec![0u8; rows * FEATURES];
    r.read_exact(&mut codes)?;
    let tables = h
        .tables
        .iter()
        .map(|t| t.iter().map(|&b| f32::from_bits(b)).collect())
        .collect();
    let wide = h
        .wide
        .iter()
        .map(|&f| Ok((f, get(r, rows, f32::from_le_bytes)?)))
        .collect::<io::Result<_>>()?;
    let features = FeatureCodes::from_parts(codes, tables, wide).map_err(|e| bad(&e))?;
    let offsets = get(r, rows + 1, |b| u64::from_le_bytes(b) as usize)?;
    if offsets.first() != Some(&0)
        || offsets.last() != Some(&entries)
        || offsets.windows(2).any(|w| w[0] > w[1])
    {
        return Err(bad("offsets"));
    }
    let legal = get(r, entries, u32::from_le_bytes)?;
    if legal.iter().any(|&a| a as usize >= ACTIONS) {
        return Err(bad("action index"));
    }
    let target = get(r, entries, f32::from_le_bytes)?;
    let chosen = get(r, rows, u32::from_le_bytes)?;
    if (0..rows).any(|i| chosen[i] as usize >= offsets[i + 1] - offsets[i]) {
        return Err(bad("chosen action"));
    }
    let value = get(r, rows, f32::from_le_bytes)?;
    let has_options = get(r, rows, |[b]: [u8; 1]| b != 0)?;
    let option_score = get(r, entries, f32::from_le_bytes)?;
    let option_win = get(r, entries, f32::from_le_bytes)?;
    let mut belief = vec![0u8; if h.belief { 32 * rows } else { 0 }];
    r.read_exact(&mut belief)?;
    let players: Vec<Player> = get(r, rows, |[b]: [u8; 1]| b)?
        .into_iter()
        .map(player)
        .collect::<io::Result<_>>()?;
    let roles = get(r, rows, |b: [u8; 3]| b)?
        .into_iter()
        .map(|[d, a, b]| {
            Ok(Roles {
                declarer: player(d)?,
                defenders: [player(a)?, player(b)?],
            })
        })
        .collect::<io::Result<_>>()?;
    Ok(Examples {
        features,
        offsets,
        legal,
        target,
        chosen,
        value,
        has_options,
        option_score,
        option_win,
        belief,
        player: players,
        roles,
    })
}

fn splits(data: &Data) -> [&Examples; 3] {
    [&data.train, &data.validation, &data.test]
}

/// Writes `data` to `path` (atomically: a partial file never has the
/// final name).
pub fn write(path: &Path, key: &str, data: &Data) -> io::Result<()> {
    let header = Header {
        key: key.to_string(),
        sources: data.sources.clone(),
        coverage: data.coverage.clone(),
        splits: splits(data)
            .iter()
            .map(|e| SplitHeader {
                rows: e.rows(),
                entries: e.legal.len(),
                tables: e
                    .features
                    .tables()
                    .iter()
                    .map(|t| t.iter().map(|v| v.to_bits()).collect())
                    .collect(),
                wide: e.features.wide().iter().map(|(f, _)| *f).collect(),
                belief: !e.belief.is_empty(),
            })
            .collect(),
    };
    let json = serde_json::to_vec(&header).map_err(io::Error::other)?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    let mut w = BufWriter::with_capacity(1 << 22, fs::File::create(&tmp)?);
    w.write_all(MAGIC)?;
    w.write_all(&(json.len() as u64).to_le_bytes())?;
    w.write_all(&json)?;
    for e in splits(data) {
        write_examples(&mut w, e)?;
    }
    w.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    fs::rename(&tmp, path)
}

/// Reads the cache file `path` written under `key`.
pub fn read(path: &Path, key: &str) -> io::Result<Data> {
    let bad = |why: &str| io::Error::other(format!("cache corrupt: {why}"));
    let mut r = BufReader::with_capacity(1 << 22, fs::File::open(path)?);
    let mut magic = [0u8; 8];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(bad("magic"));
    }
    let mut len = [0u8; 8];
    r.read_exact(&mut len)?;
    let len = usize::try_from(u64::from_le_bytes(len)).map_err(|_| bad("header length"))?;
    if len > 1 << 30 {
        return Err(bad("header length"));
    }
    let mut json = vec![0u8; len];
    r.read_exact(&mut json)?;
    let header: Header = serde_json::from_slice(&json).map_err(io::Error::other)?;
    if header.key != key || header.splits.len() != 3 {
        return Err(bad("key"));
    }
    let mut parts = Vec::with_capacity(3);
    for h in &header.splits {
        parts.push(read_examples(&mut r, h)?);
    }
    if r.read(&mut [0u8; 1])? != 0 {
        return Err(bad("trailing bytes"));
    }
    let mut parts = parts.into_iter();
    let mut next = || parts.next().expect("three splits");
    Ok(Data {
        train: next(),
        validation: next(),
        test: next(),
        coverage: header.coverage,
        sources: header.sources,
    })
}
