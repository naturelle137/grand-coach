// SPDX-License-Identifier: EUPL-1.2

//! The neural opponent adapter (#69): a tiny random-weight fixture model
//! plays complete seeded games in the arena, alone and against Classic
//! profiles, reproducibly; every model problem falls back to the Classic
//! decision with its reason.

use std::sync::Arc;

use burn::tensor::Device;
use skat_ai::arena::{compare_with, play_with};
use skat_ai::{Level, NeuralOpponent, Opponent, Player};
use skat_core::{Game, Phase, PlayerView, RuleConfig, Seat};
use skat_nn::{
    ENCODER_VERSION, FIXTURE_HIDDEN, Fallback, ModelError, ModelHeader, NetShape, NeuralModel,
    Phases, PolicyValueNet, fixture_model, sha256, write_model,
};

const RULES: RuleConfig = RuleConfig { kontra_re: true };

fn model(bytes: &[u8]) -> Result<Arc<NeuralModel>, ModelError> {
    NeuralModel::load(bytes, &sha256(bytes)).map(Arc::new)
}

fn neural(phases: Phases, seed: u64) -> NeuralOpponent {
    let bytes = fixture_model(11, phases);
    NeuralOpponent::new(model(&bytes), Opponent::new(Level::Easy, seed))
}

/// Plays a game with `players` and returns every decision view with the
/// player's explanation.
fn decisions(
    deal: u64,
    players: &[NeuralOpponent; 3],
) -> Vec<(PlayerView, skat_ai::NeuralDecision)> {
    let mut game = Game::new(deal, RULES);
    let mut out = Vec::new();
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let d = players[seat.index()].decide_explained(&view, None).unwrap();
        assert!(view.legal_actions.contains(&d.decision.action));
        game.apply(d.decision.action).unwrap();
        out.push((view, d));
    }
    out
}

#[test]
fn fixture_model_plays_complete_games_without_illegal_actions() {
    let players = Seat::ALL.map(|s| neural(Phases::ALL, s.index() as u64));
    let mut played = 0;
    let mut phases = std::collections::BTreeSet::new();
    for deal in 0..40 {
        // `play_with` panics on an illegal action.
        let game = play_with(deal, RULES, [&players[0], &players[1], &players[2]]);
        assert!(matches!(game.phase(), Phase::Beendet | Phase::Eingepasst));
        played += usize::from(game.result().is_some());
        for (view, d) in decisions(deal, &players) {
            assert_eq!(d.fallback, None, "the fixture covers every phase");
            assert!(d.value.is_some_and(f32::is_finite));
            phases.insert(format!("{:?}", view.phase));
        }
    }
    assert!(played > 0, "some games are played to the end");
    assert!(phases.len() >= 2, "{phases:?}");
}

#[test]
fn fixture_model_against_classic_profiles_in_the_arena() {
    let fixture = model(&fixture_model(11, Phases::ALL)).unwrap();
    for level in [Level::Rookie, Level::Easy] {
        for deal in 0..4 {
            let a = |g: usize, s: Seat| {
                let classic = Opponent::new(Level::Easy, (g * 3 + s.index()) as u64);
                NeuralOpponent::new(Ok(fixture.clone()), classic)
            };
            let b = |g: usize, s: Seat| Opponent::new(level, 100 + (g * 3 + s.index()) as u64);
            let first = compare_with(deal, RULES, a, b);
            assert_eq!(first.a.seat_games, 9);
            assert_eq!(first.b.seat_games, 9);
            assert_eq!(first, compare_with(deal, RULES, a, b), "reproducible");
        }
    }
}

#[test]
fn fixed_model_and_seeds_reproduce_the_game() {
    let players = Seat::ALL.map(|s| neural(Phases::ALL, s.index() as u64));
    let again = Seat::ALL.map(|s| neural(Phases::ALL, s.index() as u64));
    for deal in 0..10 {
        let a = play_with(deal, RULES, [&players[0], &players[1], &players[2]]);
        let b = play_with(deal, RULES, [&again[0], &again[1], &again[2]]);
        assert_eq!(a, b, "deal {deal}");
    }
}

/// Every decision of `opponent` along seeded games equals the Classic
/// opponent's decision and carries `reason`.
fn falls_back_everywhere(opponent: &NeuralOpponent, reason: impl Fn(&PlayerView) -> Fallback) {
    let players = [opponent.clone(), opponent.clone(), opponent.clone()];
    for deal in 0..5 {
        for (view, d) in decisions(deal, &players) {
            assert_eq!(d.fallback, Some(reason(&view)));
            assert_eq!(d.value, None);
            assert_eq!(Some(d.decision), opponent.classic.decide(&view, None));
        }
    }
}

#[test]
fn missing_or_broken_models_fall_back_to_classic() {
    let classic = Opponent::new(Level::Easy, 5);
    let missing = NeuralOpponent::new(Err(ModelError::Missing), classic);
    falls_back_everywhere(&missing, |_| Fallback::Model(ModelError::Missing));

    let mut bytes = fixture_model(11, Phases::ALL);
    bytes[6] ^= 0xff;
    let error = model(&bytes).unwrap_err();
    assert!(matches!(error, ModelError::Encoder { .. }));
    let incompatible = NeuralOpponent::new(Err(error.clone()), classic);
    falls_back_everywhere(&incompatible, |_| Fallback::Model(error.clone()));
}

#[test]
fn non_finite_outputs_fall_back_to_classic() {
    let shape = NetShape::current(FIXTURE_HIDDEN);
    let net = PolicyValueNet::from_weights(shape, &Device::flex(), || f32::NAN);
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape,
        phases: Phases::ALL,
    };
    let bytes = write_model(&header, net).unwrap();
    let opponent = NeuralOpponent::new(model(&bytes), Opponent::new(Level::Easy, 5));
    falls_back_everywhere(&opponent, |_| Fallback::NonFinite);
}

#[test]
fn uncovered_phases_fall_back_to_classic() {
    let opponent = neural(Phases::SPIEL, 9);
    let players = [opponent.clone(), opponent.clone(), opponent.clone()];
    let (mut model_decided, mut classic_decided) = (0, 0);
    for deal in 0..10 {
        for (view, d) in decisions(deal, &players) {
            if view.phase == Phase::Spiel {
                assert_eq!(d.fallback, None);
                model_decided += 1;
            } else {
                assert_eq!(d.fallback, Some(Fallback::UnsupportedPhase(view.phase)));
                assert_eq!(Some(d.decision), opponent.classic.decide(&view, None));
                classic_decided += 1;
            }
        }
    }
    assert!(model_decided > 0 && classic_decided > 0);
}

#[test]
fn no_decision_for_a_seat_not_to_move() {
    let opponent = neural(Phases::ALL, 1);
    let game = Game::new(3, RULES);
    let idle = Seat::ALL
        .into_iter()
        .find(|&s| Some(s) != game.current_seat())
        .unwrap();
    assert_eq!(opponent.decide_explained(&game.view(idle), None), None);
    assert_eq!(Player::decide(&opponent, &game.view(idle), None), None);
}
