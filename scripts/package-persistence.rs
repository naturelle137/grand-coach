// SPDX-License-Identifier: EUPL-1.2

//! RC test helper, linked against the same skat-app build as the package.
//! Uses actual training events; never included inside the application archive.

use skat_app::{
    Action, AssistanceLevel, Budget, CoachLevel, CoachSettings, Command, Debriefing,
    LearningHistory, Level, OpponentEngine, Opponents, Phase, SeriesLength, Session, SessionConfig,
    TrainingCommand, TrainingSession, TrainingTopic,
};
use std::io::BufRead;
use std::path::PathBuf;
use std::time::Instant;

/// One JSON line on stdout, then block until the driver answers on stdin, so
/// it can sample this process' RSS at a quiescent point.
fn sync(phase: &str) {
    println!("{{\"sync\":\"{phase}\"}}");
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).unwrap();
}

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// Timing and RSS-sync scenario for `scripts/package-measure.py`; one fresh
/// process per run. Panics on any model fallback so a run is never silently
/// measured on Classic.
fn measure() {
    let config = SessionConfig {
        seed: 7,
        series: SeriesLength::Fixed(1),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        ..SessionConfig::default()
    };
    let mut classic = Session::new(config);
    classic.run_opponents();
    sync("baseline");
    let cold = Instant::now();
    let mut session = Session::new(SessionConfig {
        engine: OpponentEngine::Neural,
        ..config
    });
    let provenance = session.neural_provenance().unwrap();
    provenance.model.as_ref().expect("model must load");
    let mut cold_ms = None;
    let mut warm = Vec::new();
    for _ in 0..512 {
        if let Some(job) = session.opponent_job() {
            let card_play = session.view().phase == Phase::Spiel;
            let start = Instant::now();
            let (action, reason) = job.run_explained(None);
            let elapsed = ms(start);
            if card_play {
                assert!(
                    reason.is_none(),
                    "model fallback invalidates the run: {reason:?}"
                );
                if cold_ms.is_none() {
                    cold_ms = Some(ms(cold));
                } else {
                    warm.push(elapsed);
                }
            }
            session.finish_opponent(action).unwrap();
        } else if let Some(&action) = session.view().legal_actions.first() {
            session.execute(Command::Act(action)).unwrap();
        } else {
            break;
        }
    }
    let cold_ms = cold_ms.expect("no card-play decision");
    println!("{{\"cold_start_ms\":{cold_ms},\"warm_decision_ms\":{warm:?}}}");
    // Debriefing: the human plays Pass/first legal actions with the coach on.
    let mut run = Session::new(config);
    run.set_coach_budget(Budget::INTERACTIVE);
    run.execute(Command::SetCoach(CoachSettings {
        level: CoachLevel::EveryMove,
        ..CoachSettings::default()
    }))
    .unwrap();
    run.set_debrief_model(Some(skat_app::neural_model().expect("model must load")));
    loop {
        run.run_opponents();
        let view = run.view();
        let Some(&first) = view.legal_actions.first() else {
            break;
        };
        let action = if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
            Action::Pass
        } else {
            first
        };
        run.execute(Command::Act(action)).unwrap();
    }
    let start = Instant::now();
    let mut reviews = 0;
    while let Some(job) = run.coach_job() {
        let result = job.run(None);
        run.finish_coach(result).unwrap();
        reviews += 1;
    }
    let Some(Debriefing::Ready(moments)) = run.view().coach.debriefing else {
        panic!("debriefing not ready");
    };
    println!(
        "{{\"debriefing_ms\":{},\"reviews\":{reviews},\"moments\":{}}}",
        ms(start),
        moments.len()
    );
    sync("after");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("create or check");
    if mode == "measure" {
        return measure();
    }
    if mode == "hybrid" || mode.starts_with("hybrid-") {
        let config = SessionConfig {
            seed: 7,
            series: SeriesLength::Fixed(2),
            opponents: Opponents::Levels([Level::Easy, Level::Easy]),
            engine: OpponentEngine::Hybrid,
            ..SessionConfig::default()
        };
        let mut session = Session::new(config);
        let mut classic = Session::new(SessionConfig {
            engine: OpponentEngine::Classic,
            ..config
        });
        assert_eq!(
            session.neural_provenance().unwrap().model.is_ok(),
            mode == "hybrid"
        );
        for _ in 0..512 {
            if let Some(job) = session.opponent_job() {
                let card_play = session.view().phase == Phase::Spiel;
                let action = job.run(None);
                session.finish_opponent(action).unwrap();
                if mode != "hybrid" {
                    let other = classic.opponent_job().unwrap().run(None);
                    classic.finish_opponent(other).unwrap();
                    let mut view = session.view();
                    view.config.engine = OpponentEngine::Classic;
                    assert_eq!(view, classic.view());
                }
                if card_play {
                    println!("{mode}: card-play decision verified");
                    return;
                }
            } else {
                let command = session
                    .view()
                    .legal_actions
                    .first()
                    .copied()
                    .map_or(Command::NextGame, Command::Act);
                session.execute(command.clone()).unwrap();
                if mode != "hybrid" {
                    classic.execute(command).unwrap();
                }
            }
        }
        panic!("no Hybrid card-play decision");
    }
    if ["neural", "missing", "fallback", "format", "encoder"].contains(&mode.as_str()) {
        let mut session = Session::new(SessionConfig {
            seed: 7,
            series: SeriesLength::Fixed(2),
            kontra_re: false,
            engine: OpponentEngine::Neural,
            ..SessionConfig::default()
        });
        let provenance = session.neural_provenance().expect("Neural session");
        assert_eq!(provenance.model.is_ok(), mode == "neural");
        let mut observed = false;
        for _ in 0..512 {
            if session.view().series_finished {
                break;
            }
            if let Some(job) = session.opponent_job() {
                let card_play = session.view().phase == Phase::Spiel;
                let (action, reason) = job.run_explained(None);
                if mode == "neural" && card_play && reason.is_none() {
                    observed = true;
                }
                if mode != "neural" && card_play {
                    let expected = match mode.as_str() {
                        "fallback" => "Model(HashMismatch)",
                        "missing" => "Model(Missing)",
                        "format" => "Model(Format { found: 65535 })",
                        "encoder" => "Model(Encoder { found: 65535 })",
                        _ => unreachable!(),
                    };
                    assert_eq!(format!("{reason:?}"), format!("Some({expected})"));
                    observed = true;
                }
                session.finish_opponent(action).unwrap();
            } else {
                let command = session
                    .view()
                    .legal_actions
                    .first()
                    .copied()
                    .map_or(Command::NextGame, Command::Act);
                session.execute(command).unwrap();
            }
            if observed {
                break;
            }
        }
        assert!(observed, "expected Neural model decision or model fallback");
        println!("{mode}: decision verified");
        return;
    }
    let path = PathBuf::from(args.next().expect("history path"));
    if mode == "create" {
        assert!(!path.exists(), "probe only creates a new scratch history");
        let mut session = TrainingSession::new(
            TrainingTopic::TrumpfOutside,
            AssistanceLevel::AnswerBeforeReveal,
        );
        let mut events = session.take_events();
        session.execute(TrainingCommand::Skip).unwrap();
        events.extend(session.take_events());
        let mut history = LearningHistory::new();
        let attempt = history.begin_attempt(session.attempt_kind()).unwrap();
        history.record(attempt, &events).unwrap();
        assert!(history.summary().totals.opportunities > 0);
        history.save_local(&path).unwrap();
        assert_eq!(LearningHistory::open_local(&path).unwrap(), history);
    } else {
        assert_eq!(mode, "check");
        let history = LearningHistory::open_local(&path).unwrap();
        assert!(history.summary().totals.opportunities > 0);
        assert!(history.summary().totals.skips > 0);
    }
}
