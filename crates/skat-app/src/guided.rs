// SPDX-License-Identifier: EUPL-1.2

//! Explicit, decision-scoped dialogue. No storage and no asynchronous rule engine.

use crate::coach::{CoachResult, DecisionKey};
use crate::{AppError, CoachLevel, Session};
use serde::{Deserialize, Serialize};
use skat_coach::guided::{self, BidLesson, Lesson, Memory, Observation, PreLesson};
pub use skat_coach::guided::{
    Answer as GuidedAnswer, BidRule as GuidedBidRule, Branch as GuidedBranch,
    GameBranch as GuidedGameBranch, Keep as GuidedKeep, PreRule as GuidedPreRule,
    Step as GuidedStep,
};
use skat_core::{Action, Event, PlayerView, Seat};

/// Unforgeable-in-practice session/decision identity; checked on every response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedId {
    lineage: u64,
    /// Original one-based game number.
    pub game: u32,
    /// Actions before the original decision.
    pub actions: usize,
    generation: u64,
}

impl From<DecisionKey> for GuidedId {
    fn from(k: DecisionKey) -> Self {
        Self {
            lineage: k.lineage,
            game: k.number,
            actions: k.actions,
            generation: 0,
        }
    }
}

/// User transition; never a game action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuidedAction {
    /// Check an answer to the current prompt.
    Answer(GuidedAnswer),
    /// Deliberately reveal the worked rule (or observation) answer.
    Reveal,
    /// Retry the current prompt after a wrong answer.
    Retry,
    /// Open/close memory, only after the worked answer is allowed.
    Memory(bool),
    /// Request estimates separately; still requires enabled figures.
    Figures,
    /// Skip/end the dialogue and return to free play immediately.
    Return,
}

/// In-memory event handoff for a future progress consumer; no storage or mastery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedEvent {
    /// Original decision and unique entry identity.
    pub id: GuidedId,
    /// Monotonic within this dialogue entry; deduplicate by (id, sequence).
    pub sequence: u32,
    /// Explicit user transition; none means entry or actual play observation.
    pub action: Option<GuidedAction>,
    /// State after the transition.
    pub step: GuidedStep,
    /// A worked answer has been exposed; sticky for the entry.
    pub assisted: bool,
}

/// Public Reizen context, plus the orientation answer once given or revealed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedReizen {
    /// Public current bid (0: nothing bid yet), never a pre-jump value.
    pub current: u16,
    /// After orientation: whether this seat holds rather than names a bid.
    pub holds: Option<bool>,
    /// After orientation: the value at stake (none above the highest value).
    pub stake: Option<u16>,
}

/// Skataufnahme/Drücken context; the candidate game only after orientation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedPreplay {
    /// Skataufnahme or Drücken.
    pub phase: skat_core::Phase,
    /// After orientation: the candidate game of Easy's rule.
    pub game: Option<skat_core::GameType>,
    /// Skataufnahme after orientation: the strength Easy needs for Hand.
    pub threshold: Option<i32>,
    /// Drücken: the own twelve cards as neutral answer controls.
    pub cards: Vec<skat_core::Card>,
}

/// Frontend view: answer-bearing fields withheld until explicitly allowed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuidedView {
    /// Original decision; estimates and later observation keep this identity.
    pub id: GuidedId,
    /// Only the active step.
    pub step: GuidedStep,
    /// Immediate branch task after orientation; contains no selected card.
    pub task: Option<GuidedBranch>,
    /// Last attempt outcome; wrong answers do not advance.
    pub correct: Option<bool>,
    /// Rule after deliberate reveal only.
    pub rule: Option<skat_coach::guided::CardRule>,
    /// Public source for orientation feedback, only after an attempt/reveal.
    pub evidence: Option<skat_coach::Source>,
    /// Optional collapsed memory; withheld before the rule answer.
    pub memory: Option<Memory>,
    /// Actual card source; its deduction is hidden while Observe is active.
    pub observed_source: Option<skat_coach::Source>,
    /// Observation feedback after answer or deliberate reveal.
    pub observation: Option<Observation>,
    /// Separately requested original-decision estimates, never rule reasoning.
    pub figures: Option<skat_coach::Hints>,
    /// Original analysis budget if estimates are available.
    pub budget: Option<crate::Budget>,
    /// Estimates requested but unavailable/incomplete work has no fabricated numbers.
    pub figures_requested: bool,
    /// Explicit basis of the separately requested model comparison.
    pub basis: crate::CoachBasis,
    /// Frozen original permitted view for neutral card-answer controls.
    pub legal_cards: Vec<skat_core::Card>,
    /// Reizen dialogue only: public bid and, after orientation, the stake.
    pub reizen: Option<GuidedReizen>,
    /// Easy's bidding rule after a correct threshold attempt or deliberate reveal.
    pub bid_rule: Option<GuidedBidRule>,
    /// Skataufnahme/Drücken dialogue only: phase and, after orientation, the game.
    pub preplay: Option<GuidedPreplay>,
    /// Easy's Hand/pickup or discard rule after a correct attempt or reveal.
    pub pre_rule: Option<GuidedPreRule>,
}

#[derive(Clone, Debug)]
pub(crate) struct GuidedState {
    key: DecisionKey,
    id: GuidedId,
    lesson: Option<Lesson>,
    bid: Option<BidLesson>,
    pre: Option<PreLesson>,
    step: GuidedStep,
    correct: Option<bool>,
    revealed: bool,
    memory: bool,
    observation: Option<Observation>,
    chosen: Option<skat_core::Card>,
    result: Option<CoachResult>,
    figures: bool,
    sequence: u32,
    legal_cards: Vec<skat_core::Card>,
    hand: Vec<skat_core::Card>,
}

impl GuidedState {
    pub(crate) fn new(key: DecisionKey, view: &PlayerView, result: Option<CoachResult>) -> Self {
        let lesson = guided::lesson(view);
        let bid = guided::bid_lesson(view);
        let pre = guided::pre_lesson(view);
        let step = if lesson.is_some() || bid.is_some() || pre.is_some() {
            GuidedStep::Orient
        } else {
            GuidedStep::Unsupported
        };
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let mut id = GuidedId::from(key);
        id.generation = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            key,
            id,
            lesson,
            bid,
            pre,
            step,
            correct: None,
            revealed: false,
            memory: false,
            observation: None,
            chosen: None,
            result,
            figures: false,
            sequence: 0,
            legal_cards: view
                .legal_actions
                .iter()
                .filter_map(|a| {
                    if let Action::Play(c) = a {
                        Some(*c)
                    } else {
                        None
                    }
                })
                .collect(),
            hand: view.hand.iter().collect(),
        }
    }

    pub(crate) fn step(&self) -> GuidedStep {
        self.step
    }

    pub(crate) fn event(&mut self, action: Option<GuidedAction>) -> GuidedEvent {
        let event = GuidedEvent {
            id: self.id,
            sequence: self.sequence,
            action,
            step: self.step,
            assisted: self.revealed,
        };
        self.sequence += 1;
        event
    }

    pub(crate) fn capture(&mut self, result: &CoachResult, budget: crate::Budget) {
        if result.guided_key() == self.key
            && result.guided_budget() == budget
            && !result.is_preview()
        {
            self.result = Some(result.clone());
        }
    }

    pub(crate) fn view(&self, figures_enabled: bool, basis: crate::CoachBasis) -> GuidedView {
        let figures = (self.figures && figures_enabled)
            .then(|| {
                self.result
                    .as_ref()?
                    .explanation()
                    .map(|e| skat_coach::hints(e, skat_coach::Stage::Figures))
            })
            .flatten();
        GuidedView {
            id: self.id,
            step: self.step,
            correct: self.correct,
            task: matches!(self.step, GuidedStep::Narrow | GuidedStep::Act)
                .then(|| self.lesson.as_ref().map(|l| l.rule.branch))
                .flatten(),
            rule: self
                .revealed
                .then(|| self.lesson.as_ref().map(|l| l.rule.clone()))
                .flatten(),
            evidence: self
                .correct
                .and_then(|_| self.lesson.as_ref().map(|l| l.source)),
            memory: self
                .memory
                .then(|| self.lesson.as_ref().map(Lesson::memory))
                .flatten(),
            observed_source: self.observation.map(|o| o.source),
            observation: (self.step == GuidedStep::Complete)
                .then_some(self.observation)
                .flatten(),
            budget: figures
                .as_ref()
                .and_then(|_| self.result.as_ref().map(CoachResult::guided_budget)),
            figures,
            basis,
            figures_requested: self.figures && figures_enabled,
            legal_cards: self.legal_cards.clone(),
            reizen: self.bid.map(|b| {
                let oriented = self.revealed || self.step != GuidedStep::Orient;
                GuidedReizen {
                    current: b.rule.current,
                    holds: oriented.then_some(b.rule.holds),
                    stake: b.rule.stake.filter(|_| oriented),
                }
            }),
            bid_rule: self.revealed.then(|| self.bid.map(|b| b.rule)).flatten(),
            preplay: self.pre.map(|p| {
                let oriented = self.revealed || self.step != GuidedStep::Orient;
                let (phase, threshold, cards) = match p.rule {
                    GuidedPreRule::Pickup(r) => {
                        (skat_core::Phase::SkatAufnahme, Some(r.threshold), vec![])
                    }
                    GuidedPreRule::Discard(_) => {
                        (skat_core::Phase::Druecken, None, self.hand.clone())
                    }
                };
                GuidedPreplay {
                    phase,
                    game: oriented.then(|| p.rule.game()),
                    threshold: threshold.filter(|_| oriented),
                    cards,
                }
            }),
            pre_rule: self.revealed.then(|| self.pre.map(|p| p.rule)).flatten(),
        }
    }

    pub(crate) fn action(
        &mut self,
        id: GuidedId,
        action: GuidedAction,
        figures: bool,
    ) -> Result<bool, AppError> {
        if id != self.id {
            return Err(AppError::StaleGuided);
        }
        match action {
            GuidedAction::Return => return Ok(true),
            GuidedAction::Answer(answer) => {
                let correct = if self.step == GuidedStep::Observe {
                    matches!(answer, GuidedAnswer::Void(v) if self.observation.is_some_and(|o| o.proves_void == v))
                } else {
                    if !matches!(
                        self.step,
                        GuidedStep::Orient
                            | GuidedStep::Check
                            | GuidedStep::Narrow
                            | GuidedStep::Threshold
                    ) {
                        return Err(AppError::NoQuestion);
                    }
                    self.lesson
                        .as_ref()
                        .is_some_and(|l| l.check(self.step, answer))
                        || self.bid.is_some_and(|b| b.check(self.step, answer))
                        || self.pre.is_some_and(|p| p.check(self.step, answer))
                };
                self.correct = Some(correct);
                if correct {
                    self.step = match self.step {
                        GuidedStep::Orient if self.bid.is_some() || self.pre.is_some() => {
                            GuidedStep::Threshold
                        }
                        GuidedStep::Orient => GuidedStep::Check,
                        GuidedStep::Check => GuidedStep::Narrow,
                        GuidedStep::Narrow | GuidedStep::Threshold => {
                            self.revealed = true;
                            GuidedStep::Act
                        }
                        GuidedStep::Observe => GuidedStep::Complete,
                        step => step,
                    };
                    // Feedback of the previous question must not disclose the next one.
                    if matches!(
                        self.step,
                        GuidedStep::Check | GuidedStep::Narrow | GuidedStep::Threshold
                    ) {
                        self.correct = None;
                    }
                }
            }
            GuidedAction::Retry => self.correct = None,
            GuidedAction::Reveal => {
                if self.step == GuidedStep::Observe {
                    self.step = GuidedStep::Complete;
                } else if matches!(
                    self.step,
                    GuidedStep::Orient
                        | GuidedStep::Check
                        | GuidedStep::Narrow
                        | GuidedStep::Threshold
                        | GuidedStep::Act
                ) {
                    self.revealed = true;
                    self.step = GuidedStep::Act;
                } else {
                    return Err(AppError::NoQuestion);
                }
            }
            GuidedAction::Memory(open) => {
                if !self.revealed
                    || self.lesson.is_none()
                    || matches!(self.step, GuidedStep::Waiting | GuidedStep::Observe)
                {
                    return Err(AppError::NoQuestion);
                }
                self.memory = open;
            }
            GuidedAction::Figures => {
                if !figures || self.step != GuidedStep::Complete {
                    return Err(AppError::NoQuestion);
                }
                self.figures = true;
            }
        }
        Ok(false)
    }

    /// Capture only the originating trick. Later automated cards cannot replace it.
    /// Reizen, Skataufnahme, Drücken and unsupported dialogues end with any action.
    pub(crate) fn expires_on(&self, before: Option<Seat>) -> bool {
        self.lesson.is_none()
            || (self.chosen.is_some()
                && self.lesson.as_ref().is_some_and(|l| Some(l.seat) == before))
    }

    pub(crate) fn applied(&mut self, before: Option<Seat>, action: Action, events: &[Event]) {
        let Some(lesson) = &self.lesson else {
            return;
        };
        let skat_coach::Source::Trick { index, led, .. } = lesson.source else {
            return;
        };
        if before == Some(lesson.seat)
            && self.chosen.is_none()
            && let Action::Play(card) = action
        {
            self.chosen = Some(card);
            self.step = GuidedStep::Waiting;
            self.correct = None;
            self.memory = false;
        }
        if self.chosen.is_none() || self.observation.is_some() {
            return;
        }
        let target = lesson.remaining.unwrap_or(lesson.seat);
        let mut observed = None;
        let mut winner = None;
        for event in events {
            match *event {
                Event::CardPlayed { seat, card } if seat == target => observed = Some((seat, card)),
                Event::TrickWon { trick } => winner = Some(trick.winner),
                _ => (),
            }
        }
        if let Some((seat, card)) = observed {
            self.observation = guided::observe(
                lesson.game,
                skat_coach::Source::Trick {
                    index,
                    seat,
                    led,
                    card,
                },
                winner,
            );
            self.step = GuidedStep::Observe;
        }
    }
}

impl Session {
    /// Current explicitly entered dialogue, independent of ordinary automatic hints.
    pub fn guided(&self) -> Option<GuidedView> {
        if self.coach.settings.level == CoachLevel::Off {
            return None;
        }
        self.guided.as_ref().map(|s| {
            s.view(
                self.coach.settings.show_figures,
                self.config().coach_basis(),
            )
        })
    }
}
