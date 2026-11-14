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
            Phase::Reizen => policy::bid_rule(view)?.action,
            Phase::SkatAufnahme => policy::pickup_rule(view)?.action,
            Phase::Druecken => policy::discard_rule(view)?.action,
            Phase::Ansage => policy::announce_rule(view)?.action,
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
    use skat_analysis::policy;
    use skat_core::{CardSet, Game, GameType, RuleConfig, Seat};

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
    fn shared_bid_rule_is_easys_reizen_decision_with_public_stake() {
        let mut rng = ChaCha8Rng::seed_from_u64(148);
        let (mut holds, mut passes, mut bids) = (0, 0, 0);
        for seed in 0..500 {
            let mut game = Game::new(seed, RuleConfig { kontra_re: true });
            while game.phase() == Phase::Reizen {
                let view = game.view(game.current_seat().unwrap());
                let rule = policy::bid_rule(&view).unwrap();
                assert_eq!(BaselineOpponent.decide(&view), Some(rule.action));
                assert!(!rule.fallback && view.legal_actions.contains(&rule.action));
                assert_eq!(rule.current, view.bid);
                assert_eq!(rule.holds, view.legal_actions.contains(&Action::Hold));
                let within = rule.stake.is_some_and(|v| v <= rule.ceiling);
                assert_eq!(within, rule.action != Action::Pass, "{rule:?}");
                match rule.action {
                    Action::Hold => holds += 1,
                    Action::Bid(v) => {
                        assert_eq!(Some(v), rule.stake);
                        bids += 1;
                    }
                    _ => passes += 1,
                }
                // Random legal calls reach jumps and higher current bids.
                let action = if rng.next_u32() % 3 == 0 {
                    let i = rng.next_u32() as usize % view.legal_actions.len();
                    view.legal_actions[i]
                } else {
                    rule.action
                };
                game.apply(action).unwrap();
            }
        }
        assert!(holds > 0 && passes > 0 && bids > 0);
    }

    #[test]
    fn shared_pickup_and_discard_rules_are_easys_decisions() {
        let (mut hands, mut pickups, mut ties) = (0, 0, 0);
        for seed in 0..300 {
            let mut game = Game::new(seed, RuleConfig::default());
            while let Some(seat) = game.current_seat() {
                let view = game.view(seat);
                let action = BaselineOpponent.decide(&view).unwrap();
                match view.phase {
                    Phase::SkatAufnahme => {
                        let rule = policy::pickup_rule(&view).unwrap();
                        assert_eq!(rule.action, action);
                        assert!(!rule.fallback);
                        assert_eq!(rule.strength >= rule.threshold, action == Action::Hand);
                        if action == Action::Hand {
                            hands += 1;
                        } else {
                            pickups += 1;
                        }
                    }
                    Phase::Druecken => {
                        let rule = policy::discard_rule(&view).unwrap();
                        assert_eq!(rule.action, action);
                        assert!(!rule.fallback);
                        let put: CardSet = rule.reasons.iter().map(|r| r.0).collect();
                        assert_eq!(Action::Discard(put), action);
                        assert!(put.is_subset(rule.eligible));
                        ties += usize::from(rule.eligible.len() > 2);
                    }
                    _ => {
                        assert!(policy::pickup_rule(&view).is_none());
                        assert!(policy::discard_rule(&view).is_none());
                    }
                }
                if game.phase() == Phase::Spiel {
                    break;
                }
                game.apply(action).unwrap();
            }
        }
        assert!(
            hands > 0 && pickups > 0 && ties > 0,
            "{hands} {pickups} {ties}"
        );
    }

    /// Ansage views reached with random legal bids, pickups and discards, so
    /// that public bids above the preferred game, Hand and Null occur.
    fn announce_views(seeds: std::ops::Range<u64>) -> Vec<skat_core::PlayerView> {
        let mut rng = ChaCha8Rng::seed_from_u64(151);
        let mut views = Vec::new();
        for seed in seeds {
            let mut game = Game::new(seed, RuleConfig::default());
            while let Some(seat) = game.current_seat() {
                let view = game.view(seat);
                if view.phase == Phase::Ansage {
                    views.push(view);
                    break;
                }
                let action = if rng.next_u32() % 3 == 0 {
                    let i = rng.next_u32() as usize % view.legal_actions.len();
                    view.legal_actions[i]
                } else {
                    BaselineOpponent.decide(&view).unwrap()
                };
                game.apply(action).unwrap();
            }
        }
        views
    }

    #[test]
    fn frozen_easy_announcements_before_rule_extraction() {
        // Frozen on origin/main 16c0414 (policy::announce) before extraction.
        let mut hash = 0xcbf29ce484222325u64;
        let views = announce_views(0..2000);
        for view in &views {
            let action = policy::announce(
                view.hand,
                view.own_skat.unwrap_or_default(),
                view.bid,
                view.hand_game == Some(true),
                &view.legal_actions,
            );
            assert_eq!(BaselineOpponent.decide(view), Some(action));
            for byte in format!("{action:?}").bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
        assert_eq!((views.len(), hash), (1988, 5_977_450_191_475_415_708));
    }

    #[test]
    fn shared_announce_rule_is_easys_ansage_decision() {
        let mut seen = std::collections::BTreeMap::new();
        for view in announce_views(0..2000) {
            let rule = policy::announce_rule(&view).unwrap();
            assert_eq!(BaselineOpponent.decide(&view), Some(rule.action));
            assert!(!rule.fallback && !rule.plain, "{rule:?}");
            assert_eq!(
                (rule.bid, rule.hand_game),
                (view.bid, view.hand_game == Some(true))
            );
            assert_eq!(rule.skat, view.own_skat.unwrap_or_default());
            assert_eq!(rule.fits, rule.value >= rule.bid);
            let fitting = rule.candidates.iter().filter(|c| c.fits);
            let top = fitting.clone().map(|c| c.strength).max();
            let case = if rule.fits {
                assert_eq!(rule.game, rule.preferred);
                "preferred"
            } else if let Some(top) = top {
                let chosen = rule
                    .candidates
                    .iter()
                    .find(|c| c.game == rule.game)
                    .unwrap();
                assert!(chosen.fits && chosen.strength == top);
                // Tie order: no later fitting candidate has the same strength.
                let later = rule.candidates.iter().skip_while(|c| c.game != rule.game);
                assert_eq!(later.filter(|c| c.fits && c.strength == top).count(), 1);
                if rule.tied().is_empty() {
                    "candidate"
                } else {
                    "tie"
                }
            } else {
                assert!(rule.no_fit);
                assert_eq!(rule.game, GameType::Grand);
                "no fit"
            };
            assert_eq!(rule.ouvert, rule.null_risks == Some(0));
            assert_eq!(rule.null_risks.is_some(), rule.game == GameType::Null);
            let skat_core::Action::Announce(a) = rule.action else {
                panic!("announcement expected");
            };
            assert_eq!((a.game, a.ouvert), (rule.game, rule.ouvert));
            *seen.entry(case).or_insert(0) += 1;
            if rule.hand_game {
                assert!(rule.skat.is_empty());
                *seen.entry("hand").or_insert(0) += 1;
            }
            if rule.game == GameType::Null {
                *seen.entry("null").or_insert(0) += 1;
            }
        }
        // Null is rare in dealt games; core-derived fixtures in skat-coach cover it.
        for case in ["preferred", "candidate", "tie", "no fit", "hand"] {
            assert!(seen.contains_key(case), "{case}: {seen:?}");
        }
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
