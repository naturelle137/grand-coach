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
//!     ..SessionConfig::default()
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
mod guided;
mod learning;
mod learning_io;
mod neural;
mod quiz;
mod recording;
mod recording_io;
mod replay_coach;
mod replay_review;
mod series;
mod session;
mod sort;
#[cfg(test)]
mod tests;
mod training;
mod view;

pub use coach::{
    Advice, CoachEvent, CoachJob, CoachLevel, CoachResult, CoachSettings, CoachView, Debriefing,
    DebriefingMoment, LastMove, Pending, Unavailable,
};
pub use context::DecisionContext;
pub use learning::{
    AttemptId, AttemptKind, COMPARISON_WINDOW, CategoryCounts, ContextCount, LEARNING_FORMAT,
    LEARNING_FORMATS_READ, LIMITED_OBSERVATIONS, LearningAttempt, LearningContext, LearningEntry,
    LearningError, LearningHistory, LearningRow, LearningSummary, LearningTopic,
    MAX_LEARNING_ATTEMPTS, MAX_LEARNING_EVENTS, Observations, ProvenanceCount, Recorded,
    ResponseCategory, ResponseCounts, ResponseWindow, RetrievalCounts, TopicComparison,
};
pub use learning_io::{LearningFileError, MAX_LEARNING_BYTES};
pub use neural::{neural_model, neural_model_available};
pub use quiz::{
    AssistSettings, AssistView, AssistanceLevel, CountingState, CountingView, LessonFamily,
    LessonRevision, OpportunityId, QUESTION_AFTER_TRICKS, QuestionId, QuizFeedback, QuizOutcome,
    QuizView, RetrievalAction, RetrievalEvent,
};
pub use recording::{
    GameRecording, MAX_RECORDING_ACTIONS, PrivilegedRecordingPosition, RECORDING_ENGINE,
    RECORDING_FORMAT, RecordingContext, RecordingEnd, RecordingError,
};
pub use recording_io::{MAX_RECORDING_BYTES, RecordingFileError};
pub use replay_coach::{
    REPLAY_COACH_SEED, REPLAY_COACH_TIME_LIMIT, RecordedDecision, ReplayCoach, ReplayCoachJob,
    ReplayCoachResult, ReplayCoachView, ReplayComment, ReplayPerspective, ReplayStale,
    ReplayStatus,
};
pub use replay_review::{
    REPLAY_REVIEW_DECISION_LIMIT, REPLAY_REVIEW_TIME_LIMIT, ReplayReviewJob, ReplayReviewResult,
    ReviewEnd, ReviewMoment, ReviewStatus, ReviewSummary, ReviewUnavailable,
};
pub use series::{GameRecord, ScoreSheet, SeriesResult};
pub use session::{
    AppError, AppEvent, Command, OpponentJob, OpponentMove, Opponents, PlayerId, SeriesLength,
    Session, SessionConfig, TableDisplay,
};
pub use skat_ai::{Budget, Cancel, Level};
/// Exact counting facts and question kinds (D-64); frontends translate them.
pub use skat_analysis::counting::{AugenCount, Derivation, QuestionKind, TrumpfCount, Voids};
/// Structured coach explanations (D-13); frontends translate them.
pub use skat_coach as coach_model;
pub use sort::sort_hand;
pub use training::{
    TrainingCommand, TrainingError, TrainingFixture, TrainingSession, TrainingTask, TrainingTopic,
    TrainingTrick, TrainingView,
};
pub use view::{CoachBasis, OpenCards, PlayedCard, ResultView, SeatInfo, TableView, TrickView};

/// Core data types shown by frontends.
pub use skat_core::{
    Action, Announcement, BidRecord, Call, Card, CardSet, Contract, Deal, Event, Follow,
    GameResult, GameType, KontraLevel, OmniscientView, Phase, PlayerView, Rank, RuleConfig,
    RuleError, Seat, Spitzen, Suit, reizwerte,
};

pub use guided::{
    GuidedAction, GuidedAnswer, GuidedBranch, GuidedEvent, GuidedId, GuidedStep, GuidedView,
};
