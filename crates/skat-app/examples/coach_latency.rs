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
//! Optional argument: positions per kind (default 10).

use std::time::{Duration, Instant};

use skat_app::coach_model::Decision;
use skat_app::{
    CoachLevel, CoachSettings, Command, Level, Opponents, SeriesLength, Session, SessionConfig,
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
