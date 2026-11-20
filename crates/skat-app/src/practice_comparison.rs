// SPDX-License-Identifier: EUPL-1.2

//! Read-only comparison of a finished same-deal practice attempt with the
//! original recorded game (D-66). It lives only in memory: nothing is scored,
//! ranked, saved or analysed here. Each own decision of both lines is shown
//! from the human's permitted view at that time, so it discloses no concealed
//! card and makes no claim about which line was better.

use skat_core::Game;

use crate::coach::DecisionKey;
use crate::context::DecisionContext;
use crate::recording::{GameRecording, RecordingError};
use crate::session::PlayerId;

/// One entry of the comparison, in decision order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonRow {
    /// A run of own decisions with the same position and the same action in
    /// both lines, collapsed.
    Identical {
        /// Number of collapsed decisions.
        count: u32,
    },
    /// The n-th own decision differs (another action, another position, or
    /// only one line had it). Each side shows its own permitted snapshot.
    Diverged {
        /// One-based ordinal of the own decision.
        ordinal: u32,
        /// The original game's decision, if it had one.
        original: Option<Box<DecisionContext>>,
        /// The attempt's decision, if it had one.
        attempt: Option<Box<DecisionContext>>,
    },
}

/// The comparison of an attempt with its original.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PracticeComparison {
    /// Entries in decision order.
    pub rows: Vec<ComparisonRow>,
}

/// An own decision: actions applied before it and its snapshot.
fn decisions(r: &GameRecording) -> Result<Vec<(usize, DecisionContext)>, RecordingError> {
    r.reconstruct_privileged(r.actions.len())?;
    let dealer = r.context.dealer;
    let mut game = Game::from_deal(r.deal, r.rules);
    let mut out = Vec::new();
    for (prefix, &action) in r.actions.iter().enumerate() {
        let seat = game.current_seat().expect("entire history validated");
        if PlayerId::on_seat(seat, dealer).is_human() {
            let key = DecisionKey {
                lineage: 0,
                number: r.context.number,
                actions: prefix,
            };
            out.push((
                prefix,
                DecisionContext::new(key, dealer, &game.view(seat), action),
            ));
        }
        game.apply(action).expect("entire history validated");
    }
    Ok(out)
}

impl PracticeComparison {
    /// Compares `attempt` with `original`, both fully validated first. The
    /// i-th own decisions are paired; a pair is identical only if the whole
    /// action history before it and the action agree.
    pub fn new(
        original: &GameRecording,
        attempt: &GameRecording,
    ) -> Result<PracticeComparison, RecordingError> {
        let (a, b) = (decisions(original)?, decisions(attempt)?);
        let mut rows = Vec::new();
        for n in 0..a.len().max(b.len()) {
            let (x, y) = (a.get(n), b.get(n));
            let same = match (x, y) {
                (Some((p, cx)), Some((q, cy))) => {
                    p == q
                        && cx.action == cy.action
                        && original.actions[..*p] == attempt.actions[..*q]
                }
                _ => false,
            };
            match (same, rows.last_mut()) {
                (true, Some(ComparisonRow::Identical { count })) => *count += 1,
                (true, _) => rows.push(ComparisonRow::Identical { count: 1 }),
                (false, _) => rows.push(ComparisonRow::Diverged {
                    ordinal: n as u32 + 1,
                    original: x.map(|(_, c)| Box::new(c.clone())),
                    attempt: y.map(|(_, c)| Box::new(c.clone())),
                }),
            }
        }
        Ok(PracticeComparison { rows })
    }

    /// Number of diverging decisions.
    pub fn diverged(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| matches!(r, ComparisonRow::Diverged { .. }))
            .count()
    }
}
