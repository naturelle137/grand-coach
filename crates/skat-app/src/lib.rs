// SPDX-License-Identifier: EUPL-1.2

//! Application interface of Grand-Coach (D-12): session, commands, events
//! and view models as plain, serialisable data. Frontends use only this
//! crate; it re-exports the core data types they need to display.
//!
//! One human (player 0) plays a series against two opponents (players 1
//! and 2): each with its own strength [`Level`] (D-40), or one of the table
//! modes Goldfish and Mephisto ([`Opponents`], D-41). Series semantics:
//! D-31 in `docs/decisions.md`. Opponent decisions can be computed off the
//! UI thread with [`Session::opponent_job`].
//!
//! ```
//! use skat_app::{Command, Level, Opponents, Session, SessionConfig, SeriesLength};
//!
//! let mut session = Session::new(SessionConfig {
//!     seed: 7,
//!     series: SeriesLength::Fixed(3),
//!     kontra_re: false,
//!     opponents: Opponents::Levels([Level::Easy, Level::Easy]),
//! });
//! while !session.view().series_finished {
//!     session.run_opponents();
//!     let view = session.view();
//!     let command = match view.legal_actions.first() {
//!         Some(&action) => Command::Act(action),
//!         None => Command::NextGame,
//!     };
//!     session.execute(command).unwrap();
//! }
//! assert_eq!(session.view().sheet.records.len(), 3);
//! ```

mod coach;
mod context;
mod series;
mod session;
mod sort;
#[cfg(test)]
mod tests;
mod view;

pub use coach::{
    Advice, CoachEvent, CoachJob, CoachLevel, CoachResult, CoachSettings, CoachView, Debriefing,
    DebriefingMoment, LastMove, Pending, Unavailable,
};
pub use context::DecisionContext;
pub use series::{GameRecord, ScoreSheet, SeriesResult};
pub use session::{
    AppError, AppEvent, Command, OpponentJob, OpponentMove, Opponents, PlayerId, SeriesLength,
    Session, SessionConfig,
};
pub use skat_ai::{Budget, Cancel, Level};
/// Structured coach explanations (D-13); frontends translate them.
pub use skat_coach as coach_model;
pub use sort::sort_hand;
pub use view::{OpenCards, PlayedCard, ResultView, SeatInfo, TableView, TrickView};

/// Core data types shown by frontends.
pub use skat_core::{
    Action, Announcement, BidRecord, Call, Card, CardSet, Contract, Event, GameResult, GameType,
    KontraLevel, Phase, Rank, RuleError, Seat, Spitzen, Suit, reizwerte,
};
