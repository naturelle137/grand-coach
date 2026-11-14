// SPDX-License-Identifier: EUPL-1.2

//! Preregistered learner metadata in the existing focus history, never a
//! second store or an inference of external novelty from local history.
use crate::practice_catalogue::ValidationSet;
use crate::{FocusAction, FocusAnswer, LearningHistory};
use serde::{Deserialize, Serialize};

/// Explicit learner report, including exposure outside this installation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriorExposure {
    /// Learner explicitly reports no previous exposure.
    None,
    /// Previously seen, including a screenshot or another installation.
    Seen,
    /// Missing or uncertain report never establishes novelty.
    #[default]
    Unknown,
}

/// Submission categories are deliberately separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationResponse {
    /// Bounded supported response.
    Answer(FocusAnswer),
    /// Learner deliberately skips.
    Skip,
    /// Learner reports the task is unclear.
    Unclear,
    /// Learner ends without submitting an answer.
    Stop,
    /// Unexpected aid, including outside the app: record and stop immediately.
    UnexpectedAid,
}

/// Private per-opportunity evidence carried by focus transitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationEvidence {
    /// Immutable preregistration revision.
    pub protocol: u16,
    /// Initial or delayed four-task set.
    pub set: ValidationSet,
    /// Executable source identity, recorded before any position is exposed.
    pub build: String,
    /// Session entry time, UTC Unix seconds, not an assumed interval.
    pub started_at: u64,
    /// Actual submission/skip/stop time, UTC Unix seconds.
    pub submitted_at: Option<u64>,
    /// Explicit learner report; default unknown.
    pub prior_exposure: PriorExposure,
    /// Explicit unexpected external aid report; always assisted and stopped.
    pub external_aid: bool,
    /// Verbatim own-words explanation, assessed separately from correctness.
    pub own_words: String,
    /// Verbatim clarity note, never a strategy score.
    pub clarity: String,
}

impl ValidationEvidence {
    /// Metadata for deliberate entry. No learner response is invented.
    pub fn new(set: ValidationSet, build: String, started_at: u64) -> Self {
        Self {
            protocol: 1,
            set,
            build,
            started_at,
            submitted_at: None,
            prior_exposure: PriorExposure::Unknown,
            external_aid: false,
            own_words: String::new(),
            clarity: String::new(),
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.protocol == 1
            && !self.build.is_empty()
            && self.build.len() <= 128
            && self.started_at > 0
            && self.submitted_at.is_none_or(|time| time >= self.started_at)
            && self.own_words.len() <= 4096
            && self.clarity.len() <= 4096
    }

    pub(super) fn next(&self) -> Self {
        Self::new(self.set, self.build.clone(), self.started_at)
    }
}

/// Exact descriptive counts, with explicitly separate denominators.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCounts {
    /// All actual presentations, including repeats.
    pub opportunities: u32,
    /// Typed answers submitted.
    pub answers: u32,
    /// Supported scored answers (correct + wrong).
    pub scored: u32,
    /// Exact equality, including assisted practice.
    pub correct: u32,
    /// Supported exact inequality.
    pub wrong: u32,
    /// Unsupported typed answers, no correctness inference.
    pub unsupported: u32,
    /// Explicit skips.
    pub skipped: u32,
    /// Explicit unclear responses.
    pub unclear: u32,
    /// No answer, including explicit stop and abandoned presentation.
    pub unanswered: u32,
    /// Opportunities with an explicit help transition.
    pub help: u32,
    /// Opportunities with any sticky assistance.
    pub assisted: u32,
    /// Presentations after this task's first presentation in this history.
    pub repeats: u32,
    /// Opportunities whose external exposure remains unknown.
    pub exposure_unknown: u32,
    /// First, supported, unaided answers with complete local history and
    /// explicit no-exposure report. Local data alone cannot verify the report.
    pub eligible_answers: u32,
    /// Correct subset of eligible_answers.
    pub eligible_correct: u32,
}

/// Descriptive protocol evidence, not learning/mastery/efficacy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationSummary {
    /// First actual initial entry in retained history, UTC Unix seconds.
    pub initial_at: Option<u64>,
    /// First actual delayed entry, UTC Unix seconds.
    pub delayed_at: Option<u64>,
    /// Signed actual interval, including an accidentally early follow-up.
    pub elapsed_seconds: Option<i128>,
    /// Counts for all validation opportunities, including practice repeats.
    pub counts: ValidationCounts,
}

impl LearningHistory {
    /// Derive exact counts without changing any original results.
    pub fn validation_summary(&self) -> ValidationSummary {
        let mut summary = ValidationSummary::default();
        let mut exposed = std::collections::HashSet::new();
        let mut latest = std::collections::HashMap::new();
        for entry in self
            .focus_entries()
            .iter()
            .filter(|e| e.validation.is_some())
        {
            let state = latest.entry(entry.attempt).or_insert((entry, false));
            state.0 = entry;
            state.1 |= matches!(entry.action, FocusAction::Help);
        }
        for presentation in self
            .focus_entries()
            .iter()
            .filter(|e| e.validation.is_some() && matches!(e.action, FocusAction::Presented))
        {
            let evidence = presentation.validation.as_ref().unwrap();
            let time = match evidence.set {
                ValidationSet::Initial => &mut summary.initial_at,
                ValidationSet::Delayed => &mut summary.delayed_at,
            };
            *time = Some(time.map_or(evidence.started_at, |t| t.min(evidence.started_at)));
            let (last, helped) = latest[&presentation.attempt];
            let data = last.validation.as_ref().unwrap();
            let count = &mut summary.counts;
            count.opportunities += 1;
            let repeat = !exposed.insert(presentation.task.id.clone());
            count.repeats += u32::from(repeat);
            count.help += u32::from(helped);
            count.assisted += u32::from(last.assisted);
            count.exposure_unknown += u32::from(data.prior_exposure == PriorExposure::Unknown);
            match last.action {
                FocusAction::Answer { .. } => {
                    count.answers += 1;
                    match last.correct() {
                        Some(correct) => {
                            count.scored += 1;
                            count.correct += u32::from(correct);
                            count.wrong += u32::from(!correct);
                            if !repeat
                                && last.eligible
                                && !last.assisted
                                && data.prior_exposure == PriorExposure::None
                            {
                                count.eligible_answers += 1;
                                count.eligible_correct += u32::from(correct);
                            }
                        }
                        None => count.unsupported += 1,
                    }
                }
                FocusAction::Skip(_) => count.skipped += 1,
                FocusAction::Unclear => count.unclear += 1,
                _ => count.unanswered += 1,
            }
        }
        summary.elapsed_seconds = summary
            .initial_at
            .zip(summary.delayed_at)
            .map(|(initial, delayed)| i128::from(delayed) - i128::from(initial));
        summary
    }
}
