// SPDX-License-Identifier: EUPL-1.2

//! Catalogue mode of the existing training session. Reconstruction and all
//! answers come from the accepted catalogue, never from a second evaluator.

use serde::{Deserialize, Serialize};

use super::{TrainingMode, TrainingSession};
use crate::AssistanceLevel;
use crate::learning::{AttemptId, AttemptKind, LearningError, LearningHistory};
use crate::practice_catalogue::{
    CaseRole, Certainty, Mechanism, PRACTICE_CASES, PracticeCase, PracticeFamily, PracticeFeedback,
    PracticePosition, PracticeQuestion,
};

/// An immutable catalogue reference, resolved without upgrading revisions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FocusTask {
    /// Catalogue ID, not learner-facing prompt text.
    pub id: String,
    /// Position and question revision.
    pub version: u16,
}

impl FocusTask {
    fn of(case: PracticeCase) -> Self {
        Self {
            id: case.id().into(),
            version: case.version(),
        }
    }

    /// Unknown identities and revisions are unsupported, never substituted.
    pub fn resolve(&self) -> Option<PracticeCase> {
        PracticeCase::find(&self.id, self.version)
    }
}

/// A supported response; mechanisms are not numeric counting deviations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusAnswer {
    /// Exact integer count.
    Count(u8),
    /// At least one permitted hidden distribution, but not every one.
    Possible,
    /// Every permitted hidden distribution has this narrow mechanism.
    Certain,
    /// This narrow mechanism is absent; no safety/optimality assertion.
    Absent,
}

/// One immutable transition of a catalogue opportunity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusAction {
    /// Exposure is recorded even if the learner leaves without answering.
    Presented,
    /// Deliberate reveal, sticky even when support is lowered afterwards.
    Help,
    /// Scored supported question, with shared-analysis provenance.
    Answer {
        /// Actual submitted answer.
        given: FocusAnswer,
        /// Expected answer; None explicitly means insufficient analysis.
        expected: Option<FocusAnswer>,
        /// Analysis used for scoring, distinct from choice optimality.
        feedback: PracticeFeedback,
    },
    /// Skipped tasks reveal feedback and remain practice on subsequent visits.
    Skip(PracticeFeedback),
}

/// One history entry. The opportunity is (attempt, task ID, task version).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusEntry {
    /// Globally persisted attempt identity, never reused after reset.
    pub attempt: AttemptId,
    /// Immutable task identity and revision.
    pub task: FocusTask,
    /// Question without its answer.
    pub question: PracticeQuestion,
    /// Transition and, only after a response or reveal, scoring provenance.
    pub action: FocusAction,
    /// Explicit support at this transition.
    pub assistance: AssistanceLevel,
    /// Assistance was visible during this opportunity (sticky).
    pub assisted: bool,
    /// Eligibility checked before recording this presentation's exposure.
    /// This is only app-local unseen eligibility, never learning efficacy.
    pub eligible: bool,
}

impl FocusEntry {
    fn tag(&self) -> u8 {
        match self.action {
            FocusAction::Presented => 0,
            FocusAction::Help => 1,
            _ => 2,
        }
    }

    /// Supported question correctness; skips and unsupported results are None.
    pub fn correct(&self) -> Option<bool> {
        match self.action {
            FocusAction::Answer {
                given,
                expected: Some(expected),
                ..
            } => Some(given == expected),
            _ => None,
        }
    }

    /// Correct, unaided response to an app-local previously unseen changed task.
    pub fn unseen_correct(&self) -> bool {
        self.eligible && !self.assisted && self.correct() == Some(true)
    }
}

/// Commands for catalogue mode, reusing the training's lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusCommand {
    /// Submit a count or mechanism judgement.
    Answer(FocusAnswer),
    /// Reveal shared-analysis help.
    Help,
    /// Deliberately choose support; visible help is sticky.
    Assistance(AssistanceLevel),
    /// Skip and reveal feedback.
    Skip,
    /// Repeat the settled task as practice.
    Retry,
    /// Next task, cycling through intro/changed/boundary.
    Next,
}

/// The task model contains only the permitted view until explicit feedback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusView {
    /// Persisted transition provenance of the current presentation.
    pub entry: FocusEntry,
    /// Permitted position, without metadata or feedback.
    pub position: crate::PlayerView,
    /// Family chosen by the learner.
    pub family: PracticeFamily,
    /// One-based position in this family's three cases.
    pub number: usize,
    /// Feedback is inaccessible until answer/help/skip.
    pub feedback: Option<PracticeFeedback>,
    /// A response or skip has settled this presentation.
    pub settled: bool,
}

#[derive(Clone, Debug)]
pub(super) struct FocusSession {
    family: PracticeFamily,
    index: usize,
    entry: FocusEntry,
    position: PracticePosition,
    revealed: bool,
    settled: bool,
}

/// Shared scoring over the exact bounded catalogue question, not a card ranking.
/// Empty results are absent only for validated, supported catalogue positions.
pub fn focus_expected(feedback: &PracticeFeedback) -> Option<FocusAnswer> {
    match feedback {
        PracticeFeedback::Count(d) => d.map(|d| FocusAnswer::Count(d.expected())),
        PracticeFeedback::Consequences(ms) => {
            let certainty = |m: &Mechanism| match m {
                Mechanism::Abstich(m) => m.certainty,
                Mechanism::Overtake(m) => m.certainty,
            };
            Some(if ms.iter().any(|m| certainty(m) == Certainty::Certain) {
                FocusAnswer::Certain
            } else if ms.is_empty() {
                FocusAnswer::Absent
            } else {
                FocusAnswer::Possible
            })
        }
    }
}

impl LearningHistory {
    /// Catalogue observations in persisted order, separate from numeric summaries.
    pub fn focus_entries(&self) -> &[FocusEntry] {
        &self.focus
    }

    /// Incomplete, migrated or deliberately reset history cannot prove novelty.
    pub fn invalidate_exposure(&mut self) {
        self.exposure_complete = false;
    }

    /// Eligibility uses actual exposure, never seed or role alone.
    pub fn focus_unseen(&self, task: &FocusTask) -> bool {
        task.resolve().is_some()
            && self.exposure_complete
            && !self.refused
            && !self.focus.iter().any(|e| e.task.id == task.id)
    }

    /// Append one transition, deduplicating by opportunity and action tag.
    pub fn record_focus(&mut self, entry: &FocusEntry) -> Result<(), LearningError> {
        self.check_focus(entry)?;
        if self
            .focus
            .iter()
            .any(|e| e.attempt == entry.attempt && e.task == entry.task && e.tag() == entry.tag())
        {
            return Ok(());
        }
        if self.focus.len() + self.events.len() >= crate::MAX_LEARNING_EVENTS {
            self.refused = true;
            self.invalidate_exposure();
            return Err(LearningError::Full);
        }
        self.focus.push(entry.clone());
        if let Err(error) = self.validate_focus() {
            self.focus.pop();
            return Err(error);
        }
        Ok(())
    }

    fn check_focus(&self, entry: &FocusEntry) -> Result<(), LearningError> {
        if !self
            .attempts
            .iter()
            .any(|a| a.id == entry.attempt && matches!(a.kind, AttemptKind::Training { .. }))
        {
            return Err(LearningError::UnknownAttempt);
        }
        let case = entry
            .task
            .resolve()
            .ok_or(LearningError::InvalidProvenance)?;
        let position = case
            .position()
            .map_err(|_| LearningError::InvalidProvenance)?;
        if position.question() != entry.question {
            return Err(LearningError::InvalidProvenance);
        }
        let valid = match &entry.action {
            FocusAction::Answer {
                given,
                expected,
                feedback,
            } => {
                matches!(entry.question, PracticeQuestion::Count(_))
                    == matches!(given, FocusAnswer::Count(_))
                    && *feedback == position.feedback()
                    && *expected == focus_expected(feedback)
            }
            FocusAction::Skip(feedback) => entry.assisted && *feedback == position.feedback(),
            FocusAction::Help => entry.assisted,
            FocusAction::Presented => true,
        };
        if !valid || (entry.assistance == AssistanceLevel::VisibleAid && !entry.assisted) {
            return Err(LearningError::InvalidProvenance);
        }
        Ok(())
    }

    pub(crate) fn validate_focus(&self) -> Result<(), LearningError> {
        use std::collections::HashSet;
        if self.events.len() + self.focus.len() > crate::MAX_LEARNING_EVENTS {
            return Err(LearningError::TooLarge);
        }
        let mut keys = HashSet::new();
        let mut exposed = HashSet::new();
        let mut presented = std::collections::HashMap::new();
        let mut tasks = std::collections::HashMap::new();
        for e in &self.focus {
            self.check_focus(e)?;
            if tasks
                .insert(e.attempt, &e.task)
                .is_some_and(|task| task != &e.task)
                || self.events.iter().any(|old| old.attempt == e.attempt)
            {
                return Err(LearningError::InvalidContext);
            }
            if !keys.insert((e.attempt, e.task.clone(), e.tag())) {
                return Err(LearningError::DuplicateEvent);
            }
            let key = (e.attempt, e.task.clone());
            if e.tag() == 0 {
                let case = e.task.resolve().ok_or(LearningError::InvalidProvenance)?;
                if e.eligible && (case.role() != CaseRole::Changed || exposed.contains(&e.task.id))
                {
                    return Err(LearningError::InvalidProvenance);
                }
                exposed.insert(e.task.id.clone());
                presented.insert(key, (e.eligible, e.assisted, false));
            } else {
                let state = presented
                    .get_mut(&key)
                    .ok_or(LearningError::InvalidProvenance)?;
                if state.0 != e.eligible || (state.1 && !e.assisted) || state.2 {
                    return Err(LearningError::InvalidProvenance);
                }
                state.1 |= e.assisted;
                state.2 = e.tag() == 2;
            }
        }
        Ok(())
    }
}

impl FocusSession {
    fn present(
        family: PracticeFamily,
        index: usize,
        assistance: AssistanceLevel,
        history: &mut LearningHistory,
        reliable: bool,
    ) -> Result<Self, LearningError> {
        let case = PRACTICE_CASES
            .into_iter()
            .filter(|c| c.family() == Some(family))
            .nth(index)
            .ok_or(LearningError::InvalidProvenance)?;
        let task = FocusTask::of(case);
        let eligible = reliable && case.role() == CaseRole::Changed && history.focus_unseen(&task);
        let position = case
            .position()
            .map_err(|_| LearningError::InvalidProvenance)?;
        let presentation = 1 + history
            .focus
            .iter()
            .filter(|e| e.task == task && matches!(e.action, FocusAction::Presented))
            .count() as u32;
        let attempt = history.begin_attempt(AttemptKind::Training {
            attempt: presentation,
        })?;
        let entry = FocusEntry {
            attempt,
            task,
            question: position.question(),
            action: FocusAction::Presented,
            assistance,
            assisted: assistance == AssistanceLevel::VisibleAid,
            eligible,
        };
        history.record_focus(&entry)?;
        Ok(Self {
            family,
            index,
            revealed: entry.assisted,
            entry,
            position,
            settled: false,
        })
    }
}

impl TrainingSession {
    /// Enter catalogue mode. Record exposure before giving any task to a frontend.
    /// `reliable` requires a working persistent history, not an in-memory fallback.
    pub fn catalogue(
        family: PracticeFamily,
        assistance: AssistanceLevel,
        history: &mut LearningHistory,
        reliable: bool,
    ) -> Result<Self, LearningError> {
        let focus = FocusSession::present(family, 0, assistance, history, reliable)?;
        Ok(Self {
            mode: TrainingMode::Catalogue(focus),
        })
    }

    /// Catalogue view, or None for a legacy four-fixture training.
    pub fn focus_view(&self) -> Option<FocusView> {
        let TrainingMode::Catalogue(s) = &self.mode else {
            return None;
        };
        Some(FocusView {
            entry: s.entry.clone(),
            position: s.position.view().clone(),
            family: s.family,
            number: s.index + 1,
            feedback: (s.revealed || s.settled).then(|| s.position.feedback()),
            settled: s.settled,
        })
    }

    /// Disable credit if saving the exposure failed before the screen was drawn.
    pub fn withhold_focus_credit(&mut self, history: &mut LearningHistory) {
        history.invalidate_exposure();
        if let TrainingMode::Catalogue(s) = &mut self.mode {
            s.entry.eligible = false;
            for e in history
                .focus
                .iter_mut()
                .filter(|e| e.attempt == s.entry.attempt)
            {
                e.eligible = false;
            }
        }
    }

    /// Transition catalogue mode and record in the same history used by games.
    pub fn execute_focus(
        &mut self,
        command: FocusCommand,
        history: &mut LearningHistory,
        reliable: bool,
    ) -> Result<(), LearningError> {
        let TrainingMode::Catalogue(s) = &mut self.mode else {
            return Err(LearningError::InvalidContext);
        };
        match command {
            FocusCommand::Retry | FocusCommand::Next => {
                if !s.settled {
                    return Err(LearningError::InvalidContext);
                }
                let index = if command == FocusCommand::Next {
                    (s.index + 1) % 3
                } else {
                    s.index
                };
                *s = FocusSession::present(s.family, index, s.entry.assistance, history, reliable)?;
                return Ok(());
            }
            FocusCommand::Assistance(level) => {
                s.entry.assistance = level;
                if level != AssistanceLevel::VisibleAid || s.settled {
                    return Ok(());
                }
            }
            _ if s.settled => return Err(LearningError::InvalidContext),
            _ => {}
        }
        let mut entry = s.entry.clone();
        match command {
            FocusCommand::Help | FocusCommand::Assistance(_) => {
                if s.revealed {
                    return Ok(());
                }
                entry.assisted = true;
                entry.action = FocusAction::Help;
            }
            FocusCommand::Answer(given) => {
                if matches!(entry.question, PracticeQuestion::Count(_))
                    != matches!(given, FocusAnswer::Count(_))
                {
                    return Err(LearningError::InvalidProvenance);
                }
                let feedback = s.position.feedback();
                entry.action = FocusAction::Answer {
                    given,
                    expected: focus_expected(&feedback),
                    feedback,
                };
            }
            FocusCommand::Skip => {
                entry.assisted = true;
                entry.action = FocusAction::Skip(s.position.feedback());
            }
            _ => unreachable!(),
        }
        history.record_focus(&entry)?;
        s.revealed = true;
        s.settled = !matches!(entry.action, FocusAction::Help);
        s.entry = entry;
        Ok(())
    }
}
