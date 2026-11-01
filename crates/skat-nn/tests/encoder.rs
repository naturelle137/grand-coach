// SPDX-License-Identifier: EUPL-1.2

//! The encoder reads only the permitted view (D-23, #69):
//!
//! - swapping concealed cards while a seat's view stays the same leaves its
//!   features unchanged, for every seat and phase, including a known own
//!   Skat and revealed Ouvert cards,
//! - the own Skat appears only for the declarer after Drücken, Ouvert cards
//!   only after the reveal, for every role.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_core::{Action, Card, CardSet, Deal, Game, Phase, RuleConfig, Seat};
use skat_nn::encoder::{OUVERT_BLOCK, OWN_SKAT_BLOCK};
use skat_nn::{FEATURES, encode};

const RULES: RuleConfig = RuleConfig { kontra_re: true };

/// A game history steered towards rare decisions: Hand, Ouvert, Kontra and
/// Re are chosen often; everything else is the first or last legal action.
fn history(seed: u64) -> (Deal, Vec<Action>) {
    let mut game = Game::new(seed, RULES);
    let all = game.omniscient_view();
    let deal = Deal::new(all.original_hands, all.original_skat).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut actions = Vec::new();
    while let Some(seat) = game.current_seat() {
        let legal = game.view(seat).legal_actions;
        let coin = rng.next_u32().is_multiple_of(2);
        let ouvert = legal
            .iter()
            .copied()
            .rfind(|a| matches!(a, Action::Announce(x) if x.ouvert));
        let action = match game.phase() {
            // Bid a little so that someone declares.
            Phase::Reizen => {
                let bid = legal.iter().copied().find(|a| match a {
                    Action::Bid(v) => *v <= 24,
                    Action::Hold => true,
                    _ => false,
                });
                bid.unwrap_or(Action::Pass)
            }
            Phase::SkatAufnahme if coin => Action::Hand,
            Phase::Ansage if coin && ouvert.is_some() => ouvert.unwrap(),
            Phase::Spiel if coin && legal.contains(&Action::Kontra) => Action::Kontra,
            Phase::Spiel if coin && legal.contains(&Action::Re) => Action::Re,
            _ => legal[rng.next_u32() as usize % legal.len()],
        };
        game.apply(action).unwrap();
        actions.push(action);
    }
    (deal, actions)
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

/// Up to `limit` games with the same history in which two cards `seat`
/// cannot see have swapped places, `seat`'s view is identical and the
/// concealed cards differ.
fn alternatives(deal: Deal, actions: &[Action], seat: Seat, limit: usize) -> Vec<Game> {
    let game = replay(deal, actions).unwrap();
    let view = game.view(seat);
    let mut seen = view.hand | view.own_skat.unwrap_or_default();
    seen |= view.declarer_cards.unwrap_or_default();
    for trick in &view.tricks {
        seen |= trick.cards.iter().copied().collect();
    }
    seen |= view.current_trick.cards.iter().flatten().copied().collect();
    let concealed = |g: &Game| {
        let all = g.omniscient_view();
        (all.hands, all.skat)
    };
    let unseen: Vec<Card> = (CardSet::ALL - seen).into_iter().collect();
    let mut out = Vec::new();
    for (i, &x) in unseen.iter().enumerate() {
        for &y in &unseen[i + 1..] {
            let hands = Seat::ALL.map(|s| swap(deal.hand(s), x, y));
            let alt = Deal::new(hands, swap(deal.skat(), x, y)).unwrap();
            let alt_actions: Vec<Action> = actions
                .iter()
                .map(|&a| match a {
                    Action::Discard(cards) => Action::Discard(swap(cards, x, y)),
                    other => other,
                })
                .collect();
            if let Some(g) = replay(alt, &alt_actions)
                && g.view(seat) == view
                && concealed(&g) != concealed(&game)
            {
                out.push(g);
                if out.len() == limit {
                    return out;
                }
            }
        }
    }
    out
}

#[test]
fn hidden_deal_substitution_gives_identical_features() {
    let mut compared = 0;
    let (mut with_skat, mut with_ouvert) = (0, 0);
    for seed in 0..12 {
        let (deal, actions) = history(seed);
        for n in (0..=actions.len()).step_by(3) {
            let game = replay(deal, &actions[..n]).unwrap();
            for seat in Seat::ALL {
                let view = game.view(seat);
                let features = encode(&view);
                for alt in alternatives(deal, &actions[..n], seat, 3) {
                    assert_eq!(encode(&alt.view(seat)), features, "seed {seed}, step {n}");
                    compared += 1;
                    with_skat += usize::from(view.own_skat.is_some());
                    with_ouvert += usize::from(view.declarer_cards.is_some());
                }
            }
        }
    }
    assert!(compared > 500, "{compared} comparisons");
    assert!(
        with_skat > 0 && with_ouvert > 0,
        "{with_skat} {with_ouvert}"
    );
}

fn block_cards(features: &[f32], block: std::ops::Range<usize>) -> CardSet {
    features[block]
        .iter()
        .enumerate()
        .filter(|&(_, &x)| x == 1.0)
        .filter_map(|(i, _)| Card::from_index(i as u8))
        .collect()
}

#[test]
fn skat_and_ouvert_cards_appear_only_after_their_lawful_reveal() {
    // [declarer, defender, bidding] × [own Skat seen, Ouvert seen]
    let mut roles = [[0usize; 2]; 3];
    for seed in 0..200 {
        let (deal, actions) = history(seed);
        let mut game = Game::from_deal(deal, RULES);
        let mut discarded = false;
        for step in 0..=actions.len() {
            let all = game.omniscient_view();
            for seat in Seat::ALL {
                let features = encode(&game.view(seat));
                assert_eq!(features.len(), FEATURES);
                let declarer = game.declarer() == Some(seat);
                let skat = block_cards(&features, OWN_SKAT_BLOCK);
                if declarer && discarded {
                    assert_eq!(skat, all.skat, "seed {seed}, step {step}");
                } else {
                    assert!(skat.is_empty(), "seed {seed}, step {step}, {seat:?}");
                }
                let ouvert = block_cards(&features, OUVERT_BLOCK);
                if game.is_revealed() {
                    let d = game.declarer().unwrap();
                    assert!(game.contract().unwrap().ouvert);
                    assert_eq!(ouvert, all.hands[d.index()], "seed {seed}, step {step}");
                } else {
                    assert!(ouvert.is_empty(), "seed {seed}, step {step}, {seat:?}");
                }
                let role = match game.declarer() {
                    None => 2,
                    Some(_) if declarer => 0,
                    Some(_) => 1,
                };
                roles[role][0] += usize::from(!skat.is_empty());
                roles[role][1] += usize::from(!ouvert.is_empty());
            }
            if let Some(&action) = actions.get(step) {
                discarded |= matches!(action, Action::Discard(_));
                game.apply(action).unwrap();
            }
        }
    }
    // The declarer sees its own Skat; nobody sees it otherwise. Revealed
    // Ouvert cards are seen by every role in card play.
    assert!(
        roles[0][0] > 0 && roles[1][0] == 0 && roles[2][0] == 0,
        "{roles:?}"
    );
    assert!(
        roles[0][1] > 0 && roles[1][1] > 0 && roles[2][1] == 0,
        "{roles:?}"
    );
}
