// SPDX-License-Identifier: EUPL-1.2

//! egui desktop frontend of Grand-Coach. Uses only `skat-app` for game
//! functionality (D-12); all texts come from `assets/i18n/` (D-32).

mod app;
pub mod assist;
pub mod cards;
pub mod coach;
mod comparison;
pub mod i18n;
pub mod learning;
mod recording;
pub mod style;
mod text;
mod training;

pub use app::{GrandCoachApp, SmokeGoal};
pub use cards::{Deck, Theme};
pub use i18n::Language;

/// Guided dialogue component, also used by permitted-view fixture renderers.
pub mod guided;
mod live_abstich;
