// SPDX-License-Identifier: EUPL-1.2

//! The Mephisto opponent (D-41): decides with every card known.
//!
//! It receives the whole [`Game`], so its use is visible in every
//! signature (D-23). Each decision is [`skat_analysis::analyze_known`] on
//! the real distribution: every option is played out exactly, card play
//! double-dummy, and the best option by [`crate::utility`] is chosen. Two
//! Mephisto defenders solve the same double-dummy game, in which the
//! defenders minimise together, so they play as one side.
//!
//! Bidding: the analysis values a bid with the other seats bidding by the
//! rules of thumb. Mephisto only says or holds a value if it also wins the
//! game when everybody else passes at once (checked exactly, with the
//! game it would then choose); passing is always allowed. As declarer it
//! therefore only plays games it wins double-dummy, against any defence.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use skat_analysis::sampling::World;
use skat_analysis::{Analysis, Budget, Cancel, Status, analyze_known};
use skat_core::{Action, Game, Phase, Seat};

use crate::opponent::{decision, select};
use crate::{BaselineOpponent, Decision};

/// An opponent that sees every card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mephisto;

impl Mephisto {
    /// Node budget per task: one option before card play, every option in
    /// card play (`docs/ai.md`).
    pub const BUDGET: Budget = Budget {
        samples: 1,
        nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
    };

    /// Chooses one of the legal actions of the seat to move in `game`;
    /// `None` if the game is over. Without an estimate (cancelled,
    /// exhausted budget) the rules of thumb decide.
    pub fn decide(&self, game: &Game, cancel: Option<&Cancel>) -> Option<Decision> {
        let seat = game.current_seat()?;
        let view = game.view(seat);
        let rules = BaselineOpponent.decide(&view)?;
        let world = World::from_omniscient(&game.omniscient_view());
        let Ok(mut analysis) = analyze_known(&view, world, Self::BUDGET, cancel) else {
            return Some(Decision {
                action: rules,
                status: None,
                fallback: true,
            });
        };
        if view.phase == Phase::Reizen {
            keep_safe_bids(game, seat, &mut analysis, cancel);
        }
        let rules = if allowed(&analysis, rules) {
            rules
        } else {
            Action::Pass
        };
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        Some(decision(
            select(&view, &analysis.options, rules, 0.0, &mut rng),
            rules,
            analysis.status,
        ))
    }
}

fn allowed(analysis: &Analysis, action: Action) -> bool {
    analysis
        .options
        .iter()
        .any(|o| o.action == action && o.estimate.is_some())
}

/// Removes the estimates of bids and holds that would lose if everybody
/// else passed at once. Safe bids form a prefix of the bidding values (a
/// game that covers a value covers every lower one), so a binary search
/// suffices.
fn keep_safe_bids(game: &Game, seat: Seat, analysis: &mut Analysis, cancel: Option<&Cancel>) {
    let bids: Vec<usize> = (0..analysis.options.len())
        .filter(|&i| matches!(analysis.options[i].action, Action::Bid(_)))
        .collect();
    let (mut low, mut high) = (0, bids.len());
    while low < high {
        let mid = (low + high) / 2;
        if wins_alone(game, seat, analysis.options[bids[mid]].action, cancel) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    for &i in &bids[low..] {
        analysis.options[i].estimate = None;
    }
    for option in &mut analysis.options {
        if option.action == Action::Hold && !wins_alone(game, seat, Action::Hold, cancel) {
            option.estimate = None;
        }
    }
}

/// Whether `seat` wins after `action` if every other seat passes at once.
fn wins_alone(game: &Game, seat: Seat, action: Action, cancel: Option<&Cancel>) -> bool {
    let mut g = game.clone();
    if g.apply(action).is_err() {
        return false;
    }
    while g.phase() == Phase::Reizen {
        match g.current_seat() {
            Some(s) if s != seat && g.apply(Action::Pass).is_ok() => {}
            _ => return false,
        }
    }
    if g.phase() != Phase::SkatAufnahme || g.declarer() != Some(seat) {
        return false;
    }
    let view = g.view(seat);
    let world = World::from_omniscient(&g.omniscient_view());
    match analyze_known(&view, world, Mephisto::BUDGET, cancel) {
        Ok(a) if a.status == Status::Complete => a
            .options
            .iter()
            .any(|o| o.estimate.is_some_and(|e| e.win_chance >= 1.0)),
        _ => false,
    }
}
