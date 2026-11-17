// SPDX-License-Identifier: EUPL-1.2

//! Deliberately entered hypothetical live questions, from a closed PlayerView.
//! No absence oracle, strategic diagnosis or hidden-deal reconstruction.
use serde::{Deserialize, Serialize};
use skat_analysis::mechanism::{Abstich, Certainty, Mechanism, mechanisms};

use crate::{
    Action, AttemptId, AttemptKind, Card, FocusAnswer, LearningError, LearningHistory,
    MechanismCounts, MechanismResponse, Phase, PlayerView, Seat, Session,
};

/// Immutable live question identity inside a store-assigned attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LiveAbstichId {
    /// Session lineage, not a deal seed.
    pub lineage: u64,
    /// One-based game number.
    pub game: u32,
    /// Applied actions at the decision boundary.
    pub actions: usize,
    /// Learner-selected hypothetical card.
    pub card: Card,
    /// First positive opponent in stable seat order; absent for unsupported selections.
    pub opponent: Option<Seat>,
    /// Question and shared-evidence schema version (Abstich only).
    pub version: u32,
}

/// Explicit transitions; none plays the candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveAbstichAction {
    /// Submit a conditional answer.
    Answer(FocusAnswer),
    /// Reveal the shared deduction, permanently marking assistance.
    Help,
    /// Explicitly skip without answering.
    Skip,
    /// Return to play, retaining an unanswered presentation.
    Close,
}

/// One bounded permitted-view observation. Expected answers are never persisted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveAbstichObservation {
    pub(crate) id: LiveAbstichId,
    pub(crate) provenance: PlayerView,
    pub(crate) given: Option<FocusAnswer>,
    pub(crate) assisted: bool,
    pub(crate) skipped: bool,
    #[serde(skip)]
    pub(crate) evidence: Option<Abstich>,
}

impl LiveAbstichObservation {
    /// Identity, including candidate and target.
    pub fn id(&self) -> LiveAbstichId {
        self.id
    }
    /// Explicit response category, never a judgement of the played card.
    pub fn response(&self) -> MechanismResponse {
        if self.evidence.is_none() {
            return MechanismResponse::Unsupported;
        }
        if self.skipped {
            return MechanismResponse::Skipped;
        }
        self.given.map_or(MechanismResponse::Unanswered, |given| {
            MechanismResponse::classify(given, self.evidence.map(expected))
        })
    }
    fn validate(&mut self) -> Result<(), LearningError> {
        let v = &self.provenance;
        if self.id.version != 1
            || self.id.game == 0
            || self.id.actions > crate::MAX_RECORDING_ACTIONS
            || v.tricks.len() > 10
            || v.bidding.len() > crate::MAX_RECORDING_ACTIONS
            || v.legal_actions.len() > 12
            || v.own_skat.is_some()
                != (v.declarer == Some(v.seat) && v.contract.is_some_and(|c| !c.hand))
            || !valid_provenance(v)
            || !eligible(v)
            || !v.legal_actions.contains(&Action::Play(self.id.card))
            || self.given.is_some_and(|g| {
                !matches!(
                    g,
                    FocusAnswer::Possible | FocusAnswer::Certain | FocusAnswer::Absent
                )
            })
            || (self.skipped && self.given.is_some())
        {
            return Err(LearningError::InvalidProvenance);
        }
        let evidence = deduction(v, self.id.card);
        if evidence.map(|e| e.seat) != self.id.opponent
            || (evidence.is_none() && (self.given.is_some() || self.assisted || self.skipped))
        {
            return Err(LearningError::InvalidProvenance);
        }
        self.evidence = evidence;
        Ok(())
    }
}

fn valid_provenance(v: &PlayerView) -> bool {
    let Some(contract) = v.contract else {
        return false;
    };
    let legal: crate::CardSet = v
        .legal_actions
        .iter()
        .filter_map(|a| match a {
            Action::Play(c) => Some(*c),
            _ => None,
        })
        .collect();
    legal == skat_core::legal_cards(contract.game, v.hand, v.current_trick.led())
        && v.hand_game == Some(contract.hand)
        && v.result.is_none()
        && v.declarer.is_some()
        && v.current_trick
            .cards
            .iter()
            .skip(v.current_trick.len())
            .all(Option::is_none)
        && v.current_trick.len() < 3
        && v.current_trick
            .leader
            .unwrap_or(v.seat)
            .index()
            .wrapping_add(v.current_trick.len())
            % 3
            == v.seat.index()
}

fn eligible(v: &PlayerView) -> bool {
    v.phase == Phase::Spiel
        && v.current_seat == Some(v.seat)
        && v.legal_actions
            .iter()
            .filter(|a| matches!(a, Action::Play(_)))
            .count()
            > 1
        && v.declarer_cards.is_none()
        && v.contract
            .is_some_and(|c| !c.ouvert && !matches!(c.game, crate::GameType::Null))
}
fn deduction(v: &PlayerView, card: Card) -> Option<Abstich> {
    let found = mechanisms(v, Action::Play(card));
    Seat::ALL.into_iter().find_map(|seat| {
        found.iter().find_map(|m| match m {
            Mechanism::Abstich(a) if a.seat == seat => Some(*a),
            _ => None,
        })
    })
}
fn expected(a: Abstich) -> FocusAnswer {
    match a.certainty {
        Certainty::Possible => FocusAnswer::Possible,
        Certainty::Certain => FocusAnswer::Certain,
    }
}

/// Current screen; neither the selection nor the pending prompt exposes certainty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveAbstichView {
    /// Current decision token; selection commands must echo it.
    pub boundary: (u64, u32, usize),
    /// All legal candidates, in game order, without ranking.
    pub candidates: Vec<Card>,
    /// Selected opportunity, if any.
    pub id: Option<LiveAbstichId>,
    /// Unsupported mode or selected candidate.
    pub unsupported: bool,
    /// Revealed only after explicit answer or help.
    pub evidence: Option<Abstich>,
    /// Descriptive settlement, only after explicit answer or skip.
    pub response: Option<MechanismResponse>,
    /// Sticky assistance.
    pub assisted: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LiveAbstichState {
    pub(crate) open: bool,
    selected: Option<usize>,
    pub(crate) observations: Vec<LiveAbstichObservation>,
}

impl Session {
    /// Forget in-memory observations after confirmed history reset. The game is unchanged.
    pub fn forget_live_abstich(&mut self) {
        self.live_abstich = Default::default();
    }
    /// Frozen observations of this decision, for the single learning history.
    pub fn live_abstich_observations(&self) -> &[LiveAbstichObservation] {
        &self.live_abstich.observations
    }
    /// Deliberate entry remains available even when this mode is unsupported.
    pub fn live_abstich(&self) -> Option<LiveAbstichView> {
        if !self.live_abstich.open {
            return None;
        }
        let o = self
            .live_abstich
            .selected
            .map(|i| &self.live_abstich.observations[i]);
        Some(LiveAbstichView {
            boundary: self
                .human_decision()
                .map_or((0, 0, 0), |k| (k.lineage, k.number, k.actions)),
            candidates: self.view_candidates(),
            id: o.map(|o| o.id),
            unsupported: !self.live_abstich_eligible() || o.is_some_and(|o| o.evidence.is_none()),
            evidence: o
                .filter(|o| o.assisted || o.given.is_some())
                .and_then(|o| o.evidence),
            response: o
                .filter(|o| o.given.is_some() || o.skipped)
                .map(|o| o.response()),
            assisted: o.is_some_and(|o| o.assisted),
        })
    }
    fn view_candidates(&self) -> Vec<Card> {
        self.game
            .view(crate::PlayerId::HUMAN.seat(self.dealer))
            .legal_actions
            .into_iter()
            .filter_map(|a| {
                if let Action::Play(c) = a {
                    Some(c)
                } else {
                    None
                }
            })
            .collect()
    }
    fn live_abstich_eligible(&self) -> bool {
        !self.is_practice()
            && !self.config().shows_all_cards()
            && self.human_decision().is_some()
            && eligible(&self.game.view(crate::PlayerId::HUMAN.seat(self.dealer)))
    }
    pub(crate) fn start_live_abstich(&mut self) -> Result<Vec<crate::AppEvent>, crate::AppError> {
        if self.quiz.is_pending() || self.guided.is_some() || self.coach.pending.is_some() {
            return Err(crate::AppError::QuestionPending);
        }
        self.live_abstich.open = true;
        self.live_abstich.selected = None;
        Ok(Vec::new())
    }
    pub(crate) fn select_live_abstich(
        &mut self,
        boundary: (u64, u32, usize),
        card: Card,
    ) -> Result<Vec<crate::AppEvent>, crate::AppError> {
        if !self.live_abstich.open
            || !self.live_abstich_eligible()
            || !self.view_candidates().contains(&card)
        {
            return Err(crate::AppError::StaleLiveAbstich);
        }
        let key = self
            .human_decision()
            .ok_or(crate::AppError::StaleLiveAbstich)?;
        if boundary != (key.lineage, key.number, key.actions) {
            return Err(crate::AppError::StaleLiveAbstich);
        }
        if let Some(i) = self
            .live_abstich
            .observations
            .iter()
            .position(|o| o.id.card == card)
        {
            self.live_abstich.selected = Some(i);
        } else {
            let provenance = self.game.view(crate::PlayerId::HUMAN.seat(self.dealer));
            let evidence = deduction(&provenance, card);
            let id = LiveAbstichId {
                lineage: key.lineage,
                game: key.number,
                actions: key.actions,
                card,
                opponent: evidence.map(|e| e.seat),
                version: 1,
            };
            self.live_abstich.selected = Some(self.live_abstich.observations.len());
            self.live_abstich.observations.push(LiveAbstichObservation {
                id,
                provenance,
                evidence,
                given: None,
                assisted: false,
                skipped: false,
            });
        }
        Ok(vec![crate::AppEvent::LiveAbstich])
    }
    pub(crate) fn respond_live_abstich(
        &mut self,
        id: LiveAbstichId,
        action: LiveAbstichAction,
    ) -> Result<Vec<crate::AppEvent>, crate::AppError> {
        let key = self
            .human_decision()
            .ok_or(crate::AppError::StaleLiveAbstich)?;
        if !self.live_abstich.open
            || (id.lineage, id.game, id.actions) != (key.lineage, key.number, key.actions)
        {
            return Err(crate::AppError::StaleLiveAbstich);
        }
        let o = self
            .live_abstich
            .selected
            .and_then(|i| self.live_abstich.observations.get_mut(i))
            .filter(|o| o.id == id)
            .ok_or(crate::AppError::StaleLiveAbstich)?;
        if action == LiveAbstichAction::Close {
            self.live_abstich.open = false;
            return Ok(Vec::new());
        }
        if o.evidence.is_none() {
            return Err(crate::AppError::StaleLiveAbstich);
        }
        if o.given.is_none() && !o.skipped {
            match action {
                LiveAbstichAction::Help => o.assisted = true,
                LiveAbstichAction::Skip => o.skipped = true,
                LiveAbstichAction::Answer(g)
                    if matches!(
                        g,
                        FocusAnswer::Possible | FocusAnswer::Certain | FocusAnswer::Absent
                    ) =>
                {
                    o.given = Some(g)
                }
                _ => return Err(crate::AppError::StaleLiveAbstich),
            }
        }
        Ok(vec![crate::AppEvent::LiveAbstich])
    }
}

/// Persisted observation, independently namespaced by the store attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveAbstichEntry {
    /// Durable attempt; no seed or restart collision.
    pub attempt: AttemptId,
    /// Immutable provenance and monotone response state.
    pub observation: LiveAbstichObservation,
}

impl LearningHistory {
    /// Live observations in original presentation order.
    pub fn live_abstich_entries(&self) -> &[LiveAbstichEntry] {
        &self.live_abstich
    }
    /// Idempotently record a snapshot batch; provenance is checked at entry/load, never per frame.
    pub fn record_live_abstich(
        &mut self,
        attempt: AttemptId,
        observations: &[LiveAbstichObservation],
    ) -> Result<bool, LearningError> {
        let a = self
            .attempts
            .iter()
            .find(|a| a.id == attempt)
            .ok_or(LearningError::UnknownAttempt)?;
        if a.kind != AttemptKind::Series {
            return Err(LearningError::InvalidContext);
        }
        let mut updates = Vec::new();
        let mut new = Vec::new();
        for o in observations {
            let mut o = o.clone();
            let key = (attempt, o.id);
            if let Some(&i) = self.live_keys.get(&key) {
                let old = &self.live_abstich[i].observation;
                if old.provenance != o.provenance {
                    return Err(LearningError::InvalidProvenance);
                }
                // A settled response and assistance cannot be replaced by resent old batches.
                o.evidence = old.evidence;
                if old.given.is_some() || old.skipped {
                    continue;
                }
                if old.assisted {
                    o.assisted = true;
                }
                if o.given.is_some_and(|g| {
                    !matches!(
                        g,
                        FocusAnswer::Possible | FocusAnswer::Certain | FocusAnswer::Absent
                    )
                }) || (o.given.is_some() && o.skipped)
                    || (o.evidence.is_none() && (o.given.is_some() || o.skipped || o.assisted))
                {
                    return Err(LearningError::InvalidProvenance);
                }
                if *old != o {
                    updates.push((i, o));
                }
            } else {
                if new
                    .iter()
                    .any(|e: &LiveAbstichEntry| e.observation.id == o.id)
                {
                    continue;
                }
                o.validate()?;
                new.push(LiveAbstichEntry {
                    attempt,
                    observation: o,
                });
            }
        }
        if self.events.len() + self.focus.len() + self.live_abstich.len() + new.len()
            > crate::MAX_LEARNING_EVENTS
        {
            self.refused = true;
            return Err(LearningError::Full);
        }
        let changed = !updates.is_empty() || !new.is_empty();
        for (i, o) in updates {
            self.live_abstich[i].observation = o;
        }
        for e in new {
            self.live_keys
                .insert((attempt, e.observation.id), self.live_abstich.len());
            self.live_abstich.push(e);
        }
        Ok(changed)
    }
    pub(crate) fn validate_live_abstich(&mut self) -> Result<(), LearningError> {
        if self.events.len() + self.focus.len() + self.live_abstich.len()
            > crate::MAX_LEARNING_EVENTS
        {
            return Err(LearningError::TooLarge);
        }
        self.live_keys.clear();
        let kinds: std::collections::HashMap<_, _> =
            self.attempts.iter().map(|a| (a.id, a.kind)).collect();
        for (i, e) in self.live_abstich.iter_mut().enumerate() {
            if kinds.get(&e.attempt) != Some(&AttemptKind::Series) {
                return Err(LearningError::InvalidContext);
            }
            e.observation.validate()?;
            if self
                .live_keys
                .insert((e.attempt, e.observation.id), i)
                .is_some()
            {
                return Err(LearningError::DuplicateEvent);
            }
        }
        Ok(())
    }
    /// Separate series/closed-view/schema-v1 rows split by sticky assistance.
    pub fn live_abstich_summary(&self) -> [(bool, MechanismCounts); 2] {
        let mut rows = [
            (false, MechanismCounts::default()),
            (true, MechanismCounts::default()),
        ];
        for e in &self.live_abstich {
            let c = &mut rows[usize::from(e.observation.assisted)].1;
            if e.observation.evidence.is_none() {
                c.presented += 1;
                c.unsupported += 1;
                c.unknown += 1;
            } else {
                c.add(e.observation.response());
            }
        }
        rows
    }
}
