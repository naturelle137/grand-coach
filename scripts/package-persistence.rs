// SPDX-License-Identifier: EUPL-1.2

//! RC test helper, linked against the same skat-app build as the package.
//! Uses actual training events; never included inside the application archive.

use skat_app::{
    AssistanceLevel, Command, LearningHistory, Level, OpponentEngine, Opponents, Phase,
    SeriesLength, Session, SessionConfig, TrainingCommand, TrainingSession, TrainingTopic,
};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("create or check");
    if mode == "hybrid" || mode == "hybrid-missing" {
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
                if mode == "hybrid-missing" {
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
                if mode == "hybrid-missing" {
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
                let (action, reason) = job.run_explained(None);
                if mode == "neural" && reason.is_none() {
                    observed = true;
                }
                if mode != "neural" {
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
