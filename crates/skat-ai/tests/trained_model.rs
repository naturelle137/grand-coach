// SPDX-License-Identifier: EUPL-1.2

//! The shipped card-play model (`assets/models/card-play.gcnn`, #73): it
//! loads with the hash recorded in its training manifest, decides every
//! card-play decision of complete seeded games without an illegal action
//! (pre-play falls back to Classic), plays against Classic levels in the
//! arena, and gives the same decision whenever the seat's view is the same,
//! whatever the concealed cards are (D-23).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use skat_ai::arena::{compare_with, play_with};
use skat_ai::{Level, NeuralOpponent, Opponent};
use skat_core::{Action, Card, CardSet, Deal, Game, Phase, RuleConfig, Seat};
use skat_nn::{Fallback, NeuralModel, Phases, sha256};

const RULES: RuleConfig = RuleConfig { kontra_re: true };

fn models() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models")
}

/// The model, loaded with the hash of its training manifest.
fn model() -> Arc<NeuralModel> {
    let bytes = std::fs::read(models().join("card-play.gcnn")).expect("the shipped model");
    let manifest = std::fs::read_to_string(models().join("card-play.json")).unwrap();
    let recorded = manifest
        .split("\"model_sha256\": \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the manifest records the model hash");
    let hash: String = sha256(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hash, recorded, "model file and manifest belong together");
    Arc::new(NeuralModel::load(&bytes, &sha256(&bytes)).expect("the shipped model loads"))
}

fn neural(model: &Arc<NeuralModel>, seed: u64) -> NeuralOpponent {
    NeuralOpponent::new(Ok(model.clone()), Opponent::new(Level::Easy, seed))
}

#[test]
fn shipped_model_decides_card_play_only() {
    let m = model();
    assert_eq!(m.header().phases, Phases::SPIEL);
    let players = Seat::ALL.map(|s| neural(&m, s.index() as u64));
    let (mut by_model, mut finished) = (0, 0);
    for deal in 0..20 {
        // `play_with` panics on an illegal action.
        let game = play_with(deal, RULES, [&players[0], &players[1], &players[2]]);
        finished += usize::from(game.phase() == Phase::Beendet);
        let mut game = Game::new(deal, RULES);
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            let d = players[seat.index()].decide_explained(&view, None).unwrap();
            assert!(view.legal_actions.contains(&d.decision.action));
            if view.phase == Phase::Spiel {
                assert_eq!(d.fallback, None, "deal {deal}: the model plays the cards");
                assert!(d.value.is_some_and(f32::is_finite));
                by_model += 1;
            } else {
                assert_eq!(d.fallback, Some(Fallback::UnsupportedPhase(view.phase)));
            }
            game.apply(d.decision.action).unwrap();
        }
    }
    assert!(finished > 10 && by_model > 300, "{finished} {by_model}");
}

#[test]
fn shipped_model_plays_against_classic_levels_in_the_arena() {
    let m = model();
    for level in [Level::Rookie, Level::Easy] {
        for deal in 0..2 {
            let c = compare_with(
                1000 + deal,
                RULES,
                |g, s| neural(&m, (g * 3 + s.index()) as u64),
                |g, s| Opponent::new(level, (g * 3 + s.index()) as u64),
            );
            assert!(c.difference().is_finite());
        }
    }
}

fn replay(deal: Deal, actions: &[Action]) -> Option<Game> {
    let mut game = Game::from_deal(deal, RULES);
    for &a in actions {
        game.apply(a).ok()?;
    }
    Some(game)
}

fn swap(set: CardSet, x: Card, y: Card) -> CardSet {
    let pair = CardSet::single(x) | CardSet::single(y);
    if (set & pair).len() == 1 {
        (set - pair) | (pair - set)
    } else {
        set
    }
}

#[test]
fn shipped_model_sees_only_the_permitted_view() {
    let m = model();
    let players = Seat::ALL.map(|s| neural(&m, s.index() as u64));
    let mut compared = 0;
    for seed in 0..4 {
        let mut game = Game::new(seed, RULES);
        let all = game.omniscient_view();
        let deal = Deal::new(all.original_hands, all.original_skat).unwrap();
        let mut actions = Vec::new();
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if view.phase == Phase::Spiel && actions.len() % 4 == 0 {
                let expected = m.evaluate(&view).unwrap();
                // Swap two cards the seat cannot see; whenever the same
                // history stays legal and the view is the same, so is the
                // model's evaluation.
                let mut seen = view.hand | view.own_skat.unwrap_or_default();
                seen |= view.declarer_cards.unwrap_or_default();
                for trick in &view.tricks {
                    seen |= trick.cards.iter().copied().collect();
                }
                seen |= view.current_trick.cards.iter().flatten().copied().collect();
                let unseen: Vec<Card> = (CardSet::ALL - seen).into_iter().collect();
                for (i, &x) in unseen.iter().enumerate().step_by(3) {
                    for &y in unseen[i + 1..].iter().step_by(2) {
                        let hands = Seat::ALL.map(|s| swap(deal.hand(s), x, y));
                        let alt = Deal::new(hands, swap(deal.skat(), x, y)).unwrap();
                        let alt_actions: Vec<Action> = actions
                            .iter()
                            .map(|&a| match a {
                                Action::Discard(c) => Action::Discard(swap(c, x, y)),
                                other => other,
                            })
                            .collect();
                        if let Some(g) = replay(alt, &alt_actions)
                            && g.view(seat) == view
                            && g.omniscient_view().hands != game.omniscient_view().hands
                        {
                            assert_eq!(m.evaluate(&g.view(seat)).unwrap(), expected);
                            compared += 1;
                        }
                    }
                }
            }
            let action = players[seat.index()]
                .decide_explained(&view, None)
                .unwrap()
                .decision
                .action;
            game.apply(action).unwrap();
            actions.push(action);
        }
    }
    assert!(compared > 50, "{compared} comparisons");
}
