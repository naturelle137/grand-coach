// SPDX-License-Identifier: EUPL-1.2

//! Coach latency per human decision, measured through the application
//! interface as the frontend uses it (`docs/coach.md`): a session with the
//! coach on "every move" and `Budget::INTERACTIVE`; at each human decision
//! the time from `Session::coach_job` to an accepted `finish_coach`
//! (analysis and explanation). The human follows the coach (its best
//! option, otherwise the first legal action), so Hand, Drücken and the
//! announcement are reached; opponents are Medium, Kontra/Re is on.
//! Series seeds 1, 2, …; the first positions of each kind.
//!
//! ```sh
//! cargo run --locked --release -p skat-app --example coach_latency
//! ```
//!
//! Optional argument: positions per kind (default 10). `--fast-play GAMES
//! [NODES_PER_TASK]` compares FIFO and learned review ordering on identical
//! maximal-backlog traces (24 samples; default interactive node budget).

use std::time::{Duration, Instant};

use skat_app::coach_model::Decision;
use skat_app::{
    Action, Budget, Cancel, CoachLevel, CoachSettings, Command, Debriefing, Level, Opponents,
    Phase, SeriesLength, Session, SessionConfig,
};

const KINDS: [&str; 8] = [
    "Reizen",
    "Hand/pickup",
    "Drücken",
    "Announcement",
    "Kontra/Re",
    "Card play, trick 1",
    "Card play, trick 4",
    "Card play, trick 8",
];

fn kind(decision: Decision, tricks_played: u8) -> Option<usize> {
    Some(match decision {
        Decision::Bid | Decision::Answer => 0,
        Decision::Pickup => 1,
        Decision::Discard => 2,
        Decision::Announce => 3,
        Decision::Kontra | Decision::Re => 4,
        Decision::Card => match tricks_played {
            0 => 5,
            3 => 6,
            7 => 7,
            _ => return None,
        },
    })
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i]
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--fast-play") {
        fast_play();
        return;
    }
    let per_kind: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10);
    let mut times: Vec<Vec<Duration>> = vec![Vec::new(); KINDS.len()];
    let mut incomplete = [0usize; KINDS.len()];
    let mut seed = 0;
    while times.iter().any(|t| t.len() < per_kind) && seed < 1_000 {
        seed += 1;
        let mut session = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Fixed(3),
            kontra_re: true,
            opponents: Opponents::Levels([Level::Medium; 2]),
            ..SessionConfig::default()
        });
        session
            .execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                warnings: false,
                show_figures: true,
                focus: None,
            }))
            .unwrap();
        loop {
            session.run_opponents();
            let view = session.view();
            if view.series_finished {
                break;
            }
            if view.legal_actions.is_empty() {
                session.execute(Command::NextGame).unwrap();
                continue;
            }
            let job = session.coach_job().expect("every move: a coach job");
            let start = Instant::now();
            let result = job.run(None);
            let explanation = result.explanation().cloned();
            session.finish_coach(result).expect("same decision");
            let elapsed = start.elapsed();
            let view = session.view();
            let explanation = explanation.as_ref();
            let complete = explanation.is_some_and(|e| e.complete());
            let k = explanation.and_then(|e| kind(e.decision, view.tricks_played));
            if let Some(k) = k
                && times[k].len() < per_kind
            {
                times[k].push(elapsed);
                if !complete {
                    incomplete[k] += 1;
                }
            }
            let action = explanation
                .and_then(|e| e.best())
                .map(|o| o.action)
                .unwrap_or(view.legal_actions[0]);
            session.execute(Command::Act(action)).unwrap();
        }
    }
    println!("up to {per_kind} positions per kind; series seeds 1..={seed}; release build");
    println!();
    println!("| Decision | Positions | Median | p90 | Max | Incomplete |");
    println!("|---|---|---|---|---|---|");
    for (k, t) in times.iter_mut().enumerate() {
        if t.is_empty() {
            println!("| {} | 0 | – | – | – | – |", KINDS[k]);
            continue;
        }
        t.sort();
        println!(
            "| {} | {} | {:.3} s | {:.3} s | {:.3} s | {} |",
            KINDS[k],
            t.len(),
            percentile(t, 0.5).as_secs_f64(),
            percentile(t, 0.9).as_secs_f64(),
            t.last().unwrap().as_secs_f64(),
            incomplete[k],
        );
    }
}

/// A no-wait trace: every own decision remains queued while the game runs.
/// This is a deliberately maximal backlog, not a simulated human think time.
fn fast_play() {
    let games: u64 = std::env::args()
        .nth(2)
        .map(|s| s.parse().unwrap())
        .unwrap_or(3);
    let nodes: u64 = std::env::args()
        .nth(3)
        .map(|s| s.parse().unwrap())
        .unwrap_or(Budget::INTERACTIVE.nodes_per_task);
    let budget = Budget {
        samples: 24,
        nodes_per_task: nodes,
    };
    let loading = Instant::now();
    let model =
        skat_app::neural_model().expect("paired learned measurement requires a usable model");
    println!(
        "model SHA-256: {:02x?}; load {:.6} s (excluded below)",
        model.sha256(),
        loading.elapsed().as_secs_f64()
    );
    println!("maximal no-wait backlog; Easy opponents; Pass/first-legal human; budget={budget:?}");
    println!(
        "times include trace capture, review ordering, scripted cancellation, analysis and view building"
    );
    println!(
        "| Seed | Mode | Queue peak | Cancellations | First useful | Full debrief | Review execution | Moments | Missed |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for seed in 1..=games {
        let mut session = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Fixed(1),
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        });
        session.set_coach_budget(budget);
        session
            .execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                ..CoachSettings::default()
            }))
            .unwrap();
        let capture = Instant::now();
        let mut decisions = 0;
        loop {
            session.run_opponents();
            let view = session.view();
            if view.legal_actions.is_empty() {
                break;
            }
            let action =
                if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
                    Action::Pass
                } else {
                    view.legal_actions[0]
                };
            session.execute(Command::Act(action)).unwrap();
            decisions += 1;
        }
        let capture_time = capture.elapsed();
        let Some(Debriefing::Computing { remaining, .. }) = session.view().coach.debriefing else {
            panic!("no-wait trace must build a review queue");
        };
        assert_eq!(remaining, decisions);
        let mut results = Vec::new();
        // Alternate measurement order to avoid always warming the same mode.
        let modes = if seed % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for learned in modes {
            let mut run = session.clone();
            let start = Instant::now();
            run.set_debrief_model(learned.then(|| model.clone()));
            let cancel = Cancel::new();
            cancel.cancel();
            run.finish_coach(run.coach_job().unwrap().run(Some(&cancel)))
                .unwrap();
            let mut first = None;
            let mut reviews = 0;
            let mut analysis_time = std::time::Duration::ZERO;
            let mut identities = std::collections::HashSet::new();
            while let Some(job) = run.coach_job() {
                assert!(job.is_review());
                identities.insert(job.input_identity());
                let analysis_start = Instant::now();
                let result = job.run(None);
                analysis_time += analysis_start.elapsed();
                run.finish_coach(result).unwrap();
                reviews += 1;
                let moments = match run.view().coach.debriefing.unwrap() {
                    Debriefing::Computing { moments, .. } | Debriefing::Ready(moments) => moments,
                };
                if !moments.is_empty() && first.is_none() {
                    first = Some(capture_time + start.elapsed());
                }
            }
            assert_eq!(reviews, decisions, "no review may be dropped");
            println!(
                "seed {seed}: {} distinct review inputs of {reviews} (reusable: {})",
                identities.len(),
                reviews as usize - identities.len()
            );
            let full = capture_time + start.elapsed();
            let Some(Debriefing::Ready(moments)) = run.view().coach.debriefing else {
                panic!("finished");
            };
            let first =
                first.map_or_else(|| "none".into(), |t| format!("{:.6} s", t.as_secs_f64()));
            let mode = if learned {
                "learned order"
            } else {
                "Classic FIFO"
            };
            println!(
                "| {seed} | {mode} | {remaining} | 1 (scripted) | {first} | {:.6} s | {:.6} s | {} | 0 |",
                full.as_secs_f64(),
                analysis_time.as_secs_f64(),
                moments.len()
            );
            results.push(moments);
        }
        assert_eq!(
            results[0], results[1],
            "identical complete moments and evidence on the same trace"
        );
    }
}
