// SPDX-License-Identifier: EUPL-1.2

//! The neural opponent adapter (#69): a tiny random-weight fixture model
//! plays complete seeded games in the arena, alone and against Classic
//! profiles, reproducibly; every model problem falls back to the Classic
//! decision with its reason.

use std::sync::Arc;

use burn::tensor::Device;
use skat_ai::arena::{compare_with, play_with};
use skat_ai::{BaselineOpponent, Level, NeuralOpponent, Opponent, Player, Profile};
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig, Seat};
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
            // Only a Kontra choice of the rules of thumb has no model value.
            let rules_kontra = matches!(d.decision.action, Action::Kontra | Action::KeinKontra);
            assert!(d.value.is_some_and(f32::is_finite) || rules_kontra);
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

#[test]
fn kontra_follows_the_rules_of_thumb_unless_the_profile_analyses_it() {
    let kontra_choice = |a: &Action| matches!(a, Action::Kontra | Action::KeinKontra);
    let (mut by_rules, mut by_model) = (0, 0);
    for search_kontra in [false, true] {
        let players = Seat::ALL.map(|s| {
            let mut p = neural(Phases::ALL, s.index() as u64);
            p.classic.profile = Profile {
                search_kontra,
                ..p.classic.profile
            };
            p
        });
        for deal in 0..40 {
            for (view, d) in decisions(deal, &players) {
                if !view.legal_actions.iter().any(kontra_choice) {
                    continue;
                }
                let rules = BaselineOpponent.decide(&view).unwrap();
                if search_kontra {
                    assert!(d.value.is_some(), "the model decides");
                    by_model += 1;
                } else if kontra_choice(&rules) {
                    assert_eq!(d.decision.action, rules);
                    assert_eq!((d.value, d.fallback), (None, None));
                    by_rules += 1;
                } else {
                    assert!(!kontra_choice(&d.decision.action), "{d:?}");
                    assert!(d.value.is_some(), "the model chooses the card");
                }
            }
        }
    }
    assert!(by_rules > 0 && by_model > 0, "{by_rules} {by_model}");
}

#[test]
fn learned_phase_mask_leaves_other_phases_to_classic() {
    let mask = Phases::from_bits(0b1_0001).unwrap(); // Reizen and Spiel
    let players =
        Seat::ALL.map(|s| neural(Phases::ALL, s.index() as u64).with_learned_phases(mask));
    let mut seen = Vec::new();
    for deal in 0..20 {
        for (view, d) in decisions(deal, &players) {
            if mask.contains(view.phase) {
                assert_eq!(d.fallback, None);
            } else if d.value.is_none() && d.fallback.is_some() {
                assert_eq!(d.fallback, Some(Fallback::UnsupportedPhase(view.phase)));
                seen.push(view.phase);
            }
        }
    }
    assert!(
        seen.contains(&Phase::Druecken) || seen.contains(&Phase::Ansage),
        "{seen:?}"
    );
}

/// The experimental pre-play model shipped in `assets/models` (#74).
fn pre_play(seed: u64) -> Option<NeuralOpponent> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
    let bytes = std::fs::read(dir.join("pre-play.gcnn")).ok()?;
    let manifest = std::fs::read_to_string(dir.join("pre-play.json")).ok()?;
    let expected = manifest
        .split("\"model_sha256\"")
        .nth(1)?
        .split('"')
        .nth(1)?
        .to_owned();
    let hash: String = sha256(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hash, expected, "manifest names the model file");
    Some(NeuralOpponent::new(
        model(&bytes),
        Opponent::new(Level::Strong, seed),
    ))
}

#[test]
fn shipped_pre_play_model_plays_complete_games_in_every_phase_with_legal_masked_actions() {
    let players = [0, 1, 2].map(|s| pre_play(s).expect("pre-play model is shipped"));
    let mut by_phase = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for deal in 0..40 {
        for (view, d) in decisions(deal, &players) {
            assert!(view.legal_actions.contains(&d.decision.action));
            let e = by_phase.entry(format!("{:?}", view.phase)).or_default();
            if d.fallback.is_none() {
                e.0 += 1;
            }
            e.1 += 1;
        }
    }
    // Kontra/Re follow the rules of thumb and are reported without a model
    // value; every other decision of a covered phase is the model's.
    let model_decided: usize = by_phase.values().map(|(m, _)| m).sum();
    let all: usize = by_phase.values().map(|(_, a)| a).sum();
    assert!(by_phase.len() >= 4, "{by_phase:?}");
    assert!(model_decided * 10 > all * 9, "{by_phase:?}");
    for (phase, (m, _)) in &by_phase {
        assert!(*m > 0, "model never decided {phase}: {by_phase:?}");
    }
}

#[test]
fn pre_play_decisions_depend_only_on_the_permitted_view() {
    let player = pre_play(3).expect("pre-play model is shipped");
    let mut compared = 0;
    for deal in 0..20 {
        let real = skat_core::Deal::from_seed(deal);
        for me in Seat::ALL {
            // Hidden hands of the other two seats are exchanged; the
            // Skat and the acting seat's own hand stay.
            let others = Seat::ALL
                .into_iter()
                .filter(|s| *s != me)
                .collect::<Vec<_>>();
            let mut hands = Seat::ALL.map(|s| real.hand(s));
            hands.swap(others[0].index(), others[1].index());
            let fake = skat_core::Deal::new(hands, real.skat()).unwrap();
            let mut a = Game::from_deal(real, RULES);
            let mut b = Game::from_deal(fake, RULES);
            while let Some(seat) = a.current_seat() {
                let view = a.view(seat);
                let d = player.decide_explained(&view, None).unwrap();
                if seat == me {
                    assert_eq!(view, b.view(me), "view carries hidden cards");
                    let d2 = player.decide_explained(&b.view(me), None).unwrap();
                    assert_eq!(d, d2);
                    compared += 1;
                }
                if a.apply(d.decision.action).is_err() || b.apply(d.decision.action).is_err() {
                    break;
                }
            }
        }
    }
    assert!(compared > 100);
}
