// SPDX-License-Identifier: EUPL-1.2

//! Descriptive explicit mechanism responses, never strategic play diagnoses.
//! See `docs/mechanism-responses.md`. Inputs are validated format 4 entries.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::practice_catalogue::{MechanismKind, PracticeQuestion};
use crate::{
    COMPARISON_WINDOW, FocusAction, FocusAnswer, FocusEntry, FocusTask, LearningContext,
    LearningHistory,
};

/// Observed response against supported conditional evidence, independent of help.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MechanismResponse {
    /// Same answer as the supported analysis result.
    Correct,
    /// Certain asserted for merely possible evidence.
    OverstatedCertainty,
    /// Absence asserted despite a supported possibility (including certainty).
    DeniedPossibility,
    /// Presence asserted despite supported absence.
    FalsePresence,
    /// Merely possible asserted for certain evidence.
    UnderstatedCertainty,
    /// Missing or incompatible evidence; no diagnosis.
    Unsupported,
    /// Presented, not settled.
    Unanswered,
    /// Explicit skip, no submitted answer.
    Skipped,
}

impl MechanismResponse {
    /// Explicit answer taxonomy. None/numeric evidence is unsupported here.
    pub fn classify(given: FocusAnswer, expected: Option<FocusAnswer>) -> Self {
        use FocusAnswer::{Absent, Certain, Possible};
        match (given, expected) {
            (Absent, Some(Absent)) | (Possible, Some(Possible)) | (Certain, Some(Certain)) => {
                Self::Correct
            }
            (Certain, Some(Possible)) => Self::OverstatedCertainty,
            (Absent, Some(Possible | Certain)) => Self::DeniedPossibility,
            (Possible | Certain, Some(Absent)) => Self::FalsePresence,
            (Possible, Some(Certain)) => Self::UnderstatedCertainty,
            _ => Self::Unsupported,
        }
    }

    fn of(entry: &FocusEntry) -> Self {
        match entry.action {
            FocusAction::Answer {
                given, expected, ..
            } => Self::classify(given, expected),
            FocusAction::Skip(_) => Self::Skipped,
            _ => Self::Unanswered,
        }
    }
}

/// Exposure known only within the currently available app history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MechanismExposure {
    /// First recorded presentation, with complete available history.
    First,
    /// A preceding recorded presentation of the same task ID exists.
    Repeated,
    /// Incomplete history cannot establish first exposure.
    Unknown,
}

/// Opportunity denominators and category counts; no percentages or mastery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismCounts {
    /// All unique observed presentations.
    pub presented: u32,
    /// Explicit submitted answers, including unsupported answers.
    pub answered: u32,
    /// Supported scored answers (correct plus the four mismatch categories).
    pub scored: u32,
    /// Explicit skips.
    pub skipped: u32,
    /// Unanswered plus unsupported answers.
    pub unknown: u32,
    /// Same supported answer.
    pub correct: u32,
    /// Certain for possible.
    pub overstated: u32,
    /// Absent for possible/certain.
    pub denied: u32,
    /// Possible/certain for absent.
    pub false_presence: u32,
    /// Possible for certain.
    pub understated: u32,
    /// Answers without compatible evidence.
    pub unsupported: u32,
    /// No settlement recorded.
    pub unanswered: u32,
}
impl MechanismCounts {
    fn add(&mut self, response: MechanismResponse) {
        use MechanismResponse::*;
        self.presented += 1;
        match response {
            Unanswered => {
                self.unanswered += 1;
                self.unknown += 1;
            }
            Skipped => self.skipped += 1,
            Unsupported => {
                self.answered += 1;
                self.unsupported += 1;
                self.unknown += 1;
            }
            r => {
                self.answered += 1;
                self.scored += 1;
                match r {
                    Correct => self.correct += 1,
                    OverstatedCertainty => self.overstated += 1,
                    DeniedPossibility => self.denied += 1,
                    FalsePresence => self.false_presence += 1,
                    UnderstatedCertainty => self.understated += 1,
                    _ => unreachable!(),
                }
            }
        }
    }
}

/// Two adjacent ten-settlement windows within exactly the same row context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismComparison {
    /// Ten settlements immediately before the recent ten.
    pub earlier: MechanismCounts,
    /// Most recent ten settlements, in persisted presentation order.
    pub recent: MechanismCounts,
}

/// A task and its observed context. Never pools different evidence domains.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismRow {
    /// Immutable catalogue ID and version.
    pub task: FocusTask,
    /// Narrow mechanism topic.
    pub kind: MechanismKind,
    /// Actual attempt context; currently only dedicated catalogue training.
    pub context: LearningContext,
    /// Sticky help, including initially visible aid.
    pub assisted: bool,
    /// App-local exposure, independent of changed-task eligibility.
    pub exposure: MechanismExposure,
    /// Recorded app-local unseen changed case, not a transfer measure.
    pub eligible: bool,
    /// All unique observed opportunities in this row.
    pub counts: MechanismCounts,
    /// None below twenty settlements of this exact row.
    pub comparison: Option<MechanismComparison>,
}

impl LearningHistory {
    /// Derive from immutable validated entries in O(history) expected time and
    /// bounded memory. No catalogue reconstruction or engine call.
    pub fn mechanism_responses(&self) -> Vec<MechanismRow> {
        let kinds: HashMap<_, _> = self
            .attempts()
            .iter()
            .map(|a| (a.id, a.kind.context()))
            .collect();
        let mut opportunities: Vec<(&FocusEntry, MechanismExposure)> = Vec::new();
        let mut indices: HashMap<_, usize> = HashMap::new();
        let mut exposed = HashSet::new();
        for e in self.focus_entries() {
            if !matches!(e.question, PracticeQuestion::Consequence { .. }) {
                continue;
            }
            let key = (e.attempt, &e.task);
            if let Some(&index) = indices.get(&key) {
                opportunities[index] = (e, opportunities[index].1);
            } else {
                let exposure = if !exposed.insert(&e.task.id) {
                    MechanismExposure::Repeated
                } else if self.exposure_complete {
                    MechanismExposure::First
                } else {
                    MechanismExposure::Unknown
                };
                indices.insert(key, opportunities.len());
                opportunities.push((e, exposure));
            }
        }
        let mut rows: Vec<MechanismRow> = Vec::new();
        let mut row_indices = HashMap::new();
        let mut settled: Vec<Vec<MechanismResponse>> = Vec::new();
        for (e, exposure) in opportunities {
            let Some(&context) = kinds.get(&e.attempt) else {
                continue;
            };
            let PracticeQuestion::Consequence { kind, .. } = e.question else {
                continue;
            };
            let key = (&e.task, context, e.assisted, exposure, e.eligible);
            let index = *row_indices.entry(key).or_insert_with(|| {
                rows.push(MechanismRow {
                    task: e.task.clone(),
                    kind,
                    context,
                    assisted: e.assisted,
                    exposure,
                    eligible: e.eligible,
                    counts: MechanismCounts::default(),
                    comparison: None,
                });
                settled.push(Vec::new());
                rows.len() - 1
            });
            let response = MechanismResponse::of(e);
            rows[index].counts.add(response);
            if response != MechanismResponse::Unanswered {
                settled[index].push(response);
            }
        }
        let n = COMPARISON_WINDOW as usize;
        for (row, responses) in rows.iter_mut().zip(settled) {
            if responses.len() >= 2 * n {
                let mut earlier = MechanismCounts::default();
                let mut recent = MechanismCounts::default();
                for &r in &responses[responses.len() - 2 * n..responses.len() - n] {
                    earlier.add(r);
                }
                for &r in &responses[responses.len() - n..] {
                    recent.add(r);
                }
                row.comparison = Some(MechanismComparison { earlier, recent });
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practice_catalogue::PracticeFamily;
    use crate::{AssistanceLevel, AttemptKind, FocusCommand, LearningHistory, TrainingSession};

    #[test]
    fn unsupported_and_unanswered_denominators_remain_distinct_from_scored_and_skipped() {
        let mut counts = MechanismCounts::default();
        for response in [
            MechanismResponse::Unsupported,
            MechanismResponse::Unanswered,
            MechanismResponse::Skipped,
            MechanismResponse::Correct,
            MechanismResponse::DeniedPossibility,
        ] {
            counts.add(response);
        }
        assert_eq!(
            (
                counts.presented,
                counts.answered,
                counts.scored,
                counts.skipped,
                counts.unknown
            ),
            (5, 3, 2, 1, 2)
        );
        assert_eq!(
            (
                counts.correct,
                counts.denied,
                counts.unsupported,
                counts.unanswered
            ),
            (1, 1, 1, 1)
        );
    }

    #[test]
    fn summary_at_retention_limit_uses_entries_without_catalogue_reconstruction() {
        // Obtain one legal immutable template once. Construct the maximum stream
        // directly so this tests summary complexity, not existing IO validation.
        let mut template_history = LearningHistory::new();
        let mut session = TrainingSession::catalogue(
            PracticeFamily::SideCardVulnerability,
            AssistanceLevel::AnswerBeforeReveal,
            &mut template_history,
            true,
        )
        .unwrap();
        let presented = template_history.focus_entries()[0].clone();
        session
            .execute_focus(
                FocusCommand::Answer(FocusAnswer::Possible),
                &mut template_history,
                true,
            )
            .unwrap();
        let answer = template_history.focus_entries()[1].clone();
        let mut h = LearningHistory::new();
        for i in 0..crate::MAX_LEARNING_ATTEMPTS {
            let attempt = h
                .begin_attempt(AttemptKind::Training {
                    attempt: i as u32 + 1,
                })
                .unwrap();
            let mut entry = presented.clone();
            entry.attempt = attempt;
            h.focus.push(entry.clone());
            if i < 10_000 {
                entry.action = FocusAction::Help;
                entry.assisted = true;
                h.focus.push(entry.clone());
            }
            entry.action = answer.action.clone();
            h.focus.push(entry);
        }
        assert_eq!(h.focus.len(), crate::MAX_LEARNING_EVENTS);
        let rows = h.mechanism_responses();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().map(|r| r.counts.presented).sum::<u32>(), 20_000);
        assert_eq!(rows.iter().map(|r| r.counts.correct).sum::<u32>(), 20_000);
        assert_eq!(rows.iter().filter_map(|r| r.comparison).count(), 2);
    }
}
