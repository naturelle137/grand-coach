// SPDX-License-Identifier: EUPL-1.2

//! Opt-in hybrid opponent; proposal/search computation lives in analysis.

use std::sync::Arc;

use skat_analysis::hybrid::{HybridAnalysis, analyze_hybrid};
use skat_analysis::{Budget, Cancel};
use skat_core::{Action, Phase, PlayerView};
use skat_nn::{ModelError, NeuralModel};

use crate::{Decision, Level, Opponent, Player, kontra_by_rules};

/// Experimental learned proposals with ordinary search verification.
/// Not calibrated and not selected by any shipped level or table.
#[derive(Clone, Debug)]
pub struct HybridOpponent {
    model: Result<Arc<NeuralModel>, ModelError>,
    /// Independent opponent seed, never the deal seed.
    pub seed: u64,
    /// Ordinary per-world search budget (default: Strong's).
    pub budget: Budget,
}

/// An experimental decision with inspectable screening/fallback provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct HybridDecision {
    /// Legal choice. Learned screening never sets a complete Classic status.
    pub decision: Decision,
    /// Shared computation; `None` for rule phases or an invalid view.
    pub analysis: Option<HybridAnalysis>,
}

impl HybridOpponent {
    /// Construct an opt-in opponent; loading/IO belongs to the app or tool.
    pub fn new(model: Result<Arc<NeuralModel>, ModelError>, seed: u64) -> Self {
        Self {
            model,
            seed,
            budget: Budget::INTERACTIVE,
        }
    }

    /// Decide from the seat's permitted view and retain analysis provenance.
    /// Pre-play and Kontra/Re retain Classic Strong behaviour. If no proposed
    /// option finishes verification, use the ordinary rules' legal move.
    pub fn decide_explained(
        &self,
        view: &PlayerView,
        cancel: Option<&Cancel>,
    ) -> Option<HybridDecision> {
        let mut classic = Opponent::new(Level::Strong, self.seed);
        classic.profile.budget = Some(self.budget);
        if view.phase != Phase::Spiel
            || view
                .legal_actions
                .iter()
                .any(|a| !matches!(a, Action::Play(_)))
        {
            return classic.decide(view, cancel).map(|decision| HybridDecision {
                decision,
                analysis: None,
            });
        }
        let (_, seed) = classic.analysis_input(view)?;
        let analysis = match analyze_hybrid(view, self.model.as_deref(), self.budget, seed, cancel)
        {
            Ok(a) => a,
            Err(_) => {
                return classic.decide(view, cancel).map(|decision| HybridDecision {
                    decision,
                    analysis: None,
                });
            }
        };
        let rules = crate::BaselineOpponent.decide(view)?;
        let (selected, status) = match &analysis {
            HybridAnalysis::Classic { analysis, .. } => (
                kontra_by_rules(view, &analysis.options),
                Some(analysis.status),
            ),
            HybridAnalysis::LearnedEstimate(_) => (analysis.recommendation(view), None),
        };
        Some(HybridDecision {
            decision: Decision {
                action: selected.unwrap_or(rules),
                status,
                fallback: selected.is_none(),
            },
            analysis: Some(analysis),
        })
    }
}

impl Player for HybridOpponent {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        self.decide_explained(view, cancel).map(|d| d.decision)
    }
}
