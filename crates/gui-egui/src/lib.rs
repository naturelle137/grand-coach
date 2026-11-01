// SPDX-License-Identifier: EUPL-1.2

//! egui desktop frontend of Grand-Coach. Uses only `skat-app` for game
//! functionality (D-12); all texts come from `assets/i18n/` (D-32).

mod app;
pub mod cards;
pub mod coach;
pub mod i18n;
mod text;

pub use app::GrandCoachApp;
pub use cards::{Deck, Theme};
pub use i18n::Language;
