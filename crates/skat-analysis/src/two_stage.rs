// SPDX-License-Identifier: EUPL-1.2

//! Experimental two-stage sampling for card play (#104), opt-in only.
//!
//! [`analyze_two_stage`] evaluates every legal card on the first
//! [`TwoStage::screen_worlds`] distributions of the ordinary single-stage
//! analysis (the same worlds, in the same order), keeps the options whose
//! utility is close to the best one and evaluates only those on the
//! remaining distributions. Surviving options therefore get exactly the
//! values the single-stage analysis gives them. Shipped opponents, the
//! coach and the training-data generator keep calling the single-stage
//! [`analyze`](crate::analyze) / [`analyze_experimental`](crate::analyze_experimental),
//! which stay the reference for paired comparisons ([`agreement`]).
//!
//! The screening parameters ([`TwoStage::PROPOSAL`]) are provisional until
//! measured; see `docs/analysis.md` ("Two-stage card play").

use skat_core::{Action, Phase, PlayerView};

use crate::analysis::{Analysis, Worlds, analyze_inner, utility};
use crate::budget::{Budget, Cancel};
use crate::sampling::Diagnostic;

/// Screening parameters of the two-stage card-play analysis.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TwoStage {
    /// Distributions every legal option is evaluated on in stage 1: the
    /// first ones of the single-stage analysis. A budget with no more
    /// samples than this skips the screening.
    pub screen_worlds: u32,
    /// An option survives if its utility is at most this many standard
    /// errors below the best option's.
    pub se_multiple: f64,
    /// Lower bound of that standard error in points (double-dummy only
    /// resolves scoring classes, so few distributions can agree by chance).
    pub se_floor: f64,
    /// Fewest survivors (filled up in rank order).
    pub min_keep: u32,
    /// Most survivors (cut in rank order).
    pub max_keep: u32,
}

impl TwoStage {
    /// The proposal of #104: 8 screening distributions, 2.5 standard
    /// errors, a 2-point floor, 2 to 4 survivors. Provisional; tune on the
    /// tuning suite before any default changes.
    pub const PROPOSAL: TwoStage = TwoStage {
        screen_worlds: 8,
        se_multiple: 2.5,
        se_floor: 2.0,
        min_keep: 2,
        max_keep: 4,
    };

    /// The parameters with nonsensical values clamped: at least one
    /// screening distribution and survivor, `max_keep >= min_keep`,
    /// non-negative finite multiple and floor (otherwise 0).
    pub fn normalized(self) -> TwoStage {
        let non_negative = |x: f64| if x.is_finite() && x > 0.0 { x } else { 0.0 };
        let min_keep = self.min_keep.max(1);
        TwoStage {
            screen_worlds: self.screen_worlds.max(1),
            se_multiple: non_negative(self.se_multiple),
            se_floor: non_negative(self.se_floor),
            min_keep,
            max_keep: self.max_keep.max(min_keep),
        }
    }
}

impl Default for TwoStage {
    fn default() -> TwoStage {
        TwoStage::PROPOSAL
    }
}

/// Search kernel of both stages, matching the single-stage reference it
/// is compared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kernel {
    /// Like [`analyze`](crate::analyze) (Strong, Medium, coach).
    Classic,
    /// Like [`analyze_experimental`](crate::analyze_experimental) without
    /// preview (Experimental opponent, teacher).
    Experimental,
}

/// Result of [`analyze_two_stage`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TwoStageAnalysis {
    /// The analysis. Screened-out options keep their stage-1 estimate
    /// (fewer `samples`); status and effort follow the single-stage rules.
    pub analysis: Analysis,
    /// What the screening did.
    pub report: StageReport,
}

/// What the screening did in one analysis.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StageReport {
    /// Whether stage 2 was restricted, and if not, why.
    pub outcome: StageOutcome,
    /// Distributions evaluated for every option in stage 1 (0 if the
    /// screening was skipped before stage 1).
    pub screen_worlds: u32,
    /// Indices of the options evaluated on the remaining distributions,
    /// ascending; every option if nothing was screened out.
    pub survivors: Vec<usize>,
    /// Per option, in the order of `Analysis::options`.
    pub options: Vec<OptionStage>,
    /// Search nodes used by stage 1 (all nodes if skipped before stage 1).
    pub screen_nodes: u64,
}

/// Screening result of one option.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionStage {
    /// Evaluated on the remaining distributions.
    pub kept: bool,
    /// Effective sample size of the evidence weights over the finished
    /// distributions this option was evaluated on (0 if none).
    pub effective_samples: f64,
}

/// Whether stage 2 was restricted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StageOutcome {
    /// Stage 2 evaluated only the survivors.
    Screened,
    /// A single survivor (only with `min_keep == 1`): no stage 2.
    OneSurvivor,
    /// Every option survived: stage 2 equals the single-stage analysis.
    AllSurvived,
    /// No screening; the analysis equals the single-stage reference.
    Skipped(SkipReason),
}

/// Why no screening took place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SkipReason {
    /// Not this seat's turn.
    NoDecision,
    /// Not card play (bidding, Drücken and other pre-play decisions).
    NotCardPlay,
    /// The decision offers Kontra or Re (excluded from screening).
    KontraRe,
    /// All distributions were enumerated: the exact result is kept.
    Exact,
    /// At most `max(2, min_keep)` legal options: all would survive.
    FewOptions,
    /// No more planned distributions than `screen_worlds`.
    SmallBudget,
    /// A stage-1 task was cancelled or ran out of nodes; the remaining
    /// distributions were evaluated for every option.
    Interrupted,
    /// The stage-1 evidence weights have an effective sample size below 2,
    /// too few to estimate a standard error.
    LowEffectiveSamples,
}

/// Fewest effective stage-1 samples for a standard error.
const MIN_SCREEN_ESS: f64 = 2.0;

/// Two-stage evaluation of a card-play decision (see the module docs).
/// Same view, seed, budget, kernel and parameters give the same result,
/// independent of the thread count. Every decision that is not plain card
/// play with a sampled information set is evaluated exactly like the
/// single-stage reference, with the reason in the report.
pub fn analyze_two_stage(
    view: &PlayerView,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
    kernel: Kernel,
    screening: TwoStage,
) -> Result<TwoStageAnalysis, Vec<Diagnostic>> {
    let source = match kernel {
        Kernel::Classic => Worlds::Sampled,
        Kernel::Experimental => Worlds::Experimental(None),
    };
    let (analysis, report) = analyze_inner(view, budget, seed, cancel, source, Some(screening))?;
    Ok(TwoStageAnalysis {
        analysis,
        report: report.expect("requested screening always reports"),
    })
}

/// Why `view` cannot be screened before any distribution is evaluated, or
/// `None` if stage 1 may run.
pub(crate) fn skip_reason(
    view: &PlayerView,
    enumerated: bool,
    worlds: usize,
    screening: &TwoStage,
) -> Option<SkipReason> {
    let n = view.legal_actions.len();
    Some(if n == 0 {
        SkipReason::NoDecision
    } else if view.phase != Phase::Spiel {
        SkipReason::NotCardPlay
    } else if !view
        .legal_actions
        .iter()
        .all(|a| matches!(a, Action::Play(_)))
    {
        SkipReason::KontraRe
    } else if enumerated {
        SkipReason::Exact
    } else if n <= screening.min_keep.max(2) as usize {
        SkipReason::FewOptions
    } else if worlds <= screening.screen_worlds as usize {
        SkipReason::SmallBudget
    } else {
        return None;
    })
}

/// Survivors of the screening, ascending, from the utilities of every
/// option (`utilities[world][option]`) on the finished stage-1
/// distributions with positive evidence `weights`; `None` if their
/// effective sample size is below 2.
///
/// The best option has the highest weighted mean utility (ties: lowest
/// index). Another option survives if its mean is at most `se_multiple`
/// standard errors below, where the standard error is that of the paired
/// per-distribution difference to the best (both use the same worlds),
/// raised to `se_floor`. Survivors are then filled up to `min_keep` and
/// cut to `max_keep` in rank order (mean descending, index ascending), so
/// ties are resolved by option order.
pub(crate) fn survivors(
    weights: &[f64],
    utilities: &[Vec<f64>],
    options: usize,
    screening: &TwoStage,
) -> Option<Vec<usize>> {
    let total: f64 = weights.iter().sum();
    let squares: f64 = weights.iter().map(|w| w * w).sum();
    if total <= 0.0 || options == 0 {
        return None;
    }
    let ess = total * total / squares;
    if ess < MIN_SCREEN_ESS {
        return None;
    }
    let mean = |f: &dyn Fn(&[f64]) -> f64| {
        weights
            .iter()
            .zip(utilities)
            .map(|(w, u)| w * f(u))
            .sum::<f64>()
            / total
    };
    let means: Vec<f64> = (0..options).map(|o| mean(&|u| u[o])).collect();
    let mut ranked: Vec<usize> = (0..options).collect();
    ranked.sort_by(|&a, &b| means[b].total_cmp(&means[a]).then(a.cmp(&b)));
    let best = ranked[0];
    let close = |o: usize| {
        if o == best {
            return true;
        }
        let gap = means[best] - means[o];
        let variance = mean(&|u| (u[best] - u[o] - gap).powi(2));
        let se = (variance / ess).sqrt().max(screening.se_floor);
        gap <= screening.se_multiple * se
    };
    let mut keep: Vec<usize> = ranked.iter().copied().filter(|&o| close(o)).collect();
    for &o in &ranked {
        if keep.len() >= screening.min_keep as usize {
            break;
        }
        if !keep.contains(&o) {
            keep.push(o);
        }
    }
    // Re-rank the filled-up set before the cut.
    keep.sort_by_key(|o| ranked.iter().position(|r| r == o));
    keep.truncate(screening.max_keep as usize);
    keep.sort_unstable();
    Some(keep)
}

/// The option a seat would choose from `analysis`: highest
/// [`utility`] among the options evaluated on the most distributions
/// (as the analysed opponents do after screening); ties: lowest index.
pub fn best_option(view: &PlayerView, analysis: &Analysis) -> Option<usize> {
    let most = analysis
        .options
        .iter()
        .filter_map(|o| o.estimate.map(|e| e.samples))
        .max()?;
    let mut best: Option<(usize, f64)> = None;
    for (i, o) in analysis.options.iter().enumerate() {
        if let Some(e) = o.estimate
            && e.samples == most
        {
            let u = utility(view, &e);
            if best.is_none_or(|(_, b)| u > b) {
                best = Some((i, u));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Paired comparison of a candidate analysis (e.g. two-stage) with the
/// single-stage reference of the same view, seed and budget.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Agreement {
    /// [`best_option`] of the reference.
    pub reference: Option<usize>,
    /// [`best_option`] of the candidate.
    pub candidate: Option<usize>,
    /// Utility loss, valued by the reference, of choosing the candidate's
    /// option instead of the reference's (0 if they agree); `None` if
    /// either has no choice or the reference has no estimate for it.
    pub regret: Option<f64>,
}

impl Agreement {
    /// Both chose the same option.
    pub fn identical(&self) -> bool {
        self.reference.is_some() && self.reference == self.candidate
    }
}

/// Compares the choices of `reference` and `candidate` for `view`.
pub fn agreement(view: &PlayerView, reference: &Analysis, candidate: &Analysis) -> Agreement {
    let r = best_option(view, reference);
    let c = best_option(view, candidate);
    let value = |i: usize| {
        reference
            .options
            .get(i)
            .and_then(|o| o.estimate)
            .map(|e| utility(view, &e))
    };
    let regret = match (r, c) {
        (Some(r), Some(c)) if r == c => Some(0.0),
        (Some(r), Some(c)) => value(r).zip(value(c)).map(|(a, b)| a - b),
        _ => None,
    };
    Agreement {
        reference: r,
        candidate: c,
        regret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[[f64; 5]]) -> Vec<Vec<f64>> {
        values.iter().map(|r| r.to_vec()).collect()
    }

    #[test]
    fn clearly_worse_options_are_dropped_and_ties_resolve_by_index() {
        // Options 0 and 2 tie for best; 1 is close (mean 9.5); 3 and 4 are
        // far below.
        let u = rows(&[
            [10.0, 9.0, 10.0, -40.0, -60.0],
            [10.0, 10.0, 10.0, -40.0, -60.0],
            [10.0, 9.0, 10.0, -40.0, -60.0],
            [10.0, 10.0, 10.0, -40.0, -60.0],
        ]);
        let w = [1.0; 4];
        let keep = survivors(&w, &u, 5, &TwoStage::PROPOSAL).unwrap();
        assert_eq!(keep, vec![0, 1, 2]);
        // At most 2: both tied options outrank the close one.
        let two = TwoStage {
            max_keep: 2,
            ..TwoStage::PROPOSAL
        };
        assert_eq!(survivors(&w, &u, 5, &two).unwrap(), vec![0, 2]);
    }

    #[test]
    fn min_keep_fills_up_in_rank_order() {
        let u = rows(&[
            [50.0, -50.0, -10.0, -60.0, -70.0],
            [50.0, -50.0, -10.0, -60.0, -70.0],
            [50.0, -50.0, -10.0, -60.0, -70.0],
        ]);
        let w = [1.0; 3];
        assert_eq!(
            survivors(&w, &u, 5, &TwoStage::PROPOSAL).unwrap(),
            vec![0, 2]
        );
        let one = TwoStage {
            min_keep: 1,
            ..TwoStage::PROPOSAL
        };
        assert_eq!(survivors(&w, &u, 5, &one).unwrap(), vec![0]);
    }

    #[test]
    fn the_floor_keeps_options_within_its_band() {
        // Constant gap of 4 points: no variance, so only the floor (2.0,
        // times 2.5 = 5 points) keeps option 1; without a floor it goes.
        let u = rows(&[[10.0, 6.0, -30.0, -30.0, -30.0]; 3]);
        let w = [1.0; 3];
        let keep = survivors(&w, &u, 5, &TwoStage::PROPOSAL).unwrap();
        assert_eq!(keep, vec![0, 1]);
        let no_floor = TwoStage {
            se_floor: 0.0,
            min_keep: 1,
            ..TwoStage::PROPOSAL
        };
        assert_eq!(survivors(&w, &u, 5, &no_floor).unwrap(), vec![0]);
    }

    #[test]
    fn weighted_effective_samples_gate_the_screening() {
        let u = rows(&[[10.0, -50.0, -50.0, -50.0, -50.0]; 3]);
        // One world carries almost all weight: ESS < 2.
        assert_eq!(
            survivors(&[1.0, 1e-6, 1e-6], &u, 5, &TwoStage::PROPOSAL),
            None
        );
        assert_eq!(survivors(&[], &[], 5, &TwoStage::PROPOSAL), None);
        assert!(survivors(&[1.0, 1.0, 0.5], &u, 5, &TwoStage::PROPOSAL).is_some());
    }

    #[test]
    fn weights_change_the_paired_standard_error() {
        // Option 1 is 20 points better on world 0 and 20 worse on world 1.
        let u = rows(&[
            [0.0, 20.0, -90.0, -90.0, -90.0],
            [0.0, -20.0, -90.0, -90.0, -90.0],
            [0.0, 0.0, -90.0, -90.0, -90.0],
        ]);
        let even = survivors(&[1.0, 1.0, 1.0], &u, 5, &TwoStage::PROPOSAL).unwrap();
        assert_eq!(even, vec![0, 1]);
        // Weighting world 1 up makes option 1 clearly worse.
        let tight = TwoStage {
            min_keep: 1,
            se_floor: 0.0,
            se_multiple: 0.5,
            ..TwoStage::PROPOSAL
        };
        assert_eq!(survivors(&[1.0, 4.0, 1.0], &u, 5, &tight).unwrap(), vec![0]);
    }

    #[test]
    fn nonsensical_parameters_are_clamped() {
        let p = TwoStage {
            screen_worlds: 0,
            se_multiple: f64::NAN,
            se_floor: -1.0,
            min_keep: 0,
            max_keep: 0,
        }
        .normalized();
        assert_eq!(
            p,
            TwoStage {
                screen_worlds: 1,
                se_multiple: 0.0,
                se_floor: 0.0,
                min_keep: 1,
                max_keep: 1,
            }
        );
    }
}
