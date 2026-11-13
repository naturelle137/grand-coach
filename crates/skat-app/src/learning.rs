// SPDX-License-Identifier: EUPL-1.2

//! Local learning history (M8 foundation, D-70): one versioned store of
//! observed counting-question events and descriptive counts over them.
//!
//! The history holds the [`RetrievalEvent`]s a [`crate::Session`] emits,
//! each tagged with a store-assigned [`AttemptId`]. It never holds cards,
//! deals or recordings. Counts describe what was observed; they are not a
//! measure of mastery, retention or transfer, and no answer is credited as
//! unaided when help was available during its opportunity.
//!
//! Deduplication: an event is identified by its attempt, its
//! [`crate::QuestionId`] and its action tag (opportunity, help, settlement;
//! an answer and a skip are both the one settlement). Recording the same
//! event again, after a reload or in a re-sent batch, changes nothing.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use skat_analysis::counting::QuestionKind;

use crate::quiz::{RetrievalAction, RetrievalEvent};

/// Format version of the learning history. Version 1 is the first one;
/// there are no older formats to migrate.
pub const LEARNING_FORMAT: u32 = 1;

/// Most events one history holds. Further events are refused, never
/// replacing older ones; [`LearningSummary::full`] reports it.
pub const MAX_LEARNING_EVENTS: usize = 50_000;

/// Most attempts one history holds.
pub const MAX_LEARNING_ATTEMPTS: usize = 20_000;

/// Below this many opportunities the overview states that the observations
/// are limited.
pub const LIMITED_OBSERVATIONS: u32 = 20;

/// Identity of one attempt within one history: assigned by
/// [`LearningHistory::begin_attempt`] from a persisted counter, so it is
/// never reused, not even after a [`LearningHistory::reset`]. Session
/// lineages and seeds are not used, because they repeat across restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AttemptId(pub u64);

/// What kind of play an attempt was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttemptKind {
    /// An ordinary series.
    Series,
    /// An isolated same-deal practice attempt (D-66).
    Practice {
        /// Attempt number within one entered practice, from 1; higher
        /// numbers are intentional retries of the same deal.
        attempt: u32,
    },
}

impl AttemptKind {
    /// The overview context of this kind.
    pub fn context(self) -> LearningContext {
        match self {
            AttemptKind::Series => LearningContext::Series,
            AttemptKind::Practice { attempt: 0 | 1 } => LearningContext::Practice,
            AttemptKind::Practice { .. } => LearningContext::PracticeRetry,
        }
    }
}

/// One attempt of the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningAttempt {
    /// Its identity.
    pub id: AttemptId,
    /// Its kind.
    pub kind: AttemptKind,
}

/// One recorded event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningEntry {
    /// The attempt it was observed in.
    pub attempt: AttemptId,
    /// The event exactly as the session emitted it.
    pub event: RetrievalEvent,
}

/// Why a history is not valid, or why an operation was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningError {
    /// Another format version than [`LEARNING_FORMAT`].
    UnsupportedFormat,
    /// More than [`MAX_LEARNING_EVENTS`] or [`MAX_LEARNING_ATTEMPTS`].
    TooLarge,
    /// An attempt ID is zero, repeated or not below the counter.
    InvalidAttempt,
    /// An event names an attempt the history does not contain.
    UnknownAttempt,
    /// The same event (attempt, question ID, action tag) appears twice.
    DuplicateEvent,
    /// The history is full; nothing was recorded.
    Full,
}

/// The store. Plain data; file access is in [`LearningHistory::open_local`]
/// and [`LearningHistory::save_local`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningHistory {
    format: u32,
    /// Next attempt ID; survives [`LearningHistory::reset`].
    next_attempt: u64,
    attempts: Vec<LearningAttempt>,
    events: Vec<LearningEntry>,
    /// Deduplication keys of `events`; rebuilt by [`LearningHistory::validate`].
    #[serde(skip)]
    keys: HashSet<EventKey>,
    /// Events were refused because the history was full (not persisted).
    #[serde(skip)]
    refused: bool,
}

impl Default for LearningHistory {
    fn default() -> Self {
        LearningHistory::new()
    }
}

/// Deduplication key of one event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EventKey {
    attempt: AttemptId,
    series: u64,
    game: u32,
    trick: u8,
    tag: u8,
}

impl EventKey {
    fn of(attempt: AttemptId, event: &RetrievalEvent) -> EventKey {
        EventKey {
            attempt,
            series: event.id.series,
            game: event.id.game,
            trick: event.id.trick,
            tag: match event.action {
                RetrievalAction::Opportunity => 0,
                RetrievalAction::Help => 1,
                RetrievalAction::Answer { .. } | RetrievalAction::Skip { .. } => 2,
            },
        }
    }
}

/// What [`LearningHistory::record`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    /// New events stored.
    pub added: usize,
    /// Events already stored, ignored.
    pub duplicates: usize,
}

impl LearningHistory {
    /// An empty history.
    pub fn new() -> LearningHistory {
        LearningHistory {
            format: LEARNING_FORMAT,
            next_attempt: 1,
            attempts: Vec::new(),
            events: Vec::new(),
            keys: HashSet::new(),
            refused: false,
        }
    }

    /// Starts a new attempt and returns its fresh identity.
    pub fn begin_attempt(&mut self, kind: AttemptKind) -> Result<AttemptId, LearningError> {
        if self.attempts.len() >= MAX_LEARNING_ATTEMPTS || self.next_attempt == u64::MAX {
            self.refused = true;
            return Err(LearningError::Full);
        }
        let id = AttemptId(self.next_attempt);
        self.next_attempt += 1;
        self.attempts.push(LearningAttempt { id, kind });
        Ok(id)
    }

    /// Records the `events` of `attempt`, skipping those already stored.
    /// Nothing is recorded if the attempt is unknown or the new events would
    /// exceed [`MAX_LEARNING_EVENTS`].
    pub fn record<'a>(
        &mut self,
        attempt: AttemptId,
        events: impl IntoIterator<Item = &'a RetrievalEvent>,
    ) -> Result<Recorded, LearningError> {
        if !self.attempts.iter().rev().any(|a| a.id == attempt) {
            return Err(LearningError::UnknownAttempt);
        }
        let mut new = Vec::new();
        let mut seen = HashSet::new();
        let mut duplicates = 0;
        for event in events {
            let key = EventKey::of(attempt, event);
            if self.keys.contains(&key) || !seen.insert(key) {
                duplicates += 1;
            } else {
                new.push(LearningEntry {
                    attempt,
                    event: *event,
                });
            }
        }
        if self.events.len() + new.len() > MAX_LEARNING_EVENTS {
            self.refused = true;
            return Err(LearningError::Full);
        }
        self.keys.extend(seen);
        let added = new.len();
        self.events.extend(new);
        Ok(Recorded { added, duplicates })
    }

    /// Deliberately forgets every attempt and event. The attempt counter is
    /// kept, so no identity issued before is ever issued again.
    pub fn reset(&mut self) {
        self.attempts.clear();
        self.events.clear();
        self.keys.clear();
        self.refused = false;
    }

    /// The attempts in order of creation.
    pub fn attempts(&self) -> &[LearningAttempt] {
        &self.attempts
    }

    /// The events in order of recording.
    pub fn events(&self) -> &[LearningEntry] {
        &self.events
    }

    /// `true` without any event.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Checks a decoded history and rebuilds its deduplication index.
    pub(crate) fn validate(&mut self) -> Result<(), LearningError> {
        if self.format != LEARNING_FORMAT {
            return Err(LearningError::UnsupportedFormat);
        }
        if self.events.len() > MAX_LEARNING_EVENTS || self.attempts.len() > MAX_LEARNING_ATTEMPTS {
            return Err(LearningError::TooLarge);
        }
        let mut ids = HashSet::new();
        for a in &self.attempts {
            if a.id.0 == 0 || a.id.0 >= self.next_attempt || !ids.insert(a.id) {
                return Err(LearningError::InvalidAttempt);
            }
        }
        let mut keys = HashSet::new();
        for e in &self.events {
            if !ids.contains(&e.attempt) {
                return Err(LearningError::UnknownAttempt);
            }
            if !keys.insert(EventKey::of(e.attempt, &e.event)) {
                return Err(LearningError::DuplicateEvent);
            }
        }
        self.keys = keys;
        self.refused = false;
        Ok(())
    }

    /// Descriptive counts per topic and context.
    pub fn summary(&self) -> LearningSummary {
        let mut rows: Vec<LearningRow> = Vec::new();
        let mut totals = RetrievalCounts::default();
        let mut used = HashSet::new();
        let kinds: HashMap<AttemptId, AttemptKind> =
            self.attempts.iter().map(|a| (a.id, a.kind)).collect();
        for e in &self.events {
            let Some(&kind) = kinds.get(&e.attempt) else {
                continue;
            };
            used.insert(e.attempt);
            let topic = LearningTopic::of(e.event.question);
            let context = kind.context();
            let row = match rows
                .iter_mut()
                .find(|r| r.topic == topic && r.context == context)
            {
                Some(row) => row,
                None => {
                    rows.push(LearningRow {
                        topic,
                        context,
                        counts: RetrievalCounts::default(),
                    });
                    rows.last_mut().expect("just pushed")
                }
            };
            row.counts.add(&e.event);
            totals.add(&e.event);
        }
        rows.sort_by_key(|r| (r.topic, r.context));
        let observations = match totals.opportunities {
            0 => Observations::None,
            n if n < LIMITED_OBSERVATIONS => Observations::Limited,
            _ => Observations::Several,
        };
        LearningSummary {
            rows,
            totals,
            attempts: used.len() as u32,
            observations,
            full: self.refused
                || self.events.len() >= MAX_LEARNING_EVENTS
                || self.attempts.len() >= MAX_LEARNING_ATTEMPTS,
        }
    }
}

/// Topic of a counting question: its question family, without the class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LearningTopic {
    /// Remaining Trumpf.
    TrumpfOutside,
    /// Publicly proven voids.
    ProvenVoids,
    /// Augen won.
    AugenWon,
    /// Cards of a class played.
    ClassPlayed,
}

impl LearningTopic {
    /// The topic of a question.
    pub fn of(kind: QuestionKind) -> LearningTopic {
        match kind {
            QuestionKind::TrumpfOutside => LearningTopic::TrumpfOutside,
            QuestionKind::ProvenVoids { .. } => LearningTopic::ProvenVoids,
            QuestionKind::AugenWon { .. } => LearningTopic::AugenWon,
            QuestionKind::ClassPlayed { .. } => LearningTopic::ClassPlayed,
        }
    }
}

/// Where observations were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LearningContext {
    /// An ordinary series.
    Series,
    /// The first attempt of an entered same-deal practice.
    Practice,
    /// An intentional retry of the same deal.
    PracticeRetry,
}

/// Counts of observed events. Each field counts its own event type, so
/// opportunities are never inferred from answers or the other way round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalCounts {
    /// Questions offered.
    pub opportunities: u32,
    /// Explicit help restorations (VisibleAid opportunities start with
    /// help and produce none).
    pub help: u32,
    /// Numeric answers.
    pub answers: u32,
    /// Exact answers.
    pub correct: u32,
    /// Answers without help available during the opportunity.
    pub unaided_answers: u32,
    /// Exact answers among them.
    pub unaided_correct: u32,
    /// Skips, including switching the quiz off.
    pub skips: u32,
}

impl RetrievalCounts {
    fn add(&mut self, e: &RetrievalEvent) {
        match e.action {
            RetrievalAction::Opportunity => self.opportunities += 1,
            RetrievalAction::Help => self.help += 1,
            RetrievalAction::Answer { correct, .. } => {
                self.answers += 1;
                self.correct += u32::from(correct);
                if !e.assisted {
                    self.unaided_answers += 1;
                    self.unaided_correct += u32::from(correct);
                }
            }
            RetrievalAction::Skip { .. } => self.skips += 1,
        }
    }
}

/// Counts of one topic in one context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningRow {
    /// The topic.
    pub topic: LearningTopic,
    /// The context.
    pub context: LearningContext,
    /// The counts.
    pub counts: RetrievalCounts,
}

/// How much was observed overall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observations {
    /// Nothing yet.
    None,
    /// Fewer than [`LIMITED_OBSERVATIONS`] opportunities.
    Limited,
    /// At least [`LIMITED_OBSERVATIONS`] opportunities. Still only counts.
    Several,
}

/// The overview of a history. Descriptive only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningSummary {
    /// Rows with observations, ordered by topic, then context.
    pub rows: Vec<LearningRow>,
    /// All rows together.
    pub totals: RetrievalCounts,
    /// Attempts with at least one event.
    pub attempts: u32,
    /// Sample size category.
    pub observations: Observations,
    /// New events are or were refused because the history is full.
    pub full: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quiz::{AssistanceLevel, QuestionId};

    fn event(trick: u8, action: RetrievalAction, assisted: bool) -> RetrievalEvent {
        RetrievalEvent {
            id: QuestionId {
                series: 9,
                game: 1,
                trick,
            },
            question: QuestionKind::TrumpfOutside,
            action,
            assistance: AssistanceLevel::AnswerBeforeReveal,
            assisted,
        }
    }

    #[test]
    fn answer_and_skip_share_one_settlement_key() {
        let mut h = LearningHistory::new();
        let a = h.begin_attempt(AttemptKind::Series).unwrap();
        let answer = event(
            3,
            RetrievalAction::Answer {
                given: 2,
                correct: true,
            },
            false,
        );
        let skip = event(3, RetrievalAction::Skip { disabled: false }, false);
        assert_eq!(h.record(a, [&answer]).unwrap().added, 1);
        assert_eq!(h.record(a, [&skip]).unwrap().duplicates, 1);
        assert_eq!(h.summary().totals.skips, 0);
    }

    #[test]
    fn a_full_history_refuses_without_dropping_and_reset_keeps_the_counter() {
        let mut h = LearningHistory::new();
        let a = h.begin_attempt(AttemptKind::Series).unwrap();
        let events: Vec<_> = (0..=u8::MAX)
            .map(|t| event(t, RetrievalAction::Opportunity, false))
            .collect();
        h.record(a, &events).unwrap();
        // Fill to the limit with other games.
        while h.events.len() + events.len() <= MAX_LEARNING_EVENTS {
            let mut more = events.clone();
            let game = h.events.len() as u32;
            for e in &mut more {
                e.id.game = game;
            }
            h.record(a, &more).unwrap();
        }
        let before = h.events.len();
        let mut last = events.clone();
        for e in &mut last {
            e.id.game = u32::MAX;
        }
        assert_eq!(h.record(a, &last), Err(LearningError::Full));
        assert_eq!(h.events.len(), before);
        assert!(h.summary().full);
        h.reset();
        assert!(h.is_empty() && !h.summary().full);
        assert_eq!(h.begin_attempt(AttemptKind::Series), Ok(AttemptId(2)));
        assert_eq!(h.record(a, &events), Err(LearningError::UnknownAttempt));
    }
}
