// SPDX-License-Identifier: EUPL-1.2

//! Recording-review latency (D-69), measured through the application
//! interface as the recording window runs it: for each frozen fixture
//! (`tests/fixtures/recordings`, plus Herz as a late-series defender game),
//! the time from `ReplayCoach::request_review` until the review is done,
//! one decision at a time with `Budget::INTERACTIVE`,
//! `REPLAY_REVIEW_DECISION_LIMIT` and `REPLAY_REVIEW_TIME_LIMIT`. Also the
//! time from cancelling a running decision to its thread returning.
//!
//! ```sh
//! cargo run --locked --release -p skat-app --example replay_review_latency
//! ```

use std::time::{Duration, Instant};

use skat_app::{
    Budget, Cancel, GameRecording, PlayerId, REPLAY_REVIEW_DECISION_LIMIT,
    REPLAY_REVIEW_TIME_LIMIT, ReplayCoach, ReviewStatus,
};

fn main() {
    let mut fixtures: Vec<(&str, GameRecording)> = [
        (
            "herz",
            include_str!("../tests/fixtures/recordings/herz.json"),
        ),
        (
            "grand",
            include_str!("../tests/fixtures/recordings/grand.json"),
        ),
        (
            "null",
            include_str!("../tests/fixtures/recordings/null.json"),
        ),
    ]
    .into_iter()
    .map(|(name, json)| (name, serde_json::from_str(json).expect("fixture")))
    .collect();
    let mut late = fixtures[0].1.clone();
    late.context.number = 2;
    late.context.dealer = PlayerId(0);
    fixtures.push(("herz-late", late));
    println!(
        "budget {:?}, per decision {REPLAY_REVIEW_DECISION_LIMIT:?}, total {REPLAY_REVIEW_TIME_LIMIT:?}, threads {}",
        Budget::INTERACTIVE,
        std::thread::available_parallelism().map_or(0, |n| n.get())
    );
    for (name, recording) in &fixtures {
        let mut coach = ReplayCoach::open(recording, Budget::INTERACTIVE).expect("valid");
        let start = Instant::now();
        let mut slowest = Duration::ZERO;
        assert!(coach.request_review());
        while let Some(job) = coach.review_job() {
            let begun = Instant::now();
            let deadline =
                (begun + REPLAY_REVIEW_DECISION_LIMIT).min(start + REPLAY_REVIEW_TIME_LIMIT);
            let result = job.run(Some(&Cancel::with_deadline(deadline)));
            slowest = slowest.max(begun.elapsed());
            coach.finish_review(result).expect("current");
            if start.elapsed() >= REPLAY_REVIEW_TIME_LIMIT {
                coach.review_time_limit();
            }
        }
        let total = start.elapsed();
        let ReviewStatus::Done(s) = coach.review_status() else {
            panic!("review not done");
        };
        println!(
            "{name}: {total:?} total, slowest decision {slowest:?}; end {:?}, decisions {}, forced {}, eligible {}, analysed {}, provisional {}, unavailable {}, moments {} at decisions {:?} ({:?})",
            s.end,
            s.decisions,
            s.forced,
            s.eligible,
            s.analysed,
            s.provisional,
            s.unavailable,
            s.moments.len(),
            s.moments.iter().map(|m| m.decision).collect::<Vec<_>>(),
            s.moments
                .iter()
                .map(|m| (m.moment.kind, m.moment.concept))
                .collect::<Vec<_>>(),
        );
    }
    // Cancellation: the slowest kind of decision, cancelled after 50 ms.
    let mut worst = Duration::ZERO;
    for (_, recording) in &fixtures {
        let mut coach = ReplayCoach::open(recording, Budget::INTERACTIVE).expect("valid");
        assert!(coach.request_review());
        while let Some(job) = coach.review_job() {
            let cancel = Cancel::new();
            let stop = cancel.clone();
            let worker = std::thread::spawn(move || job.run(Some(&stop)));
            std::thread::sleep(Duration::from_millis(50));
            let asked = Instant::now();
            cancel.cancel();
            let result = worker.join().expect("worker");
            worst = worst.max(asked.elapsed());
            coach.finish_review(result).expect("current");
        }
    }
    println!("cancel -> thread returned: worst {worst:?}");
}
