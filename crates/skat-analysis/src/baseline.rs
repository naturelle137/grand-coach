// SPDX-License-Identifier: EUPL-1.2

//! Easy's frozen deterministic card policy, shared with the guided coach.
//! Candidate order is the core legal order. Rust min keeps the first tie;
//! max keeps the last. A follow-class master does not exclude trumping.

use crate::hand::power;
use crate::{Knowledge, TrickState, policy};
use skat_core::{Action, Card, GameType, PlayerView};

/// The actual selected policy branch, not an inferred search explanation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Branch {
    /// Null policy; outside the guided slice.
    Null,
    /// Declarer draws outstanding Trumpf.
    DrawTrumpf,
    /// Lead a master of its follow class; trumping remains possible.
    LeadMaster,
    /// Put Augen into a partner's lead (last seat or follow-class master).
    FeedPartner,
    /// Partner leads, not last, and partner's card is not a follow-class master.
    UnderPartner,
    /// Take the trick cheaply in last seat.
    WinLast,
    /// Early winner is Trumpf or a follow-class master.
    WinEarly,
    /// Lowest losing card or arbitrary lowest-card fallback.
    Fallback,
}

/// Exact ordering, including Rust iterator tie behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TieBreak {
    /// Minimum (is_trump, Augen, power), first exact tie.
    LowestCostFirst,
    /// Minimum (is_trump, power), first exact tie.
    LowestWinnerFirst,
    /// Maximum power, last exact tie.
    HighestPowerLast,
    /// Maximum Augen, last exact tie.
    HighestAugenLast,
    /// Maximum (!is_trump, Augen), last exact tie.
    FeedLast,
}

/// Exact policy inputs and outcome for one permitted decision.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardRule {
    /// The actual selected card.
    pub card: Card,
    /// Selected branch.
    pub branch: Branch,
    /// Legal cards that currently overtake the table.
    pub winners: Vec<Card>,
    /// Legal cards that leave the current winner ahead.
    pub losers: Vec<Card>,
    /// Candidates considered by this branch, in legal order.
    pub candidates: Vec<Card>,
    /// Exact current winner from public trick cards.
    pub winner: Option<skat_core::Seat>,
    /// Whether the partner currently leads.
    pub partner_ahead: bool,
    /// Whether the player is last to act in this trick.
    pub last: bool,
    /// Whether the current winning card is a master of its follow class only.
    pub table_master: bool,
    /// Actual ordering and exact-tie behavior used on the candidates.
    pub tie_break: TieBreak,
    /// Selected card's ordering key, padded with zeroes.
    pub tie_key: (i32, i32, i32),
}

/// Returns no rule for non-card decisions, including Kontra/Re.
pub fn rule(view: &PlayerView) -> Option<CardRule> {
    let game = view.contract?.game;
    let cards: Vec<_> = view
        .legal_actions
        .iter()
        .filter_map(|a| {
            if let Action::Play(c) = a {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    if cards.is_empty()
        || view
            .legal_actions
            .iter()
            .any(|a| !matches!(a, Action::Play(_)))
    {
        return None;
    }
    Some(card_rule(view, game, &cards))
}

fn card_rule(view: &PlayerView, game: GameType, cards: &[Card]) -> CardRule {
    let k = Knowledge::from_view(view);
    let trick = TrickState::of(game, &view.current_trick);
    let (card, branch) = choose_card(view, game, cards);
    let winners: Vec<_> = cards
        .iter()
        .copied()
        .filter(|&c| trick.beaten_by(game, c))
        .collect();
    let losers: Vec<_> = cards
        .iter()
        .copied()
        .filter(|&c| !trick.beaten_by(game, c))
        .collect();
    let null_loser = branch == Branch::Null
        && trick.led.is_some()
        && (view.declarer == Some(view.seat) || trick.best_seat == view.declarer)
        && !losers.is_empty();
    let candidates = match branch {
        Branch::UnderPartner | Branch::Fallback if !losers.is_empty() => losers.clone(),
        Branch::Null if null_loser => losers.clone(),
        Branch::WinLast | Branch::WinEarly => winners.clone(),
        Branch::DrawTrumpf => cards
            .iter()
            .copied()
            .filter(|&c| game.is_trump(c))
            .collect(),
        Branch::LeadMaster => cards
            .iter()
            .copied()
            .filter(|&c| !game.is_trump(c) && k.is_master(c))
            .collect(),
        _ => cards.to_vec(),
    };
    let tie_break = match branch {
        Branch::WinLast | Branch::WinEarly => TieBreak::LowestWinnerFirst,
        Branch::DrawTrumpf => TieBreak::HighestPowerLast,
        Branch::LeadMaster => TieBreak::HighestAugenLast,
        Branch::FeedPartner => TieBreak::FeedLast,
        Branch::Null if null_loser => TieBreak::HighestPowerLast,
        _ => TieBreak::LowestCostFirst,
    };
    let tie_key = match tie_break {
        TieBreak::LowestCostFirst => (
            i32::from(game.is_trump(card)),
            i32::from(card.augen()),
            power(game, card),
        ),
        TieBreak::LowestWinnerFirst => (i32::from(game.is_trump(card)), power(game, card), 0),
        TieBreak::HighestPowerLast => (power(game, card), 0, 0),
        TieBreak::HighestAugenLast => (i32::from(card.augen()), 0, 0),
        TieBreak::FeedLast => (i32::from(!game.is_trump(card)), i32::from(card.augen()), 0),
    };
    CardRule {
        card,
        branch,
        candidates,
        winners,
        losers,
        winner: trick.best_seat,
        partner_ahead: view.declarer != Some(view.seat)
            && trick.best_seat.is_some_and(|s| Some(s) != view.declarer),
        last: view.current_trick.len() == 2,
        table_master: trick.best.is_some_and(|c| k.is_master(c)),
        tie_break,
        tie_key,
    }
}

/// Legacy Easy play choice, including Kontra/Re priority.
pub fn play(view: &PlayerView) -> Action {
    let legal = &view.legal_actions;
    if legal.contains(&Action::KeinKontra) {
        return Action::KeinKontra;
    }
    let game = view.contract.map_or(GameType::Grand, |c| c.game);
    if legal.contains(&Action::Re) {
        let cards = view.hand | view.own_skat.unwrap_or_default();
        return if policy::wants_re(game, cards) {
            Action::Re
        } else {
            Action::KeinRe
        };
    }
    if legal.contains(&Action::Kontra) && policy::wants_kontra(game, view.hand) {
        return Action::Kontra;
    }
    let cards: Vec<Card> = legal
        .iter()
        .filter_map(|a| match a {
            Action::Play(card) => Some(*card),
            _ => None,
        })
        .collect();
    Action::Play(card_rule(view, game, &cards).card)
}

fn lowest(game: GameType, cards: impl Iterator<Item = Card>) -> Option<Card> {
    cards.min_by_key(|&c| (game.is_trump(c), c.augen(), power(game, c)))
}

fn choose_card(view: &PlayerView, game: GameType, cards: &[Card]) -> (Card, Branch) {
    let k = Knowledge::from_view(view);
    let trick = TrickState::of(game, &view.current_trick);
    let declarer = view.declarer.expect("declarer during play");
    let me_declarer = view.seat == declarer;
    let all = cards.iter().copied();
    let winners: Vec<Card> = all.clone().filter(|&c| trick.beaten_by(game, c)).collect();
    let losers: Vec<Card> = all.clone().filter(|&c| !trick.beaten_by(game, c)).collect();
    let last = view.current_trick.len() == 2;
    let fallback = lowest(game, all.clone()).expect("at least one legal card");

    if game == GameType::Null {
        let highest_loser = losers.iter().copied().max_by_key(|&c| power(game, c));
        return (
            match (trick.led, me_declarer) {
                (None, _) => fallback,
                (Some(_), true) => highest_loser.unwrap_or(fallback),
                (Some(_), false) if trick.best_seat == Some(declarer) => {
                    highest_loser.unwrap_or(fallback)
                }
                (Some(_), false) => fallback,
            },
            Branch::Null,
        );
    }

    if trick.led.is_none() {
        let trumps: Vec<Card> = all.clone().filter(|&c| game.is_trump(c)).collect();
        if me_declarer && !trumps.is_empty() && !k.trumps_out().is_empty() {
            return (
                *trumps.iter().max_by_key(|&&c| power(game, c)).unwrap(),
                Branch::DrawTrumpf,
            );
        }
        let master = all
            .clone()
            .filter(|&c| !game.is_trump(c) && k.is_master(c))
            .max_by_key(|&c| c.augen());
        return (
            master.unwrap_or(fallback),
            if master.is_some() {
                Branch::LeadMaster
            } else {
                Branch::Fallback
            },
        );
    }

    let partner_wins = !me_declarer && trick.best_seat.is_some_and(|s| s != declarer);
    if partner_wins {
        let safe = last || trick.best.is_some_and(|b| k.is_master(b));
        if safe {
            return (
                all.clone()
                    .max_by_key(|&c| (!game.is_trump(c), c.augen()))
                    .unwrap_or(fallback),
                Branch::FeedPartner,
            );
        }
        let under = lowest(game, losers.into_iter());
        return (
            under.unwrap_or(fallback),
            if under.is_some() {
                Branch::UnderPartner
            } else {
                Branch::Fallback
            },
        );
    }
    let cheapest_winner = winners
        .iter()
        .copied()
        .min_by_key(|&c| (game.is_trump(c), power(game, c)));
    match cheapest_winner {
        Some(card) if last || k.is_master(card) || game.is_trump(card) => (
            card,
            if last {
                Branch::WinLast
            } else {
                Branch::WinEarly
            },
        ),
        _ => (
            lowest(game, losers.into_iter()).unwrap_or(fallback),
            Branch::Fallback,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{Announcement, Game, Phase, RuleConfig, Seat, Suit};

    #[test]
    fn trace_candidates_keys_and_iterator_ties_match_actual_choices() {
        let mut ties = 0;
        let mut fallbacks = 0;
        for game_type in [GameType::Farbe(Suit::Herz), GameType::Grand, GameType::Null] {
            for seed in 0..100 {
                let mut g = Game::new(seed, RuleConfig::default());
                while g.phase() != Phase::Spiel {
                    let a = match g.phase() {
                        Phase::Reizen if g.current_seat() == Some(Seat::Vorhand) => {
                            if g.legal_actions().contains(&Action::Bid(18)) {
                                Action::Bid(18)
                            } else {
                                Action::Hold
                            }
                        }
                        Phase::Reizen => Action::Pass,
                        Phase::SkatAufnahme => Action::Hand,
                        Phase::Ansage => Action::Announce(Announcement::plain(game_type)),
                        _ => panic!("phase"),
                    };
                    g.apply(a).unwrap();
                }
                let mut n = 0;
                while let Some(seat) = g.current_seat() {
                    let view = g.view(seat);
                    let r = rule(&view).unwrap();
                    assert_eq!(play(&view), Action::Play(r.card));
                    let key = |c: Card| match r.tie_break {
                        TieBreak::LowestCostFirst => (
                            i32::from(game_type.is_trump(c)),
                            i32::from(c.augen()),
                            power(game_type, c),
                        ),
                        TieBreak::LowestWinnerFirst => {
                            (i32::from(game_type.is_trump(c)), power(game_type, c), 0)
                        }
                        TieBreak::HighestPowerLast => (power(game_type, c), 0, 0),
                        TieBreak::HighestAugenLast => (i32::from(c.augen()), 0, 0),
                        TieBreak::FeedLast => {
                            (i32::from(!game_type.is_trump(c)), i32::from(c.augen()), 0)
                        }
                    };
                    let expected = match r.tie_break {
                        TieBreak::LowestCostFirst | TieBreak::LowestWinnerFirst => {
                            r.candidates.iter().copied().min_by_key(|&c| key(c))
                        }
                        _ => r.candidates.iter().copied().max_by_key(|&c| key(c)),
                    };
                    assert_eq!(expected, Some(r.card), "{r:?}");
                    assert_eq!(r.tie_key, key(r.card));
                    if r.candidates
                        .iter()
                        .filter(|&&c| key(c) == r.tie_key)
                        .count()
                        > 1
                    {
                        ties += 1;
                    }
                    if r.branch == Branch::Fallback {
                        fallbacks += 1;
                    }
                    // Alternative legal actions expose additional branch boundaries.
                    let action = if n % 3 == 0 {
                        view.legal_actions[n % view.legal_actions.len()]
                    } else {
                        Action::Play(r.card)
                    };
                    g.apply(action).unwrap();
                    n += 1;
                }
            }
        }
        assert!(ties > 0 && fallbacks > 0);
    }
}
