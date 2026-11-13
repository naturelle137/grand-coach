// SPDX-License-Identifier: EUPL-1.2

//! Local learning history (M8 foundation, D-70): one versioned store of
//! observed counting-question events and descriptive counts over them.
//!
//! The history holds the [`RetrievalEvent`]s a [`crate::Session`] emits,
//! each tagged with a store-assigned [`AttemptId`]. Catalogue entries retain
//! bounded permitted-view scoring provenance, never full deals, other hands
//! or recordings. Counts describe what was observed; they are not a
//! measure of mastery, retention or transfer, and no answer is credited as
//! unaided when help was available during its opportunity.
//!
//! Deduplication: an event is identified by its attempt, its
//! [`crate::OpportunityId`] and its action tag (opportunity, help,
//! settlement; an answer and a skip are both the one settlement). Recording
//! the same event again, after a reload or in a re-sent batch, changes
//! nothing.
//!
//! Formats: version 2 adds focused-training attempts and task identities
//! (D-71). Version 3 adds provenance to new events: the immutable lesson
//! family and version of each opportunity and, with an answer only, the
//! exact count it was scored against. Version 1 and 2 files are read and
//! migrated in memory: counter, attempts, events and their order are kept
//! exactly, their events stay unversioned and their wrong answers keep an
//! unknown deviation; format 4 additionally holds typed catalogue transitions
//! and conservative persisted exposure. The next save writes version 4.
//!
//! Response categories ([`ResponseCategory`]) describe only the observed
//! counting response against the recorded exact count. They never name a
//! reason, a strategic mistake, mastery or transfer; strategic decision
//! classification is not part of this history yet.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use skat_analysis::counting::QuestionKind;

use crate::quiz::{LessonRevision, OpportunityId, RetrievalAction, RetrievalEvent};

/// Format version of the learning history written by this build. Versions
/// 1 (series and practice only) and 2 (without provenance) are read and
/// migrated; see [`LEARNING_FORMATS_READ`].
pub const LEARNING_FORMAT: u32 = 4;

/// Format versions this build reads: 1, 2, 3 (migrated) and 4.
pub const LEARNING_FORMATS_READ: [u32; 4] = [1, 2, 3, LEARNING_FORMAT];

/// Most events one history holds. Further events are refused, never
/// replacing older ones; [`LearningSummary::full`] reports it.
pub const MAX_LEARNING_EVENTS: usize = 50_000;

/// Most attempts one history holds.
pub const MAX_LEARNING_ATTEMPTS: usize = 20_000;

/// Below this many opportunities the overview states that the observations
/// are limited.
pub const LIMITED_OBSERVATIONS: u32 = 20;

/// Settled responses per window of an earlier/recent comparison. A topic
/// needs twice as many before any comparison is shown.
pub const COMPARISON_WINDOW: u32 = 10;

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
    /// One presentation of a focused-training task (D-71); its events name
    /// the fixture and version. Never a series or a same-deal practice.
    Training {
        /// Presentation of this fixture within one entered training, from
        /// 1; higher numbers are deliberate retries or repeats.
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
            AttemptKind::Training { attempt: 0 | 1 } => LearningContext::Training,
            AttemptKind::Training { .. } => LearningContext::TrainingRepeat,
        }
    }

    /// Whether `id` can belong to an attempt of this kind: training tasks
    /// only to training attempts, game questions only to the others.
    fn admits(self, id: &OpportunityId) -> bool {
        matches!(self, AttemptKind::Training { .. }) == matches!(id, OpportunityId::Training(_))
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
    /// A format version not in [`LEARNING_FORMATS_READ`].
    UnsupportedFormat,
    /// More than [`MAX_LEARNING_EVENTS`] or [`MAX_LEARNING_ATTEMPTS`].
    TooLarge,
    /// An attempt ID is zero, repeated or not below the counter.
    InvalidAttempt,
    /// An event names an attempt the history does not contain.
    UnknownAttempt,
    /// The same event (attempt, opportunity ID, action tag) appears twice.
    DuplicateEvent,
    /// An event's opportunity does not fit its attempt kind (a training
    /// task outside a training attempt or the other way round), or a
    /// format 1 history contains training content.
    InvalidContext,
    /// Provenance is missing or inconsistent: a new event without its
    /// lesson revision or a new answer without its exact count, a lesson
    /// that does not fit the opportunity or changes within it, an exact
    /// count that contradicts the recorded score, or provenance in a
    /// format 1 or 2 history.
    InvalidProvenance,
    /// The history is full; nothing was recorded.
    Full,
}

/// The store. Plain data; file access is in [`LearningHistory::open_local`]
/// and [`LearningHistory::save_local`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningHistory {
    format: u32,
    #[serde(default)]
    pub(crate) focus: Vec<crate::training::FocusEntry>,
    #[serde(default)]
    pub(crate) exposure_complete: bool,
    /// Next attempt ID; survives [`LearningHistory::reset`].
    next_attempt: u64,
    pub(crate) attempts: Vec<LearningAttempt>,
    pub(crate) events: Vec<LearningEntry>,
    /// Deduplication keys of `events`; rebuilt by [`LearningHistory::validate`].
    #[serde(skip)]
    keys: HashSet<EventKey>,
    /// Lesson revision of each stored opportunity; rebuilt with `keys`.
    #[serde(skip)]
    lessons: HashMap<Opportunity, Option<LessonRevision>>,
    /// Events were refused because the history was full (not persisted).
    #[serde(skip)]
    pub(crate) refused: bool,
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
    id: OpportunityId,
    tag: u8,
}

impl EventKey {
    fn of(attempt: AttemptId, event: &RetrievalEvent) -> EventKey {
        EventKey {
            attempt,
            id: event.id,
            tag: match event.action {
                RetrievalAction::Opportunity => 0,
                RetrievalAction::Help => 1,
                RetrievalAction::Answer { .. } | RetrievalAction::Skip { .. } => 2,
            },
        }
    }
}

/// One opportunity of one attempt.
type Opportunity = (AttemptId, OpportunityId);

/// Checks one event's own provenance: a lesson must fit the opportunity, an
/// exact count must agree with the score. With `fresh` (newly recorded
/// events), both must be present.
fn check_provenance(event: &RetrievalEvent, fresh: bool) -> Result<(), LearningError> {
    let lesson = match event.lesson {
        Some(lesson) => lesson.family.issues(&event.id) && lesson.version > 0,
        None => !fresh,
    };
    let expected = match event.action {
        RetrievalAction::Answer {
            given,
            correct,
            expected: Some(expected),
        } => correct == (given == expected),
        RetrievalAction::Answer { expected: None, .. } => !fresh,
        _ => true,
    };
    if lesson && expected {
        Ok(())
    } else {
        Err(LearningError::InvalidProvenance)
    }
}

/// Checks that `event` keeps the lesson already known for its opportunity
/// in `known` or `batch`, and notes it in `batch`.
fn check_lesson(
    known: &HashMap<Opportunity, Option<LessonRevision>>,
    batch: &mut HashMap<Opportunity, Option<LessonRevision>>,
    attempt: AttemptId,
    event: &RetrievalEvent,
) -> Result<(), LearningError> {
    let key = (attempt, event.id);
    match known.get(&key).or_else(|| batch.get(&key)) {
        Some(&lesson) if lesson != event.lesson => Err(LearningError::InvalidProvenance),
        Some(_) => Ok(()),
        None => {
            batch.insert(key, event.lesson);
            Ok(())
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
            focus: Vec::new(),
            exposure_complete: true,
            next_attempt: 1,
            attempts: Vec::new(),
            events: Vec::new(),
            keys: HashSet::new(),
            lessons: HashMap::new(),
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
    /// Nothing is recorded if the attempt is unknown, an event does not fit
    /// its kind ([`LearningError::InvalidContext`]), lacks or contradicts
    /// its provenance ([`LearningError::InvalidProvenance`]) or the new
    /// events would exceed [`MAX_LEARNING_EVENTS`].
    pub fn record<'a>(
        &mut self,
        attempt: AttemptId,
        events: impl IntoIterator<Item = &'a RetrievalEvent>,
    ) -> Result<Recorded, LearningError> {
        let Some(kind) = self
            .attempts
            .iter()
            .rev()
            .find(|a| a.id == attempt)
            .map(|a| a.kind)
        else {
            return Err(LearningError::UnknownAttempt);
        };
        let mut new = Vec::new();
        let mut seen = HashSet::new();
        let mut lessons = HashMap::new();
        let mut duplicates = 0;
        for event in events {
            if !kind.admits(&event.id) {
                return Err(LearningError::InvalidContext);
            }
            let key = EventKey::of(attempt, event);
            if self.keys.contains(&key) || seen.contains(&key) {
                // Already stored: its stored provenance stays.
                duplicates += 1;
            } else {
                check_provenance(event, true)?;
                check_lesson(&self.lessons, &mut lessons, attempt, event)?;
                seen.insert(key);
                new.push(LearningEntry {
                    attempt,
                    event: *event,
                });
            }
        }
        if self.events.len() + self.focus.len() + new.len() > MAX_LEARNING_EVENTS {
            self.refused = true;
            return Err(LearningError::Full);
        }
        self.keys.extend(seen);
        self.lessons.extend(lessons);
        let added = new.len();
        self.events.extend(new);
        Ok(Recorded { added, duplicates })
    }

    /// Deliberately forgets every attempt and event. The attempt counter is
    /// kept, so no identity issued before is ever issued again.
    pub fn reset(&mut self) {
        self.focus.clear();
        self.exposure_complete = false;
        self.attempts.clear();
        self.events.clear();
        self.keys.clear();
        self.lessons.clear();
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
        self.events.is_empty() && self.focus.is_empty()
    }

    /// Migrates a decoded history of an older supported format to
    /// [`LEARNING_FORMAT`], checks it and rebuilds its deduplication index.
    /// Formats 1 and 2 keep their counter, attempts and events exactly and
    /// stay unversioned: provenance in them is
    /// [`LearningError::InvalidProvenance`]. Format 1 knows no training:
    /// training content in it is [`LearningError::InvalidContext`].
    pub(crate) fn validate(&mut self) -> Result<(), LearningError> {
        if self.format < LEARNING_FORMAT && (!self.focus.is_empty() || self.exposure_complete) {
            return Err(LearningError::InvalidProvenance);
        }
        match self.format {
            1 | 2 => {
                let training = self
                    .attempts
                    .iter()
                    .any(|a| matches!(a.kind, AttemptKind::Training { .. }))
                    || self
                        .events
                        .iter()
                        .any(|e| matches!(e.event.id, OpportunityId::Training(_)));
                if self.format == 1 && training {
                    return Err(LearningError::InvalidContext);
                }
                let provenance = self.events.iter().any(|e| {
                    e.event.lesson.is_some()
                        || matches!(
                            e.event.action,
                            RetrievalAction::Answer {
                                expected: Some(_),
                                ..
                            }
                        )
                });
                if provenance {
                    return Err(LearningError::InvalidProvenance);
                }
                self.format = LEARNING_FORMAT;
            }
            3 => {
                self.exposure_complete = false;
                self.format = LEARNING_FORMAT;
            }
            LEARNING_FORMAT => {}
            _ => return Err(LearningError::UnsupportedFormat),
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
        let kinds: HashMap<AttemptId, AttemptKind> =
            self.attempts.iter().map(|a| (a.id, a.kind)).collect();
        let mut keys = HashSet::new();
        let mut lessons = HashMap::new();
        for e in &self.events {
            let Some(kind) = kinds.get(&e.attempt) else {
                return Err(LearningError::UnknownAttempt);
            };
            if !kind.admits(&e.event.id) {
                return Err(LearningError::InvalidContext);
            }
            check_provenance(&e.event, false)?;
            check_lesson(&HashMap::new(), &mut lessons, e.attempt, &e.event)?;
            if !keys.insert(EventKey::of(e.attempt, &e.event)) {
                return Err(LearningError::DuplicateEvent);
            }
        }
        self.validate_focus()?;
        self.keys = keys;
        self.lessons = lessons;
        self.refused = false;
        Ok(())
    }

    /// Descriptive counts per topic and context, response categories, the
    /// lesson revisions of the opportunities and earlier/recent
    /// comparisons in persisted order.
    pub fn summary(&self) -> LearningSummary {
        let mut rows: Vec<LearningRow> = Vec::new();
        let mut totals = RetrievalCounts::default();
        let mut responses = ResponseCounts::default();
        let mut provenance: Vec<ProvenanceCount> = Vec::new();
        let mut settled: Vec<(LearningTopic, AttemptId, LearningContext, &RetrievalEvent)> =
            Vec::new();
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
                        responses: ResponseCounts::default(),
                    });
                    rows.last_mut().expect("just pushed")
                }
            };
            row.counts.add(&e.event);
            totals.add(&e.event);
            if row.responses.add(&e.event) {
                responses.add(&e.event);
                settled.push((topic, e.attempt, context, &e.event));
            }
            if e.event.action == RetrievalAction::Opportunity {
                match provenance.iter_mut().find(|p| p.lesson == e.event.lesson) {
                    Some(p) => p.opportunities += 1,
                    None => provenance.push(ProvenanceCount {
                        lesson: e.event.lesson,
                        opportunities: 1,
                    }),
                }
            }
        }
        rows.sort_by_key(|r| (r.topic, r.context));
        provenance.sort_by_key(|p| p.lesson);
        let mut comparisons: Vec<TopicComparison> = Vec::new();
        for topic in rows.iter().map(|r| r.topic) {
            if comparisons.last().is_some_and(|c| c.topic == topic) {
                continue;
            }
            let of_topic: Vec<_> = settled.iter().filter(|s| s.0 == topic).collect();
            if of_topic.is_empty() {
                continue;
            }
            let n = of_topic.len();
            let w = COMPARISON_WINDOW as usize;
            let window = |start: usize| {
                let mut out = ResponseWindow {
                    first: start as u32 + 1,
                    last: (start + w) as u32,
                    lowest_attempt: of_topic[start].1,
                    highest_attempt: of_topic[start].1,
                    responses: ResponseCounts::default(),
                    contexts: Vec::new(),
                };
                for &&(_, attempt, context, event) in &of_topic[start..start + w] {
                    out.lowest_attempt = out.lowest_attempt.min(attempt);
                    out.highest_attempt = out.highest_attempt.max(attempt);
                    out.responses.add(event);
                    match out.contexts.iter_mut().find(|c| c.context == context) {
                        Some(c) => c.settled += 1,
                        None => out.contexts.push(ContextCount {
                            context,
                            settled: 1,
                        }),
                    }
                }
                out.contexts.sort_by_key(|c| c.context);
                out
            };
            comparisons.push(TopicComparison {
                topic,
                settled: n as u32,
                windows: (n >= 2 * w).then(|| [window(n - 2 * w), window(n - w)]),
            });
        }
        used.extend(self.focus.iter().map(|e| e.attempt));
        let observations = match totals.opportunities {
            0 => Observations::None,
            n if n < LIMITED_OBSERVATIONS => Observations::Limited,
            _ => Observations::Several,
        };
        LearningSummary {
            rows,
            totals,
            responses,
            provenance,
            comparisons,
            attempts: used.len() as u32,
            observations,
            full: self.refused
                || self.events.len() + self.focus.len() >= MAX_LEARNING_EVENTS
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
    /// The first presentation of a fixed training task in one entered
    /// training. Still practice with a known fixture, never transfer.
    Training,
    /// A deliberate retry or repeat of a training task.
    TrainingRepeat,
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

/// Observed category of one settled counting response (an answer or a
/// skip), judged only against the exact count recorded with it. It says
/// what was answered, never why, and is no strategic error, mastery or
/// transfer measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResponseCategory {
    /// The answer equals the exact count.
    Exact,
    /// The answer is above the recorded exact count.
    TooHigh,
    /// The answer is below the recorded exact count.
    TooLow,
    /// The answer differed, but its exact count was not recorded (before
    /// format 3): the deviation is unknown and not reconstructed.
    MismatchUnrecorded,
    /// No answer.
    Skipped,
}

impl ResponseCategory {
    /// The category of a settling action; `None` for an opportunity or
    /// help, which settle nothing.
    pub fn of(action: RetrievalAction) -> Option<ResponseCategory> {
        Some(match action {
            RetrievalAction::Opportunity | RetrievalAction::Help => return None,
            RetrievalAction::Answer { correct: true, .. } => ResponseCategory::Exact,
            RetrievalAction::Answer {
                given,
                expected: Some(expected),
                ..
            } if given > expected => ResponseCategory::TooHigh,
            RetrievalAction::Answer {
                expected: Some(_), ..
            } => ResponseCategory::TooLow,
            RetrievalAction::Answer { expected: None, .. } => ResponseCategory::MismatchUnrecorded,
            RetrievalAction::Skip { .. } => ResponseCategory::Skipped,
        })
    }
}

/// Settled responses per [`ResponseCategory`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryCounts {
    /// [`ResponseCategory::Exact`].
    pub exact: u32,
    /// [`ResponseCategory::TooHigh`].
    pub too_high: u32,
    /// [`ResponseCategory::TooLow`].
    pub too_low: u32,
    /// [`ResponseCategory::MismatchUnrecorded`].
    pub mismatch_unrecorded: u32,
    /// [`ResponseCategory::Skipped`].
    pub skipped: u32,
}

impl CategoryCounts {
    /// All settled responses counted.
    pub fn total(&self) -> u32 {
        self.exact + self.too_high + self.too_low + self.mismatch_unrecorded + self.skipped
    }
}

/// Settled responses by support: without help available during the
/// opportunity, and with help (visible aid or a request, sticky).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseCounts {
    /// Without help.
    pub unaided: CategoryCounts,
    /// With help.
    pub assisted: CategoryCounts,
}

impl ResponseCounts {
    /// Counts `e` if it settles its opportunity; returns whether it did.
    fn add(&mut self, e: &RetrievalEvent) -> bool {
        let Some(category) = ResponseCategory::of(e.action) else {
            return false;
        };
        let counts = if e.assisted {
            &mut self.assisted
        } else {
            &mut self.unaided
        };
        *match category {
            ResponseCategory::Exact => &mut counts.exact,
            ResponseCategory::TooHigh => &mut counts.too_high,
            ResponseCategory::TooLow => &mut counts.too_low,
            ResponseCategory::MismatchUnrecorded => &mut counts.mismatch_unrecorded,
            ResponseCategory::Skipped => &mut counts.skipped,
        } += 1;
        true
    }

    /// All settled responses.
    pub fn settled(&self) -> u32 {
        self.unaided.total() + self.assisted.total()
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
    /// Its settled responses by category and support.
    pub responses: ResponseCounts,
}

/// Opportunities recorded under one lesson revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceCount {
    /// The revision; `None`: recorded before format 3, version unknown.
    pub lesson: Option<LessonRevision>,
    /// Opportunities.
    pub opportunities: u32,
}

/// Settled responses of one context within a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCount {
    /// The context.
    pub context: LearningContext,
    /// Settled responses.
    pub settled: u32,
}

/// [`COMPARISON_WINDOW`] consecutive settled responses of one topic, in
/// the order they were recorded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseWindow {
    /// Position of its first response among the topic's settled
    /// responses, from 1 (oldest recorded).
    pub first: u32,
    /// Position of its last response.
    pub last: u32,
    /// Lowest attempt ID among its responses.
    pub lowest_attempt: AttemptId,
    /// Highest attempt ID among its responses.
    pub highest_attempt: AttemptId,
    /// Its responses by category and support.
    pub responses: ResponseCounts,
    /// Its responses per context, ordered by context.
    pub contexts: Vec<ContextCount>,
}

/// Earlier and recent recorded responses of one topic side by side. Only
/// a description of two windows: no trend, retention or transfer claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicComparison {
    /// The topic.
    pub topic: LearningTopic,
    /// All its settled responses.
    pub settled: u32,
    /// The [`COMPARISON_WINDOW`] responses before the last ones, then the
    /// last ones; `None` below twice the window.
    pub windows: Option<[ResponseWindow; 2]>,
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
    /// All settled responses by category and support.
    pub responses: ResponseCounts,
    /// Opportunities per lesson revision, unversioned first.
    pub provenance: Vec<ProvenanceCount>,
    /// Topics with settled responses, ordered by topic.
    pub comparisons: Vec<TopicComparison>,
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
    use crate::quiz::{AssistanceLevel, LessonFamily, QuestionId};

    fn event(trick: u8, action: RetrievalAction, assisted: bool) -> RetrievalEvent {
        RetrievalEvent {
            assisted,
            ..event_in(1, trick, action)
        }
    }

    fn event_in(game: u32, trick: u8, action: RetrievalAction) -> RetrievalEvent {
        RetrievalEvent {
            id: QuestionId {
                series: 9,
                game,
                trick,
            }
            .into(),
            question: QuestionKind::TrumpfOutside,
            action,
            assistance: AssistanceLevel::AnswerBeforeReveal,
            assisted: false,
            lesson: Some(LessonFamily::FocusedRetrieval.current()),
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
                expected: Some(2),
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
            for (t, e) in more.iter_mut().enumerate() {
                *e = event_in(game, t as u8, RetrievalAction::Opportunity);
            }
            h.record(a, &more).unwrap();
        }
        let before = h.events.len();
        let mut last = events.clone();
        for (t, e) in last.iter_mut().enumerate() {
            *e = event_in(u32::MAX, t as u8, RetrievalAction::Opportunity);
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
