// SPDX-License-Identifier: EUPL-1.2

//! Persistence of classified review moments (#13, history format 7):
//! deterministic dedup per recording, partial reviews never counted as
//! exhaustive, lossless migration of older formats, reset and bounds.

use skat_app::coach_model::{Concept, MomentKind};
use skat_app::*;

const SMALL: Budget = Budget {
    samples: 2,
    nodes_per_task: 1_000_000,
};

fn recording() -> GameRecording {
    serde_json::from_str(include_str!("fixtures/recordings/herz.json")).unwrap()
}

fn obs(concept: Concept, kind: MomentKind) -> StrategicObservation {
    StrategicObservation { concept, kind }
}

fn review(recording: u64, exhaustive: bool, n: usize) -> StrategicReview {
    StrategicReview {
        recording,
        exhaustive,
        moments: (0..n)
            .map(|_| obs(Concept::ProvenVoid, MomentKind::Bad))
            .collect(),
    }
}

fn reload(h: &LearningHistory) -> LearningHistory {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("gc-strategic-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("learning.json");
    h.save_local(&path).unwrap();
    let r = LearningHistory::open_local(&path).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    r
}

#[test]
fn recording_a_review_twice_changes_nothing() {
    let mut h = LearningHistory::new();
    let r = review(7, true, 2);
    assert_eq!(h.record_strategic(&r), Ok(true));
    assert_eq!(h.record_strategic(&r), Ok(false));
    assert_eq!(h.record_strategic(&review(7, true, 1)), Ok(false));
    assert_eq!(h.strategic_overview().summary.total.total(), 2);
    let h = reload(&h);
    assert_eq!(h.strategic_reviews(), [r]);
}

#[test]
fn partial_review_is_flagged_and_replaced_only_by_an_exhaustive_one() {
    let mut h = LearningHistory::new();
    assert_eq!(h.record_strategic(&review(1, false, 1)), Ok(true));
    let o = h.strategic_overview();
    assert_eq!((o.reviews, o.partial), (1, 1));
    // A different partial run of the same recording does not replace it.
    assert_eq!(h.record_strategic(&review(1, false, 3)), Ok(false));
    assert_eq!(h.record_strategic(&review(1, true, 3)), Ok(true));
    let o = h.strategic_overview();
    assert_eq!((o.reviews, o.partial), (1, 0));
    assert_eq!(o.summary.total.total(), 3);
    // An exhaustive review is final.
    assert_eq!(h.record_strategic(&review(1, false, 1)), Ok(false));
    assert_eq!(h.strategic_overview().summary.total.total(), 3);
}

#[test]
fn more_than_three_moments_are_refused() {
    let mut h = LearningHistory::new();
    assert_eq!(
        h.record_strategic(&review(1, true, 4)),
        Err(LearningError::InvalidProvenance)
    );
    assert!(h.is_empty());
}

#[test]
fn format_6_migrates_and_older_formats_cannot_hold_reviews() {
    let v6 = r#"{"format":6,"next_attempt":1,"attempts":[],"events":[]}"#;
    let mut h = LearningHistory::read_local(v6.as_bytes()).unwrap();
    assert!(h.strategic_reviews().is_empty());
    h.record_strategic(&review(5, true, 1)).unwrap();
    let saved = reload(&h);
    assert_eq!(saved.strategic_reviews().len(), 1);
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(json["format"], LEARNING_FORMAT);
    for format in 1..=6 {
        let mut v = json.clone();
        v["format"] = format.into();
        assert!(
            LearningHistory::read_local(serde_json::to_vec(&v).unwrap().as_slice()).is_err(),
            "format {format} with reviews"
        );
    }
    // A duplicate recording in one file is damaged.
    let mut v = json.clone();
    let first = v["strategic"][0].clone();
    v["strategic"].as_array_mut().unwrap().push(first);
    assert!(LearningHistory::read_local(serde_json::to_vec(&v).unwrap().as_slice()).is_err());
}

#[test]
fn reset_forgets_reviews() {
    let mut h = LearningHistory::new();
    h.record_strategic(&review(1, true, 1)).unwrap();
    assert!(!h.is_empty());
    h.reset();
    assert!(h.is_empty());
    assert_eq!(h.strategic_overview(), StrategicOverview::default());
}

#[test]
fn ended_review_is_handed_out_once_with_a_stable_fingerprint() {
    let run = |r: &GameRecording| {
        let mut coach = ReplayCoach::open(r, SMALL).unwrap();
        assert!(coach.take_review_record().is_none());
        assert!(coach.request_review());
        while let Some(job) = coach.review_job() {
            assert!(coach.take_review_record().is_none(), "still running");
            coach.finish_review(job.run(None)).unwrap();
        }
        let record = coach.take_review_record().expect("ended review");
        assert!(coach.take_review_record().is_none());
        let ReviewStatus::Done(s) = coach.review_status() else {
            panic!("not done");
        };
        assert_eq!(record.moments.len(), s.moments.len());
        assert_eq!(record.exhaustive, s.exhaustive());
        record
    };
    let r = recording();
    let (a, b) = (run(&r), run(&r));
    assert_eq!(a, b);
    let mut other = r.clone();
    other.context.number = 2;
    other.context.dealer = PlayerId(0);
    assert_ne!(run(&other).recording, a.recording);
    let mut h = LearningHistory::new();
    h.record_strategic(&a).unwrap();
    assert_eq!(h.record_strategic(&b), Ok(false));
}
