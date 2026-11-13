// SPDX-License-Identifier: EUPL-1.2

//! Bounded review of a completed recording (M7, D-69). On request, every
//! recorded human decision with a choice is analysed, one at a time and in
//! game order, from the human's historical permitted view with the seed of
//! the single historical comment (D-67); the shared
//! [`skat_coach::moments`] (D-58) then selects zero to three teachable
//! moments, exactly as the live debriefing does. Nothing here analyses,
//! ranks or thresholds on its own, and no hindsight or concealed card is
//! ever used.
//!
//! Review jobs and results are their own types: they cannot be passed to
//! [`crate::Session::finish_coach`] or [`crate::ReplayCoach::finish`], and
//! never reach opponents or practice attempts.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use skat_ai::{Budget, Cancel};
use skat_coach::{ComparedOption, Concept, Moment, MoveReview, decision_seed, moments};
use skat_core::{Action, PlayerView};

use crate::coach::Unavailable;
use crate::context::DecisionContext;
use crate::recording::RecordingEnd;
use crate::replay_coach::{REPLAY_COACH_SEED, ReplayCoach, ReplayComment, ReplayStale, comment};

/// Wall-clock limit a frontend gives one decision of the review on top of
/// its sample and node budget. A decision that reaches it counts as
/// unavailable, never as a judgement, and the review goes on.
pub const REPLAY_REVIEW_DECISION_LIMIT: Duration = Duration::from_secs(20);

/// Wall-clock limit a frontend gives the whole review. Reaching it ends the
/// review as [`ReviewEnd::TimeLimit`]: moments are then selected among the
/// decisions analysed so far and labelled as such.
pub const REPLAY_REVIEW_TIME_LIMIT: Duration = Duration::from_secs(120);

/// Identifies one review analysis: opened recording, review generation,
/// decision, budget and seed. A result is accepted only for exactly the
/// outstanding key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ReviewKey {
    source: u64,
    generation: u64,
    decision: usize,
    prefix: usize,
    budget: Budget,
    seed: u64,
}

/// The analysis of one recorded decision for the review. Holds only the
/// human's permitted view before the action; there is no hindsight review.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReviewJob {
    key: ReviewKey,
    view: Box<PlayerView>,
    chosen: Action,
}

impl ReplayReviewJob {
    /// Analyses and judges the recorded action with the shared coach, as a
    /// historical single comment would. `cancel` (with
    /// [`REPLAY_REVIEW_DECISION_LIMIT`]) stops it.
    pub fn run(&self, cancel: Option<&Cancel>) -> ReplayReviewResult {
        ReplayReviewResult {
            key: self.key,
            outcome: comment(
                &self.view,
                None,
                self.chosen,
                self.key.budget,
                self.key.seed,
                cancel,
            ),
        }
    }

    /// Index of the decision in [`ReplayCoach::decisions`].
    pub fn decision(&self) -> usize {
        self.key.decision
    }

    /// Sampling seed: [`decision_seed`] of [`REPLAY_COACH_SEED`] and the
    /// permitted view, as for the historical single comment.
    pub fn seed(&self) -> u64 {
        self.key.seed
    }

    /// Work budget.
    pub fn budget(&self) -> Budget {
        self.key.budget
    }

    /// The permitted view analysed (the human's, before the action).
    pub fn view(&self) -> &PlayerView {
        &self.view
    }
}

/// A finished review analysis for [`ReplayCoach::finish_review`].
///
/// ```compile_fail
/// fn leak(coach: &mut skat_app::ReplayCoach, r: skat_app::ReplayReviewResult) {
///     let _ = coach.finish(r);
/// }
/// ```
///
/// ```compile_fail
/// fn leak(live: &mut skat_app::Session, r: skat_app::ReplayReviewResult) {
///     let _ = live.finish_coach(r);
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReviewResult {
    key: ReviewKey,
    outcome: Result<Box<ReplayComment>, Unavailable>,
}

impl ReplayReviewResult {
    /// Index of the decision in [`ReplayCoach::decisions`].
    pub fn decision(&self) -> usize {
        self.key.decision
    }

    /// `true` if the analysis was cancelled (by its deadline).
    pub fn cancelled(&self) -> bool {
        self.outcome == Err(Unavailable::Cancelled)
    }
}

/// Why the review cannot be requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewUnavailable {
    /// The recording ends with a decision outstanding: its game is not
    /// over, so it is not reviewed as a whole.
    Incomplete,
    /// The recording has no recorded human action.
    NoDecisions,
}

/// How a review ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewEnd {
    /// Every eligible decision was attempted.
    Finished,
    /// The learner cancelled it.
    Cancelled,
    /// [`REPLAY_REVIEW_TIME_LIMIT`] was reached.
    TimeLimit,
}

/// One selected moment with where it happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewMoment {
    /// Index of the decision in [`ReplayCoach::decisions`]; selecting it
    /// navigates to the same comment before the action.
    pub decision: usize,
    /// The shared moment (D-58): kind, concept, lesson and review.
    pub moment: Moment,
    /// The legal alternative the choice is compared with.
    pub alternative: Option<ComparedOption>,
    /// The human's permitted table before the action.
    pub context: DecisionContext,
}

/// The result of a review, possibly partial.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewSummary {
    /// How it ended.
    pub end: ReviewEnd,
    /// Recorded human decisions.
    pub decisions: u32,
    /// Of these, with only one legal option (never analysed).
    pub forced: u32,
    /// Decisions with a choice: the decisions the review covers.
    pub eligible: u32,
    /// Eligible decisions analysed with a comment.
    pub analysed: u32,
    /// Of the analysed, with a provisional (incomplete) analysis.
    pub provisional: u32,
    /// Eligible decisions whose analysis reached its limit or failed.
    pub unavailable: u32,
    /// The focus the moments were selected with.
    pub focus: Option<Concept>,
    /// Zero to three moments among the analysed decisions, in game order.
    pub moments: Vec<ReviewMoment>,
}

impl ReviewSummary {
    /// `true` only if every eligible decision was analysed: then the
    /// moments are selected from the whole recording.
    pub fn exhaustive(&self) -> bool {
        self.analysed == self.eligible
    }

    /// Eligible decisions not attempted (cancelled or time limit).
    pub fn pending(&self) -> u32 {
        self.eligible - self.analysed - self.unavailable
    }
}

/// What the review part of the recording window shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReviewStatus {
    /// The review cannot be requested for this recording.
    Unavailable(ReviewUnavailable),
    /// Nothing requested (or reset by a budget change).
    Idle {
        /// Decisions the review would analyse.
        eligible: u32,
    },
    /// Running: `attempted` of `eligible` decisions done so far.
    Running {
        /// Decisions done (analysed or unavailable).
        attempted: u32,
        /// Decisions the review covers.
        eligible: u32,
    },
    /// Ended; moments and scope.
    Done(Box<ReviewSummary>),
}

/// The review's state inside a [`ReplayCoach`]: one per opened recording.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReviewState {
    generation: u64,
    focus: Option<Concept>,
    requested: bool,
    end: Option<ReviewEnd>,
    outstanding: Option<ReviewKey>,
    /// Per decision, the outcome computed with the coach's current budget.
    results: Vec<Option<Result<Box<ReplayComment>, Unavailable>>>,
}

impl ReviewState {
    /// Drops the review and its cache (budget change).
    pub(crate) fn reset(&mut self) {
        *self = ReviewState {
            generation: self.generation + 1,
            focus: self.focus,
            ..ReviewState::default()
        };
    }

    /// The cached historical comment on decision `index`, if analysed.
    pub(crate) fn cached(&self, index: usize) -> Option<&ReplayComment> {
        match self.results.get(index) {
            Some(Some(Ok(c))) => Some(c),
            _ => None,
        }
    }

    fn running(&self) -> bool {
        self.requested && self.end.is_none()
    }

    fn stop(&mut self, end: ReviewEnd) {
        self.generation += 1;
        self.outstanding = None;
        self.end = Some(end);
    }
}

impl ReplayCoach {
    /// Why the review cannot be requested, if so.
    pub fn review_unavailable(&self) -> Option<ReviewUnavailable> {
        if self.recording.end == RecordingEnd::InProgress {
            Some(ReviewUnavailable::Incomplete)
        } else if self.decisions.is_empty() {
            Some(ReviewUnavailable::NoDecisions)
        } else {
            None
        }
    }

    /// Starts (or retries) the recording review. Decisions analysed before
    /// with the same budget are kept; unavailable ones are tried again.
    /// `false` if the review is unavailable or a single comment is being
    /// computed (one replay analysis at a time).
    pub fn request_review(&mut self) -> bool {
        if self.review_unavailable().is_some() || self.computing() {
            return false;
        }
        let r = &mut self.review;
        r.results.resize(self.decisions.len(), None);
        for slot in &mut r.results {
            if matches!(slot, Some(Err(_))) {
                *slot = None;
            }
        }
        r.generation += 1;
        r.outstanding = None;
        r.requested = true;
        r.end = None;
        if self.next_review_decision().is_none() {
            self.review.end = Some(ReviewEnd::Finished);
        }
        true
    }

    /// The next review analysis, in game order. `None` unless the review
    /// runs with no analysis outstanding.
    pub fn review_job(&mut self) -> Option<ReplayReviewJob> {
        if !self.review.running() || self.review.outstanding.is_some() {
            return None;
        }
        let Some(index) = self.next_review_decision() else {
            self.review.end = Some(ReviewEnd::Finished);
            return None;
        };
        let d = self.decisions[index];
        let view = self.human_view(d.prefix);
        let key = ReviewKey {
            source: self.source,
            generation: self.review.generation,
            decision: index,
            prefix: d.prefix,
            budget: self.budget,
            seed: decision_seed(REPLAY_COACH_SEED, &view),
        };
        self.review.outstanding = Some(key);
        Some(ReplayReviewJob {
            key,
            view: Box::new(view),
            chosen: d.action,
        })
    }

    /// Accepts the result of the outstanding review analysis only. The
    /// review ends once every eligible decision has been attempted.
    pub fn finish_review(&mut self, result: ReplayReviewResult) -> Result<(), ReplayStale> {
        if self.review.outstanding != Some(result.key) {
            return Err(ReplayStale::Stale);
        }
        self.review.outstanding = None;
        self.review.results[result.key.decision] = Some(result.outcome);
        if self.next_review_decision().is_none() {
            self.review.end = Some(ReviewEnd::Finished);
        }
        Ok(())
    }

    /// Cancels a running review; analysed decisions are kept.
    pub fn cancel_review(&mut self) {
        if self.review.running() {
            self.review.stop(ReviewEnd::Cancelled);
        }
    }

    /// Ends a running review at [`REPLAY_REVIEW_TIME_LIMIT`]; analysed
    /// decisions are kept.
    pub fn review_time_limit(&mut self) {
        if self.review.running() {
            self.review.stop(ReviewEnd::TimeLimit);
        }
    }

    /// Sets the learning focus moments are selected with (D-58). The focus
    /// is no analysis input, so analysed decisions stay valid and only the
    /// selection changes.
    pub fn set_review_focus(&mut self, focus: Option<Concept>) {
        self.review.focus = focus;
    }

    /// `true` while the review runs.
    pub fn review_running(&self) -> bool {
        self.review.running()
    }

    /// The review status.
    pub fn review_status(&self) -> ReviewStatus {
        if let Some(reason) = self.review_unavailable() {
            return ReviewStatus::Unavailable(reason);
        }
        let eligible = self.decisions.iter().filter(|d| d.options > 1).count() as u32;
        let r = &self.review;
        if !r.requested {
            return ReviewStatus::Idle { eligible };
        }
        let outcomes = || {
            self.decisions
                .iter()
                .enumerate()
                .filter(|(_, d)| d.options > 1)
                .filter_map(|(i, _)| r.results.get(i).and_then(Option::as_ref).map(|o| (i, o)))
        };
        let Some(end) = r.end else {
            return ReviewStatus::Running {
                attempted: outcomes().count() as u32,
                eligible,
            };
        };
        let analysed: Vec<(usize, &ReplayComment)> = outcomes()
            .filter_map(|(i, o)| o.as_deref().ok().map(|c| (i, c)))
            .collect();
        let reviews: Vec<MoveReview> = analysed.iter().map(|(_, c)| c.review.clone()).collect();
        let moments = moments(&reviews, r.focus)
            .into_iter()
            .map(|moment| {
                let (decision, c) = analysed[moment.index];
                ReviewMoment {
                    decision,
                    alternative: c.alternative.clone(),
                    context: self.context_of(decision),
                    moment,
                }
            })
            .collect();
        ReviewStatus::Done(Box::new(ReviewSummary {
            end,
            decisions: self.decisions.len() as u32,
            forced: self.decisions.len() as u32 - eligible,
            eligible,
            analysed: analysed.len() as u32,
            provisional: analysed
                .iter()
                .filter(|(_, c)| c.review.status != skat_coach::Status::Complete)
                .count() as u32,
            unavailable: outcomes().filter(|(_, o)| o.is_err()).count() as u32,
            focus: r.focus,
            moments,
        }))
    }

    /// The first eligible decision without an outcome.
    fn next_review_decision(&self) -> Option<usize> {
        (0..self.decisions.len()).find(|&i| {
            self.decisions[i].options > 1 && matches!(self.review.results.get(i), Some(None))
        })
    }
}
