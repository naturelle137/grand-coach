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
    if std::env::args().nth(1).as_deref() == Some("--worker-play") {
        worker_play();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--review-play") {
        review_play();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--paced-play") {
        paced_play();
        return;
    }
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

/// Compare the same fixed actions with immediate and deferred reviews.
fn paced_play() {
    let games: u64 = std::env::args()
        .nth(2)
        .map(|s| s.parse().unwrap())
        .unwrap_or(6);
    let nodes: u64 = std::env::args()
        .nth(3)
        .map(|s| s.parse().unwrap())
        .unwrap_or(20_000);
    let budget = Budget {
        samples: 24,
        nodes_per_task: nodes,
    };
    for seed in 1..=games {
        let mut outcomes = Vec::new();
        for paced in [false, true] {
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
            let start = Instant::now();
            let mut identities = std::collections::HashSet::new();
            let mut decisions = 0;
            loop {
                session.run_opponents();
                let view = session.view();
                if view.legal_actions.is_empty() {
                    break;
                }
                let job = session.coach_job().expect("current decision");
                assert!(!job.is_review());
                identities.insert(job.input_identity());
                let action =
                    if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
                        Action::Pass
                    } else {
                        view.legal_actions[0]
                    };
                session.execute(Command::Act(action)).unwrap();
                if paced {
                    session.finish_coach(job.run(None)).unwrap();
                }
                decisions += 1;
            }
            let mut pending = 0;
            while let Some(job) = session.coach_job() {
                assert!(job.is_review());
                session.finish_coach(job.run(None)).unwrap();
                pending += 1;
            }
            assert_eq!(pending, if paced { 0 } else { decisions });
            let Some(Debriefing::Ready(moments)) = session.view().coach.debriefing else {
                panic!("finished")
            };
            println!(
                "seed {seed} {}: {decisions} decisions, {} unique inputs, {pending} deferred, {:.6} s, {} moments",
                if paced { "paced" } else { "fast" },
                identities.len(),
                start.elapsed().as_secs_f64(),
                moments.len()
            );
            outcomes.push((identities, moments));
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "paced and fast inputs and final evidence"
        );
    }
}

/// The GUI's preemption order: the old worker was told to cancel and is not
/// joined; `start` runs while it may still be executing. Its receiver is kept
/// to settle the worker's time later.
fn start_after_cancel<T>(
    abandoned: Option<std::sync::mpsc::Receiver<T>>,
    settle_later: &mut Vec<std::sync::mpsc::Receiver<T>>,
    start: impl FnOnce(),
) {
    start();
    settle_later.extend(abandoned);
}

/// One background worker with the GUI's job policy (`drive_coach`): the current
/// decision first, otherwise the oldest queued review. `cancel` mode drops a
/// running analysis of an earlier decision when the current one needs analysis
/// (baseline, as in egui); `finish` mode lets it complete first (candidate, no
/// restart work). Usage: `--review-play cancel|finish DELAY_MS GAMES NODES`.
fn review_play() {
    type Run = (
        skat_app::CoachJob,
        Cancel,
        std::sync::mpsc::Receiver<(skat_app::CoachResult, Duration)>,
    );
    fn spawn_run(job: skat_app::CoachJob) -> Run {
        let cancel = Cancel::new();
        let stop = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let run = job.clone();
        std::thread::spawn(move || {
            let began = Instant::now();
            let result = run.run(Some(&stop));
            let _ = tx.send((result, began.elapsed()));
        });
        (job, cancel, rx)
    }
    let args: Vec<String> = std::env::args().collect();
    let preempt = match args[2].as_str() {
        "cancel" => true,
        "finish" => false,
        _ => panic!("mode cancel|finish"),
    };
    let delay = args[3].parse::<u64>().unwrap();
    let games = args[4].parse::<u64>().unwrap();
    let nodes = args[5].parse::<u64>().unwrap();
    let has_moments = |session: &Session| {
        matches!(
            session.view().coach.debriefing,
            Some(Debriefing::Computing { ref moments, .. } | Debriefing::Ready(ref moments))
                if !moments.is_empty()
        )
    };
    for seed in 1..=games {
        let mut session = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Fixed(1),
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        });
        session.set_coach_budget(Budget {
            samples: 24,
            nodes_per_task: nodes,
        });
        session
            .execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                ..CoachSettings::default()
            }))
            .unwrap();
        let start = Instant::now();
        let mut running: Option<Run> = None;
        let mut cancelled_runs = Vec::new();
        let (mut cancelled, mut wasted, mut executed) = (0, Duration::ZERO, Duration::ZERO);
        let (mut waits, mut max_wait) = (Duration::ZERO, Duration::ZERO);
        let mut first = None;
        loop {
            session.run_opponents();
            let view = session.view();
            if view.legal_actions.is_empty() {
                break;
            }
            // The current decision needs analysis: an unfinished analysis of an
            // earlier decision is cancelled (baseline) or finishes first.
            let shown = Instant::now();
            let mut abandoned = None;
            if let Some((job, cancel, rx)) = running.take() {
                let stale = session
                    .coach_job()
                    .is_some_and(|n| !n.is_review() && !n.same_decision(&job));
                if stale && preempt {
                    // As in egui: request cancellation and start the current
                    // analysis at once; the old worker may still be running.
                    cancel.cancel();
                    abandoned = Some(rx);
                } else {
                    let (result, spent) = rx.recv().unwrap();
                    executed += spent;
                    let _ = session.finish_coach(result);
                }
            }
            let current = session.coach_job().unwrap();
            assert!(!current.is_review());
            start_after_cancel(abandoned, &mut cancelled_runs, || {
                running = Some(spawn_run(current));
            });
            let wait = shown.elapsed();
            waits += wait;
            max_wait = max_wait.max(wait);
            // Human think time: the worker runs, then queued reviews follow.
            let until = Instant::now() + Duration::from_millis(delay);
            while Instant::now() < until {
                if let Some((_, _, rx)) = running.as_ref()
                    && let Ok((result, spent)) = rx.try_recv()
                {
                    executed += spent;
                    running = None;
                    let _ = session.finish_coach(result);
                }
                if running.is_none()
                    && let Some(job) = session.coach_job()
                {
                    running = Some(spawn_run(job));
                }
                std::thread::sleep(Duration::from_micros(200));
            }
            let action =
                if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
                    Action::Pass
                } else {
                    view.legal_actions[0]
                };
            session.execute(Command::Act(action)).unwrap();
        }
        // Game over: settle the worker, then drain the queue.
        if has_moments(&session) {
            first = Some(start.elapsed());
        }
        if let Some((_, _, rx)) = running.take() {
            let (result, spent) = rx.recv().unwrap();
            executed += spent;
            let _ = session.finish_coach(result);
        }
        if first.is_none() && has_moments(&session) {
            first = Some(start.elapsed());
        }
        // Cancelled workers exit on their own; their time is wasted work.
        for rx in cancelled_runs {
            let (_, spent): (skat_app::CoachResult, Duration) = rx.recv().unwrap();
            cancelled += 1;
            wasted += spent;
            executed += spent;
        }
        let mut drained = 0;
        while let Some(job) = session.coach_job() {
            let began = Instant::now();
            session.finish_coach(job.run(None)).unwrap();
            executed += began.elapsed();
            drained += 1;
            if first.is_none() && has_moments(&session) {
                first = Some(start.elapsed());
            }
        }
        let Some(Debriefing::Ready(moments)) = session.view().coach.debriefing else {
            panic!("unfinished")
        };
        if std::env::var_os("COACH_TRACE_EVIDENCE").is_some() {
            println!("evidence={}", serde_json::to_string(&moments).unwrap());
        }
        println!(
            "seed={seed} mode={} delay_ms={delay} cancelled={cancelled} wasted={wasted:?} executed={executed:?} drained={drained} foreground_wait_total={waits:?} foreground_wait_max={max_wait:?} first={first:?} full={:?} moments={}",
            args[2],
            start.elapsed(),
            moments.len()
        );
    }
}

/// Exercise a single background coach worker while fixed actions and opponents run.
fn worker_play() {
    let delay = std::env::args().nth(2).unwrap().parse::<u64>().unwrap();
    let games = std::env::args().nth(3).unwrap().parse::<u64>().unwrap();
    let nodes = std::env::args().nth(4).unwrap().parse::<u64>().unwrap();
    for seed in 1..=games {
        let mut session = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Fixed(1),
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        });
        session.set_coach_budget(Budget {
            samples: 24,
            nodes_per_task: nodes,
        });
        session
            .execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                ..CoachSettings::default()
            }))
            .unwrap();
        let start = Instant::now();
        let mut restarts = 0;
        let mut overlap = 0;
        let mut opponent_time = Duration::ZERO;
        let mut first = None;
        loop {
            let opponent_start = Instant::now();
            session.run_opponents();
            opponent_time += opponent_start.elapsed();
            let view = session.view();
            if view.legal_actions.is_empty() {
                break;
            }
            let job = session.coach_job().unwrap();
            assert!(!job.is_review());
            let cancel = Cancel::new();
            let stop = cancel.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let began = Instant::now();
                let result = job.run(Some(&stop));
                let _ = sender.send((result, began, Instant::now()));
            });
            std::thread::sleep(Duration::from_millis(delay));
            let action =
                if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
                    Action::Pass
                } else {
                    view.legal_actions[0]
                };
            if let Ok((result, _, _)) = receiver.try_recv() {
                session.finish_coach(result).unwrap();
                session.execute(Command::Act(action)).unwrap();
            } else {
                session.execute(Command::Act(action)).unwrap();
                let opponent_start = Instant::now();
                session.run_opponents();
                let opponent_end = Instant::now();
                opponent_time += opponent_end - opponent_start;
                let result = match receiver.try_recv() {
                    Ok(result) => result,
                    Err(_) => {
                        cancel.cancel();
                        restarts += 1;
                        receiver.recv().unwrap()
                    }
                };
                overlap += usize::from(result.1 < opponent_end && result.2 > opponent_start);
                session.finish_coach(result.0).unwrap();
            }
            worker.join().unwrap();
        }
        let (pending, existing) = match session.view().coach.debriefing.unwrap() {
            Debriefing::Computing { remaining, moments } => (remaining, moments),
            Debriefing::Ready(moments) => (0, moments),
        };
        if !existing.is_empty() {
            first = Some(start.elapsed());
        }
        let mut drained = 0;
        while let Some(job) = session.coach_job() {
            assert!(job.is_review());
            session.finish_coach(job.run(None)).unwrap();
            drained += 1;
            let moments = match session.view().coach.debriefing.unwrap() {
                Debriefing::Computing { moments, .. } | Debriefing::Ready(moments) => moments,
            };
            if !moments.is_empty() && first.is_none() {
                first = Some(start.elapsed());
            }
        }
        let Some(Debriefing::Ready(moments)) = session.view().coach.debriefing else {
            panic!("unfinished")
        };
        assert_eq!(drained, pending);
        if std::env::var_os("COACH_TRACE_EVIDENCE").is_some() {
            println!("evidence={}", serde_json::to_string(&moments).unwrap());
        }
        println!(
            "seed={seed} delay_ms={delay} overlap={overlap} cancel_requests={restarts} drained={drained} first_after_finish={first:?} total={:?} foreground_opponents={opponent_time:?} moments={}",
            start.elapsed(),
            moments.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, mpsc};

    /// A controlled worker that blocks after it was told to cancel: the new
    /// analysis must start before the old worker exits (as in egui).
    #[test]
    fn current_analysis_starts_while_cancelled_worker_still_runs() {
        let cancel = Cancel::new();
        let stop = cancel.clone();
        let (release, released) = mpsc::channel::<()>();
        let (tx, rx) = mpsc::channel();
        let exited = Arc::new(AtomicBool::new(false));
        let flag = exited.clone();
        let old = std::thread::spawn(move || {
            while !stop.is_cancelled() {
                std::thread::yield_now();
            }
            released.recv().unwrap(); // blocks after cancellation
            flag.store(true, Ordering::SeqCst);
            let _ = tx.send(());
        });
        cancel.cancel();
        let mut later = Vec::new();
        let mut started_while_running = false;
        start_after_cancel(Some(rx), &mut later, || {
            started_while_running = !exited.load(Ordering::SeqCst);
        });
        assert!(started_while_running, "new analysis waited for the old one");
        assert_eq!(later.len(), 1);
        release.send(()).unwrap();
        later[0].recv().unwrap();
        old.join().unwrap();
        assert!(exited.load(Ordering::SeqCst));
    }
}
