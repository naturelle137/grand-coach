// SPDX-License-Identifier: EUPL-1.2

//! Sharded generation (#70): a resumed run writes the same bytes as an
//! uninterrupted one, incomplete or corrupt shards are never read as valid,
//! and caps and the CPU budget hold.

use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nn_train::selfplay::{RunSeed, TeacherProfile, play_deal};
use nn_train::shards::{
    Limits, Manifest, ShardError, Stop, generate, read_manifest, read_shard, shard_path, to_jsonl,
};
use skat_ai::{Budget, Level, Profile};

const TEST_TEACHER: TeacherProfile = TeacherProfile {
    name: "test",
    profile: Profile {
        budget: Some(Budget {
            samples: 2,
            nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
        }),
        ..Level::Strong.profile()
    },
};

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

fn limits(workers: usize) -> Limits {
    Limits {
        wall_time: Duration::from_secs(3600),
        disk_bytes: 1 << 30,
        cpus: nz(4),
        workers: nz(workers),
    }
}

fn manifest() -> Manifest {
    Manifest::new("test-commit", TEST_TEACHER, 7, 4, 1)
}

/// A fresh scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nn-train-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Every file of `dir` with its contents, sorted by name.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().into_string().unwrap(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn resumed_generation_matches_an_uninterrupted_run() {
    let m = manifest();
    let full = scratch("full");
    let report = generate(&full, &m, TEST_TEACHER, limits(2)).unwrap();
    assert_eq!(
        (report.written, report.skipped, report.stop),
        (4, 0, Stop::Finished)
    );
    assert_eq!(read_manifest(&full).unwrap(), m);
    let expected = files(&full);
    // Shard contents are the records of their deals, in order.
    for index in 0..4 {
        let records = read_shard(&full, index).unwrap();
        assert_eq!(records, play_deal(RunSeed(7), TEST_TEACHER.profile, index));
        assert_eq!(
            to_jsonl(&records),
            fs::read(shard_path(&full, index)).unwrap()
        );
    }

    // Interrupted by the disk cap after the first shard, then resumed.
    let resumed = scratch("resumed");
    let manifest_bytes = fs::read(full.join("manifest.json")).unwrap().len() as u64;
    let first_shard = fs::read(shard_path(&full, 0)).unwrap().len() as u64;
    let capped = Limits {
        disk_bytes: manifest_bytes + first_shard,
        ..limits(1)
    };
    let report = generate(&resumed, &m, TEST_TEACHER, capped).unwrap();
    assert_eq!((report.written, report.stop), (1, Stop::Disk));
    assert!(matches!(
        read_shard(&resumed, 1),
        Err(ShardError::Incomplete)
    ));

    // Leftovers of a crash: a half-written temporary file, a data file
    // without its `.done`, and a corrupt complete-looking shard.
    let tmp = resumed.join("shard-00001.jsonl.partial");
    fs::write(&tmp, b"{\"half\":").unwrap();
    fs::write(shard_path(&resumed, 2), b"garbage\n").unwrap();
    assert!(matches!(
        read_shard(&resumed, 2),
        Err(ShardError::Incomplete)
    ));
    fs::copy(
        full.join("shard-00003.done"),
        resumed.join("shard-00003.done"),
    )
    .unwrap();
    let mut data = fs::read(shard_path(&full, 3)).unwrap();
    data.truncate(data.len() / 2);
    fs::write(shard_path(&resumed, 3), &data).unwrap();
    assert!(matches!(
        read_shard(&resumed, 3),
        Err(ShardError::Corrupt(_))
    ));

    let report = generate(&resumed, &m, TEST_TEACHER, limits(3)).unwrap();
    assert_eq!(
        (report.skipped, report.written, report.stop),
        (1, 3, Stop::Finished)
    );
    let _ = fs::remove_file(&tmp);
    assert_eq!(files(&resumed), expected);

    // A complete run skips everything.
    let report = generate(&resumed, &m, TEST_TEACHER, limits(1)).unwrap();
    assert_eq!((report.skipped, report.written), (4, 0));
}

#[test]
fn caps_and_manifests_are_enforced() {
    let m = manifest();
    let dir = scratch("caps");
    let none = Limits {
        wall_time: Duration::ZERO,
        ..limits(1)
    };
    let report = generate(&dir, &m, TEST_TEACHER, none).unwrap();
    assert_eq!((report.written, report.stop), (0, Stop::WallTime));
    assert!(matches!(read_shard(&dir, 0), Err(ShardError::Incomplete)));

    // Another run in the same directory, a mismatching profile or more
    // workers than CPUs are refused.
    let other = Manifest::new("test-commit", TEST_TEACHER, 8, 4, 1);
    assert!(generate(&dir, &other, TEST_TEACHER, limits(1)).is_err());
    assert!(generate(&dir, &m, TeacherProfile::STRONG, limits(1)).is_err());
    let oversubscribed = Limits {
        cpus: nz(2),
        workers: nz(3),
        ..limits(1)
    };
    assert!(generate(&dir, &m, TEST_TEACHER, oversubscribed).is_err());

    // Workers times analysis threads stay within the CPU budget.
    for cpus in 1..=8 {
        for workers in 1..=cpus {
            let l = Limits {
                cpus: nz(cpus),
                workers: nz(workers),
                ..limits(1)
            };
            assert!(l.workers.get() * l.threads().get() <= cpus);
        }
    }
}
