// SPDX-License-Identifier: EUPL-1.2

//! Replay-comment latency (D-67), measured through the application
//! interface as the recording window uses it: for every recorded human
//! decision of the three frozen fixtures (`tests/fixtures/recordings`), the
//! time from `ReplayCoach::request` to an accepted `ReplayCoach::finish`,
//! with `Budget::INTERACTIVE` and `REPLAY_COACH_TIME_LIMIT`, once in the
//! historical and once in the hindsight perspective. One analysis at a time.
//!
//! ```sh
//! cargo run --locked --release -p skat-app --example replay_coach_latency
//! ```

use std::time::{Duration, Instant};

use skat_app::{
    Budget, Cancel, GameRecording, REPLAY_COACH_TIME_LIMIT, ReplayCoach, ReplayPerspective,
    ReplayStatus,
};

fn main() {
    let fixtures = [
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
    ];
    println!(
        "budget {:?}, limit {REPLAY_COACH_TIME_LIMIT:?}",
        Budget::INTERACTIVE
    );
    for perspective in [ReplayPerspective::Historical, ReplayPerspective::Hindsight] {
        let mut times: Vec<(Duration, &str, usize)> = Vec::new();
        let mut provisional = 0;
        let mut unavailable = 0;
        for (name, json) in fixtures {
            let recording: GameRecording = serde_json::from_str(json).expect("fixture");
            let mut coach = ReplayCoach::open(&recording, Budget::INTERACTIVE).expect("valid");
            for i in 0..coach.decisions().len() {
                coach.select(Some(i));
                coach.set_perspective(perspective);
                let prefix = coach.decisions()[i].prefix;
                let start = Instant::now();
                let job = coach.request().expect("selected");
                let cancel = Cancel::with_deadline(start + REPLAY_COACH_TIME_LIMIT);
                coach
                    .finish(job.run(Some(&cancel)))
                    .expect("current request");
                times.push((start.elapsed(), name, prefix));
                match coach.view().status {
                    ReplayStatus::Ready(c)
                        if !c.review.forced
                            && c.review.status != skat_app::coach_model::Status::Complete =>
                    {
                        provisional += 1
                    }
                    ReplayStatus::Ready(_) => {}
                    _ => unavailable += 1,
                }
            }
        }
        times.sort();
        let n = times.len();
        let (max, name, prefix) = times[n - 1];
        println!(
            "{perspective:?}: {n} decisions, median {:?}, p90 {:?}, max {max:?} ({name} prefix {prefix}), provisional {provisional}, unavailable {unavailable}",
            times[n / 2].0,
            times[n * 9 / 10].0,
        );
    }
}
