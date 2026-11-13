// SPDX-License-Identifier: EUPL-1.2

//! Shared analysis for opponents and coach (architecture: "coach and AI do
//! not compute separately").
//!
//! - card counting: what one seat knows from its
//!   [`PlayerView`](skat_core::PlayerView) ([`Knowledge`]) and the trick in
//!   progress ([`TrickState`]),
//! - opponent model: hidden-card distributions under hard constraints
//!   ([`sampling`]) weighted by bidding evidence ([`evidence`]),
//! - option evaluation for every decision ([`analyze`]): Monte Carlo over
//!   those distributions, double-dummy card play ([`double_dummy`]) and a
//!   shared continuation policy ([`policy`]), within a deterministic
//!   [`Budget`],
//! - the exact counting aid and counting questions ([`counting`]),
//! - simple hand measures ([`hand`]),
//! - the evidence contract for coach lessons ([`lesson`]) and the exact
//!   card-play mechanisms they may cite ([`mechanism`]), and the exact
//!   pre-play hand structure and game values ([`preplay`]),
//! - an opt-in experimental two-stage card-play analysis and a paired
//!   comparison with the single-stage reference ([`two_stage`], #104).
//!
//! Semantics, budgets and measured latency: `docs/analysis.md`.

mod analysis;
pub mod baseline;
mod budget;
pub mod counting;
pub mod double_dummy;
pub mod evidence;
pub mod hand;
mod knowledge;
pub mod lesson;
pub mod mechanism;
pub mod policy;
pub mod preplay;
pub mod sampling;
mod trick;
pub mod two_stage;

pub use analysis::{
    Analysis, Assumption, CardClass, Effort, Estimate, Inference, Observation, OptionValue, Status,
    analyze, analyze_experimental, analyze_forgetful, analyze_known, utility,
};
pub use budget::{Budget, Cancel, Interrupted, Meter, max_threads, with_max_threads};
pub use knowledge::{Knowledge, TrumpfSets};
pub use trick::TrickState;
pub use two_stage::{
    Agreement, Kernel, OptionStage, SkipReason, StageOutcome, StageReport, TwoStage,
    TwoStageAnalysis, agreement, analyze_two_stage, best_option,
};
