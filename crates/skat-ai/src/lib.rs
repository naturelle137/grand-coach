// SPDX-License-Identifier: EUPL-1.2

//! Opponent AI (M4, D-07, D-40, D-41): [`Level`] selects a [`Profile`] of
//! analysis effort, memory and mistakes; an [`Opponent`] picks one of the
//! options evaluated by `skat-analysis` or follows the rules of thumb of
//! [`BaselineOpponent`]. [`Profile::GOLDFISH`] forgets played cards;
//! [`Mephisto`] decides with every card known. A [`NeuralOpponent`] decides
//! with a learned model of `skat-nn` and falls back to a Classic
//! [`Opponent`] (D-07).
//!
//! An [`Opponent`] only ever receives the
//! [`PlayerView`](skat_core::PlayerView) of its own seat (D-23) and picks
//! one of the legal actions listed there; [`Mephisto`] takes the whole
//! [`Game`](skat_core::Game).
//! [`arena`] plays AI-vs-AI games for calibration (`docs/ai.md`).

pub mod arena;
mod baseline;
mod mephisto;
mod neural;
mod opponent;
mod rookie;

pub use baseline::BaselineOpponent;
pub use mephisto::Mephisto;
pub use neural::{NeuralDecision, NeuralOpponent, Player};
pub use opponent::{Decision, Level, Opponent, Profile};
pub use rookie::Mistakes;
pub use skat_analysis::{Analysis, Budget, Cancel, OptionValue, Status, utility};
