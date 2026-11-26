// SPDX-License-Identifier: EUPL-1.2

//! Independent engine lifecycle and recording compatibility, without arenas.

use super::*;
use crate::neural::FrozenNeural;
use skat_nn::{Fallback, ModelError, NeuralModel};

fn config() -> SessionConfig {
    SessionConfig {
        seed: 7,
        series: SeriesLength::Fixed(2),
        opponents: Opponents::Levels([Level::Easy, Level::Medium]),
        engine: OpponentEngine::Neural,
        ..SessionConfig::default()
    }
}

fn recording(s: &Session) -> GameRecording {
    s.export_current_recording_privileged().unwrap()
}

#[test]
fn classic_wire_and_default_are_unchanged_and_invalid_engines_rejected() {
    let old = r#"{"seed":0,"series":{"Fixed":36},"kontra_re":false,"opponents":{"Levels":["Medium","Medium"]}}"#;
    let c: SessionConfig = serde_json::from_str(old).unwrap();
    assert_eq!(c, SessionConfig::default());
    assert_eq!(serde_json::to_string(&c).unwrap(), old);
    for mode in [
        Opponents::Goldfish,
        Opponents::Mephisto,
        Opponents::GodMode,
        Opponents::Experimental,
        Opponents::Neural,
    ] {
        let invalid = SessionConfig {
            opponents: mode,
            ..config()
        };
        assert!(Session::try_new(invalid).is_err());
        assert!(
            serde_json::from_str::<SessionConfig>(&serde_json::to_string(&invalid).unwrap())
                .is_err()
        );
    }
    let c = config();
    assert_eq!(
        serde_json::from_str::<SessionConfig>(&serde_json::to_string(&c).unwrap()).unwrap(),
        c
    );
    let hybrid: SessionConfig = serde_json::from_str(
        &serde_json::to_string(&c)
            .unwrap()
            .replace("Neural", "Hybrid"),
    )
    .unwrap();
    assert_eq!(hybrid.engine, OpponentEngine::Hybrid);
}

#[test]
fn frozen_identity_round_trips_and_practice_rejects_substitution() {
    let original = Session::new(config());
    let legacy = Session::new(SessionConfig {
        opponents: Opponents::Neural,
        engine: OpponentEngine::Classic,
        ..config()
    });
    assert_eq!(
        original.opponent_job().unwrap().run(None).action,
        legacy.opponent_job().unwrap().run(None).action
    );
    let r = recording(&original);
    assert_eq!(r.format, 2);
    assert_eq!(r.engine, crate::RECORDING_ENGINE);
    assert_eq!(r.context.neural.as_ref(), original.neural_provenance());
    let decoded: GameRecording = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(decoded, r);
    decoded.reconstruct_privileged(0).unwrap();
    let live = Session::new(SessionConfig::default());
    let before = live.view();
    let practice = live.start_practice(&decoded).unwrap();
    assert_eq!(practice.neural_provenance(), original.neural_provenance());
    assert_eq!(practice.config(), original.config());
    assert_eq!(practice.game_seed(), original.game_seed());
    assert!(practice.export_current_recording_privileged().is_none());
    let mut different = decoded;
    different
        .context
        .neural
        .as_mut()
        .unwrap()
        .model
        .as_mut()
        .unwrap()
        .sha256 = [9; 32];
    // Replay applies recorded actions and does not require the installed weights.
    different.reconstruct_privileged(0).unwrap();
    let mut practice = live.start_practice(&different).unwrap();
    assert_eq!(
        practice.neural_provenance().unwrap().model,
        Err(ModelError::HashMismatch)
    );
    let job = practice.opponent_job().unwrap();
    let cancel = Cancel::new();
    cancel.cancel();
    let (mv, reason) = job.run_explained(Some(&cancel));
    assert_eq!(reason, Some(Fallback::Model(ModelError::HashMismatch)));
    practice.finish_opponent(mv).unwrap();
    assert_eq!(live.view(), before);
}

#[test]
fn legacy_neural_has_unknown_recorded_identity_and_visible_practice_fallback() {
    let mut r = recording(&Session::new(config()));
    r.format = 1;
    r.context.session.engine = OpponentEngine::Classic;
    r.context.session.opponents = Opponents::Neural;
    r.context.neural = None;
    let encoded = serde_json::to_string(&r).unwrap();
    assert!(!encoded.contains("neural"));
    let r: GameRecording = serde_json::from_str(&encoded).unwrap();
    r.reconstruct_privileged(0).unwrap();
    let live = Session::new(SessionConfig::default());
    let practice = live.start_practice(&r).unwrap();
    assert_eq!(
        practice.neural_provenance().unwrap().model,
        Err(ModelError::Missing)
    );
    assert_eq!(practice.config().opponents, Opponents::Neural);
    assert_eq!(
        Session::new(r.context.session)
            .neural_provenance()
            .unwrap()
            .fallback,
        Level::Strong
    );
}

#[test]
fn failures_and_cancelled_workers_keep_valid_seeded_series_and_restart() {
    let corrupt = NeuralModel::load(b"corrupt", &skat_nn::sha256(b"corrupt")).unwrap_err();
    for error in [
        ModelError::Missing,
        corrupt,
        ModelError::HashMismatch,
        ModelError::Encoder { found: 99 },
    ] {
        let mut a = Session::new(config());
        // Inject app-loader results; no global environment mutation or model IO.
        a.neural = Some(FrozenNeural::new(Err(error.clone())).configured(&a.config));
        a.recording.context.neural = a.neural_provenance().cloned();
        let mut b = a.clone();
        let initial_identity = a.neural_provenance().cloned();
        let cancel = Cancel::new();
        cancel.cancel();
        for _ in 0..512 {
            if a.view().series_finished {
                break;
            }
            if let Some(job) = a.opponent_job() {
                let Brain::Neural(opponent, _) = &job.brain else {
                    panic!("neural worker")
                };
                assert_eq!(
                    opponent.classic.profile,
                    a.config.level(job.player()).unwrap().profile()
                );
                let (mv, reason) = job.run_explained(Some(&cancel));
                assert_eq!(reason, Some(Fallback::Model(error.clone())));
                a.finish_opponent(mv).unwrap();
                b.finish_opponent(b.opponent_job().unwrap().run(Some(&cancel)))
                    .unwrap();
            } else {
                let command = a
                    .view()
                    .legal_actions
                    .first()
                    .copied()
                    .map_or(Command::NextGame, Command::Act);
                a.execute(command).unwrap();
                b.execute(command).unwrap();
            }
            assert_eq!(a.view(), b.view());
            assert_eq!(a.neural_provenance(), initial_identity.as_ref());
        }
        assert!(a.view().series_finished);
        assert_eq!(a.view().sheet.records.len(), 2);
        for r in a.completed_recordings_privileged() {
            r.reconstruct_privileged(r.actions.len()).unwrap();
        }
        let restarted = Session::new(a.config());
        assert!(b.opponent_job().is_none());
        assert_eq!(restarted.game_seed(), Session::new(config()).game_seed());
    }
}

#[test]
fn new_series_rejects_old_worker_even_with_identical_config() {
    let old = Session::new(config());
    let job = old.opponent_job().unwrap();
    let mut replacement = Session::new(config());
    let before = replacement.view();
    assert_eq!(
        replacement.finish_opponent(job.run(None)),
        Err(AppError::StaleOpponentMove)
    );
    assert_eq!(replacement.view(), before);
}

#[test]
fn neural_rejects_rookie_but_classic_retains_it() {
    for levels in [[Level::Rookie, Level::Easy], [Level::Strong, Level::Rookie]] {
        let c = SessionConfig {
            opponents: Opponents::Levels(levels),
            ..config()
        };
        assert!(Session::try_new(c).is_err());
        assert!(
            serde_json::from_str::<SessionConfig>(&serde_json::to_string(&c).unwrap()).is_err()
        );
        assert!(
            Session::try_new(SessionConfig {
                engine: OpponentEngine::Classic,
                ..c
            })
            .is_ok()
        );
    }
}

#[test]
fn tiny_model_levels_repeat_a_complete_series_and_match_classic_fallbacks() {
    let bytes = skat_nn::fixture_model(4, skat_nn::Phases::SPIEL);
    let model = std::sync::Arc::new(NeuralModel::load(&bytes, &skat_nn::sha256(&bytes)).unwrap());
    for level in [Level::Easy, Level::Medium, Level::Strong] {
        let config = SessionConfig {
            opponents: Opponents::Levels([level; 2]),
            ..config()
        };
        let mut a = Session::new(config);
        a.neural = Some(FrozenNeural::new(Ok(model.clone())).configured(&config));
        a.recording.context.neural = a.neural_provenance().cloned();
        let mut b = a.clone();
        let mut model_moves = 0;
        let cancel = Cancel::new();
        cancel.cancel();
        for _ in 0..512 {
            if a.view().series_finished {
                break;
            }
            if let Some(job) = a.opponent_job() {
                let (mv, fallback) = job.run_explained(Some(&cancel));
                if fallback.is_none() && a.game.phase() == Phase::Spiel {
                    model_moves += 1;
                }
                if fallback.is_some() {
                    let seat = a.game.current_seat().unwrap();
                    let player = PlayerId::on_seat(seat, a.dealer);
                    let expected = Opponent::new(level, a.ai_seeds[player.index() - 1])
                        .decide(&a.game.view(seat), Some(&cancel))
                        .unwrap();
                    assert_eq!(mv.action, expected.action);
                }
                a.finish_opponent(mv).unwrap();
                b.finish_opponent(b.opponent_job().unwrap().run(Some(&cancel)))
                    .unwrap();
            } else {
                let command = a
                    .view()
                    .legal_actions
                    .first()
                    .copied()
                    .map_or(Command::NextGame, Command::Act);
                a.execute(command).unwrap();
                b.execute(command).unwrap();
            }
            assert_eq!(a.view(), b.view());
        }
        assert!(a.view().series_finished);
        assert!(model_moves > 0, "exercise actual model card choices");
        for recording in a.completed_recordings_privileged() {
            recording
                .reconstruct_privileged(recording.actions.len())
                .unwrap();
        }
    }
}

#[test]
fn increment_a_greedy_recordings_keep_historical_fallback() {
    let mut original = Session::new(config());
    original.neural.as_mut().unwrap().provenance.policy_revision = "greedy-policy-1".into();
    original.recording.context.neural = original.neural_provenance().cloned();
    let mut recorded = recording(&original);
    // Increment A retained Classic Rookie in config without using it.
    recorded.context.session.opponents = Opponents::Levels([Level::Rookie, Level::Medium]);
    let recorded: GameRecording =
        serde_json::from_str(&serde_json::to_string(&recorded).unwrap()).unwrap();
    let live = Session::new(SessionConfig::default());
    let practice = live.start_practice(&recorded).unwrap();
    assert_eq!(
        practice.neural_provenance().unwrap().policy_revision,
        "greedy-policy-1"
    );
    let Brain::Neural(player, _) = practice.opponent_job().unwrap().brain else {
        panic!("neural")
    };
    assert_eq!(player.choice, skat_nn::Choice::Policy);
    assert_eq!(player.classic.profile, Level::Strong.profile());
    let mut invalid = recorded.clone();
    invalid.context.neural.as_mut().unwrap().policy_revision =
        skat_nn::PolicyLevel::REVISION.into();
    assert!(invalid.reconstruct_privileged(0).is_err());
    let mut invalid = recorded;
    invalid.context.neural.as_mut().unwrap().policy_revision = "unknown".into();
    assert!(invalid.reconstruct_privileged(0).is_err());
}

#[test]
fn hybrid_roundtrip_and_missing_model_use_selected_classic() {
    let config = SessionConfig {
        engine: OpponentEngine::Hybrid,
        ..config()
    };
    let mut session = Session::new(config);
    session.neural = Some(FrozenNeural::new(Err(ModelError::Missing)).configured(&config));
    session.recording.context.neural = session.neural_provenance().cloned();
    let recording = recording(&session);
    let decoded: GameRecording =
        serde_json::from_str(&serde_json::to_string(&recording).unwrap()).unwrap();
    decoded.reconstruct_privileged(0).unwrap();
    assert_eq!(decoded.context.session.engine, OpponentEngine::Hybrid);
    let job = session.opponent_job().unwrap();
    let seat = session.game.current_seat().unwrap();
    let player = PlayerId::on_seat(seat, session.dealer);
    let classic = Opponent::new(
        config.level(player).unwrap(),
        session.ai_seeds[player.index() - 1],
    );
    assert_eq!(
        job.run(None).action,
        classic
            .decide(&session.game.view(seat), None)
            .unwrap()
            .action
    );
    let practice = Session::new(SessionConfig::default())
        .start_practice(&decoded)
        .unwrap();
    assert_eq!(practice.config().engine, OpponentEngine::Hybrid);
}

#[test]
fn pre_play_engine_plays_a_series_with_the_pre_play_model_and_valid_recordings() {
    let config = SessionConfig {
        engine: OpponentEngine::PrePlay,
        opponents: Opponents::Levels([Level::Strong; 2]),
        ..config()
    };
    let mut s = Session::new(config);
    let frozen = s.neural.as_ref().unwrap();
    assert_eq!(frozen.provenance.policy_revision, "pre-play-levels-1");
    assert_eq!(
        frozen.model.as_ref().unwrap().sha256(),
        crate::pre_play_model().unwrap().sha256(),
        "the pre-play model, not the card-play model"
    );
    assert_ne!(
        frozen.model.as_ref().unwrap().sha256(),
        crate::neural_model().unwrap().sha256()
    );
    let mut phases = std::collections::BTreeSet::new();
    for _ in 0..512 {
        if s.view().series_finished {
            break;
        }
        if let Some(job) = s.opponent_job() {
            let phase = s.game.phase();
            let (mv, fallback) = job.run_explained(None);
            if fallback.is_none() {
                phases.insert(format!("{phase:?}"));
            }
            s.finish_opponent(mv).unwrap();
        } else {
            let command = s
                .view()
                .legal_actions
                .first()
                .copied()
                .map_or(Command::NextGame, Command::Act);
            s.execute(command).unwrap();
        }
    }
    assert!(s.view().series_finished);
    assert!(
        phases.contains("Reizen") && phases.contains("Spiel"),
        "{phases:?}"
    );
    for recording in s.completed_recordings_privileged() {
        let bytes = serde_json::to_vec(&recording).unwrap();
        let back: GameRecording = serde_json::from_slice(&bytes).unwrap();
        back.reconstruct_privileged(back.actions.len()).unwrap();
        let practice = Session::new(SessionConfig::default())
            .start_practice(&back)
            .unwrap();
        assert_eq!(practice.neural_provenance(), s.neural_provenance());
    }
}
