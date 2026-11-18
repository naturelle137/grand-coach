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
    assert!(
        serde_json::from_str::<SessionConfig>(
            &serde_json::to_string(&c)
                .unwrap()
                .replace("Neural", "Hybrid")
        )
        .is_err()
    );
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
        a.neural = Some(FrozenNeural::new(Err(error.clone())));
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
