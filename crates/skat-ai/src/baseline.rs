// SPDX-License-Identifier: EUPL-1.2

//! Baseline opponent: fixed rules of thumb on top of `skat-analysis`. It has
//! no randomness, so the same view always gives the same action.

use skat_analysis::{baseline, policy};
use skat_core::{Action, Phase, PlayerView};

/// A deliberately simple, deterministic opponent: the shared continuation
/// policy of `skat-analysis` for the decisions before card play, plus rules
/// of thumb for the cards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineOpponent;

impl BaselineOpponent {
    /// Chooses one of `view.legal_actions`; `None` if the list is empty,
    /// i.e. it is not this seat's turn.
    pub fn decide(&self, view: &PlayerView) -> Option<Action> {
        let legal = &view.legal_actions;
        if legal.is_empty() {
            return None;
        }
        let wanted = match view.phase {
            Phase::Reizen => {
                let (_, limit) = policy::choose_game(view.hand);
                policy::bid(limit, view.bid, legal)
            }
            Phase::SkatAufnahme => policy::hand_or_pickup(view.hand),
            Phase::Druecken => Action::Discard(policy::discard(view.hand)),
            Phase::Ansage => policy::announce(
                view.hand,
                view.own_skat.unwrap_or_default(),
                view.bid,
                view.hand_game == Some(true),
                legal,
            ),
            Phase::Spiel => baseline::play(view),
            Phase::Beendet | Phase::Eingepasst => return None,
        };
        if legal.contains(&wanted) {
            Some(wanted)
        } else {
            legal.first().copied()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::{Rng, SeedableRng};
    use skat_core::{Game, RuleConfig, Seat};

    /// Plays `seed` with the baseline opponent on every seat.
    fn self_play(seed: u64, kontra_re: bool) -> Game {
        let mut game = Game::new(seed, RuleConfig { kontra_re });
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            let action = BaselineOpponent.decide(&view).expect("legal action");
            assert!(
                view.legal_actions.contains(&action),
                "seed {seed}: {action:?}"
            );
            game.apply(action).unwrap();
        }
        game
    }

    #[test]
    fn self_play_finishes_legally_and_varies() {
        let mut games = std::collections::BTreeMap::new();
        let mut kontras = 0;
        for seed in 0..500 {
            let game = self_play(seed, true);
            if let Some(c) = game.contract() {
                *games.entry(format!("{:?}", c.game)).or_insert(0) += 1;
            }
            if game.kontra() != skat_core::KontraLevel::None {
                kontras += 1;
            }
        }
        // All four suits and Grand; Null needs a rare hand (NULL_RISKS).
        assert!(games.len() >= 5, "{games:?}");
        assert!(kontras > 0);
    }

    #[test]
    fn decisions_depend_only_on_the_view() {
        let mut rng = ChaCha8Rng::seed_from_u64(9);
        for seed in 0..200 {
            let mut game = Game::new(seed, RuleConfig { kontra_re: true });
            while let Some(seat) = game.current_seat() {
                let view = game.view(seat);
                let action = BaselineOpponent.decide(&view).unwrap();
                assert_eq!(BaselineOpponent.decide(&view.clone()), Some(action));
                // Mix in random legal actions so more positions are reached.
                let action = if rng.next_u32() % 4 == 0 {
                    let i = rng.next_u32() as usize % view.legal_actions.len();
                    view.legal_actions[i]
                } else {
                    action
                };
                game.apply(action).unwrap();
            }
        }
    }

    #[test]
    fn frozen_easy_choices_before_rule_extraction() {
        // Frozen on origin/main ecfb2fe before moving the rule path.
        let mut hash = 0xcbf29ce484222325u64;
        for seed in 0..200 {
            let mut game = Game::new(seed, RuleConfig { kontra_re: true });
            while let Some(seat) = game.current_seat() {
                let action = BaselineOpponent.decide(&game.view(seat)).unwrap();
                for byte in format!("{action:?}").bytes() {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                }
                game.apply(action).unwrap();
            }
        }
        assert_eq!(hash, 12_969_225_982_751_591_822);
    }

    #[test]
    fn no_decision_without_turn() {
        let game = Game::new(1, RuleConfig::default());
        let waiting = Seat::ALL
            .into_iter()
            .find(|&s| Some(s) != game.current_seat())
            .unwrap();
        assert_eq!(BaselineOpponent.decide(&game.view(waiting)), None);
    }
}
