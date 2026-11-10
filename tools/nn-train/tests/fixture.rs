// SPDX-License-Identifier: EUPL-1.2

//! The checked-in fixture dataset (`fixtures/selfplay`, #70): two Strong
//! self-play deals generated with
//! `nn-train generate tools/nn-train/fixtures/selfplay --deals 2 --shard-deals 1 --seed 0`
//! before Strong said Kontra by the rules of thumb (#108): its teacher is
//! Strong with analysed Kontra ([`FIXTURE_TEACHER`]).

use std::fs;
use std::path::{Path, PathBuf};

use nn_train::selfplay::{RECORD_FORMAT, RunSeed, Split, Table, TeacherProfile, play_deal};
use nn_train::shards::{DATASET_FORMAT, read_manifest, read_shard, verify};
use skat_ai::Profile;

/// The fixture's teacher: Strong as it was then, with analysed Kontra.
const FIXTURE_TEACHER: Profile = Profile {
    search_kontra: true,
    ..TeacherProfile::STRONG.profile
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/selfplay")
}

#[test]
fn fixture_is_complete_small_and_strong() {
    let dir = fixture();
    let m = read_manifest(&dir).unwrap();
    assert_eq!(m.dataset_format, DATASET_FORMAT);
    // A format-1 dataset: it also covers reading records without the
    // optional fields of later formats.
    assert_eq!(m.record_format, 1);
    assert!(m.record_format <= RECORD_FORMAT);
    assert_eq!(m.table().unwrap(), Table::Teacher);
    assert_eq!(m.profile, TeacherProfile::STRONG.name);
    // Written before `Profile` had `search_kontra`; otherwise the same.
    assert_eq!(
        m.profile_detail,
        format!("{FIXTURE_TEACHER:?}").replace("search_kontra: true, ", "")
    );
    assert_eq!((m.seed, m.deals, m.shard_deals), (0, 2, 1));
    let v = verify(&dir).unwrap();
    assert_eq!((v.complete, v.incomplete), (2, 0));
    // A tiny fixture, not a dataset.
    let bytes: u64 = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .sum();
    assert!(bytes < 256 * 1024, "{bytes} bytes");
    for deal in 0..2 {
        let records = read_shard(&dir, deal).unwrap();
        let split = Split::of(RunSeed(0).deal_seed(deal));
        assert!(records.iter().all(|r| r.deal == deal && r.split == split));
        let seats: std::collections::BTreeSet<_> = records.iter().map(|r| r.view.seat).collect();
        assert_eq!(seats.len(), 3);
        assert!(records.iter().any(|r| r.teacher.status.is_some()));
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn fixture_regenerates_identically() {
    let dir = fixture();
    for deal in 0..2 {
        let expected = read_shard(&dir, deal).unwrap();
        let records = play_deal(RunSeed(0), FIXTURE_TEACHER, Table::Teacher, deal);
        assert_eq!(records, expected, "deal {deal}");
    }
}
