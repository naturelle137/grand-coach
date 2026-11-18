// SPDX-License-Identifier: EUPL-1.2

//! Experimental learned proposals verified by ordinary analysis (#76).
//! The network sees only the original permitted view. Its two highest
//! policy proposals plus the shared rule card are searched at the given
//! ordinary budget. Unsearched cards have no verification, never a losing
//! value. Exact distributions and decisions outside pure card play retain
//! Classic. No shipped engine calls this path by default.

use skat_core::{Action, Phase, PlayerView};
use skat_nn::{Fallback, ModelError, NeuralModel};

use crate::sampling::{Diagnostic, HiddenCards};
use crate::{Analysis, Budget, Cancel, Effort, Estimate, Status, analyze, baseline, utility};

/// Why the ordinary all-option analysis ran instead of proposal screening.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HybridFallback {
    /// Not a pure card-play decision (including Kontra/Re or no decision).
    NotCardPlay,
    /// At most three cards: screening cannot save an option.
    FewOptions,
    /// All consistent hidden distributions fit in the sample budget.
    Exact,
    /// Cancelled before inference.
    Cancelled,
    /// Model loading or inference failed.
    Model(Fallback),
}

/// A prior and a separately computed, ordinary Monte-Carlo verification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HybridOption {
    /// A legal action, in the original view's order.
    pub action: Action,
    /// Learned policy probability; no sample count or reliability claim.
    pub prior: f32,
    /// Selected for verification (even if no distribution could finish).
    pub proposed: bool,
    /// Ordinary search estimate, absent for unsearched/interrupted cards.
    pub verification: Option<Estimate>,
}

/// A learned candidate set, never a complete ranking of the legal moves.
/// No conversion to `Analysis` is supplied: existing reliability predicates
/// must not accept a ranking that omitted legal alternatives.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LearnedProposals {
    /// SHA-256 of the actual loaded bytes.
    pub model_sha256: [u8; 32],
    /// Encoder from the loaded header, not a caller-supplied label.
    pub encoder_version: u16,
    /// Learned seat value; never an exact or Monte-Carlo estimate.
    pub value: f32,
    /// Every original legal action, including unsearched alternatives.
    pub options: Vec<HybridOption>,
    /// Completion of proposal verification only, not of all legal options.
    pub search_status: Status,
    /// Work of ordinary proposal verification only.
    pub effort: Effort,
}

/// Distinct provenance for learned screening versus unchanged Classic.
///
/// A learned result cannot enter the existing reliable-comparison API:
/// ```compile_fail
/// use skat_analysis::{hybrid::HybridAnalysis, lesson, Analysis};
/// use skat_core::{Action, PlayerView};
/// fn compare(view: &PlayerView, hybrid: &HybridAnalysis, action: Action) {
///     lesson::compare(view, hybrid, action);
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HybridAnalysis {
    /// Ordinary all-option analysis, unchanged, with its bypass reason.
    Classic {
        /// The original analysis, with ordinary reliability semantics.
        analysis: Box<Analysis>,
        /// Why learned screening did not run.
        reason: HybridFallback,
    },
    /// Learned screening; never evidence that an omitted option is worse.
    LearnedEstimate(Box<LearnedProposals>),
}

impl HybridAnalysis {
    /// Highest-utility verified proposal (or ordinary Classic option).
    /// Ties keep original legal order; only the most-sampled estimates
    /// compete. `None` means no search verification finished. This is an
    /// experimental action recommendation, not a reliable comparison.
    pub fn recommendation(&self, view: &PlayerView) -> Option<Action> {
        let options: Vec<_> = match self {
            Self::Classic { analysis, .. } => analysis
                .options
                .iter()
                .map(|o| (o.action, o.estimate))
                .collect(),
            Self::LearnedEstimate(p) => p
                .options
                .iter()
                .map(|o| (o.action, o.verification))
                .collect(),
        };
        let most = options
            .iter()
            .filter_map(|(_, e)| e.map(|e| e.samples))
            .max()?;
        let mut best: Option<(Action, f64)> = None;
        for (action, estimate) in options {
            if let Some(e) = estimate
                && e.samples == most
            {
                let u = utility(view, &e);
                if best.is_none_or(|(_, previous)| u > previous) {
                    best = Some((action, u));
                }
            }
        }
        best.map(|(action, _)| action)
    }
}

/// Verify learned proposals with the existing sampled-world solver.
/// Model bytes/loading belong to the caller; this module performs no IO.
/// The same view/model/budget/seed gives the same result, excluding external
/// cancellation timing. Budgets keep their ordinary per-world semantics.
///
/// # Errors
/// As [`analyze`]: inconsistent permitted view or invalid public history.
pub fn analyze_hybrid(
    view: &PlayerView,
    model: Result<&NeuralModel, &ModelError>,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
) -> Result<HybridAnalysis, Vec<Diagnostic>> {
    let classic = |reason| {
        analyze(view, budget, seed, cancel).map(|analysis| HybridAnalysis::Classic {
            analysis: Box::new(analysis),
            reason,
        })
    };
    if view.phase != Phase::Spiel
        || view.legal_actions.is_empty()
        || view
            .legal_actions
            .iter()
            .any(|a| !matches!(a, Action::Play(_)))
    {
        return classic(HybridFallback::NotCardPlay);
    }
    if cancel.is_some_and(Cancel::is_cancelled) {
        return classic(HybridFallback::Cancelled);
    }
    if HiddenCards::from_view(view)?.count() <= u128::from(budget.samples.max(1)) {
        return classic(HybridFallback::Exact);
    }
    if view.legal_actions.len() <= 3 {
        return classic(HybridFallback::FewOptions);
    }
    let model = match model {
        Ok(model) => model,
        Err(e) => return classic(HybridFallback::Model(Fallback::Model(e.clone()))),
    };
    let evaluation = match model.evaluate(view) {
        Ok(e) => e,
        Err(e) => return classic(HybridFallback::Model(e)),
    };
    let mut ranked: Vec<usize> = (0..evaluation.policy.len()).collect();
    // Stable sort preserves the first original legal option on equal priors.
    ranked.sort_by(|&a, &b| evaluation.policy[b].1.total_cmp(&evaluation.policy[a].1));
    let rule = baseline::play(view);
    let proposed = |i: usize| ranked[..2].contains(&i) || view.legal_actions[i] == rule;
    let restricted = PlayerView {
        legal_actions: view
            .legal_actions
            .iter()
            .enumerate()
            .filter_map(|(i, &a)| proposed(i).then_some(a))
            .collect(),
        ..view.clone()
    };
    let verified = analyze(&restricted, budget, seed, cancel)?;
    let options = evaluation
        .policy
        .iter()
        .enumerate()
        .map(|(i, &(action, prior))| HybridOption {
            action,
            prior,
            proposed: proposed(i),
            verification: verified
                .options
                .iter()
                .find(|o| o.action == action)
                .and_then(|o| o.estimate),
        })
        .collect();
    Ok(HybridAnalysis::LearnedEstimate(Box::new(
        LearnedProposals {
            model_sha256: model.sha256(),
            encoder_version: model.header().encoder_version,
            value: evaluation.value,
            options,
            search_status: verified.status,
            effort: verified.effort,
        },
    )))
}
