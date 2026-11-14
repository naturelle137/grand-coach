// SPDX-License-Identifier: EUPL-1.2

//! Bounded guided Farbspiel, Reizen, Skataufnahme and Drücken lessons. Exact
//! rule provenance, never search ranking.

use crate::Source;
pub use skat_analysis::baseline::{Branch, CardRule};
pub use skat_analysis::policy::{BidRule, DiscardRule, GameBranch, Keep, PickupRule};
use skat_analysis::{Knowledge, TrickState, baseline, policy};
use skat_core::{Card, CardSet, Follow, GameType, Phase, PlayerView, Seat};

/// One current prompt; no later answer is included in a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Step {
    /// Identify who is currently winning from the table.
    Orient,
    /// Identify the remaining seat, or none in last seat.
    Check,
    /// Try a legal card that serves this branch's immediate purpose.
    Narrow,
    /// Reizen: is the value at stake within Easy's ceiling for the own hand?
    /// Skataufnahme: does the strength reach Easy's Hand threshold?
    /// Drücken: name a card Easy puts away first.
    Threshold,
    /// Deliberately revealed worked answer; play any legal card.
    Act,
    /// Await the actual resulting card/trick, across automated turns.
    Waiting,
    /// Does the actual card prove a void in the led follow class?
    Observe,
    /// Feedback is available; return to ordinary play.
    Complete,
    /// No supported rule branch; no invented explanation.
    Unsupported,
}

/// Structured answers, checked from the frozen permitted input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Answer {
    /// Current winner or remaining player (none if no player follows).
    Seat(Option<Seat>),
    /// A candidate, without playing it.
    Card(Card),
    /// Whether the observed play proves failure to follow the led class.
    Void(bool),
    /// Reizen orientation: answer (hold) the current bid, or name a higher one.
    Holds(bool),
    /// Reizen threshold: the value at stake is within the heuristic ceiling.
    /// Skataufnahme threshold: the strength reaches the Hand threshold.
    Within(bool),
    /// Skataufnahme/Drücken orientation: which branch builds the candidate game.
    Game(GameBranch),
}

/// A rule-backed lesson. Never passed to the frontend before disclosure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lesson {
    /// Actual shared Easy outcome.
    pub rule: CardRule,
    /// Exact public source for the current winning card.
    pub source: Source,
    /// Frozen own/permitted card knowledge, also used by counting aid.
    pub knowledge: Knowledge,
    /// Announced contract's rules.
    pub game: GameType,
    /// Seat after the learner in this trick; none in last seat.
    pub remaining: Option<Seat>,
    /// Learner's seat.
    pub seat: Seat,
}

/// Select only two tested branches, with real alternatives and candidates.
pub fn lesson(view: &PlayerView) -> Option<Lesson> {
    let game = view.contract?.game;
    if view.phase != Phase::Spiel || !matches!(game, GameType::Farbe(_)) {
        return None;
    }
    let rule = baseline::rule(view)?;
    if view.legal_actions.len() < 2
        || rule.candidates.is_empty()
        || (rule.branch == Branch::UnderPartner && rule.losers.is_empty())
        || !matches!(rule.branch, Branch::UnderPartner | Branch::WinLast)
    {
        return None;
    }
    let trick = TrickState::of(game, &view.current_trick);
    let winner = trick.best_seat?;
    Some(Lesson {
        source: Source::Trick {
            index: view.tricks.len() as u8,
            seat: winner,
            led: trick.led?,
            card: trick.best?,
        },
        remaining: (!rule.last).then_some(Seat::from_index(view.seat.index() + 1)),
        rule,
        game,
        seat: view.seat,
        knowledge: Knowledge::from_view(view),
    })
}

impl Lesson {
    /// Checks the immediate factual/policy task, not universal move quality.
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        match (step, answer) {
            (Step::Orient, Answer::Seat(seat)) => seat == self.rule.winner,
            (Step::Check, Answer::Seat(seat)) => seat == self.remaining,
            (Step::Narrow, Answer::Card(c)) => self.rule.candidates.contains(&c),
            _ => false,
        }
    }

    /// Card memory categories; no hidden holder is assigned.
    pub fn memory(&self) -> Memory {
        Memory {
            own: self.knowledge.own,
            skat: self.knowledge.skat,
            played: self.knowledge.played,
            unseen: self.knowledge.unseen(),
            trumpf: skat_analysis::counting::CountingAid::from_knowledge(&self.knowledge).trumpf,
        }
    }
}

/// A Reizen lesson from Easy's shared bidding rule. Never passed to the
/// frontend before disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BidLesson {
    /// Actual shared Easy outcome and its inputs.
    pub rule: BidRule,
    /// Learner's seat.
    pub seat: Seat,
}

/// Every Reizen decision of the learner is supported, including forced Pass.
pub fn bid_lesson(view: &PlayerView) -> Option<BidLesson> {
    Some(BidLesson {
        rule: policy::bid_rule(view)?,
        seat: view.seat,
    })
}

impl BidLesson {
    /// Orientation is public; the threshold answer is Easy's actual choice.
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        match (step, answer) {
            (Step::Orient, Answer::Holds(h)) => h == self.rule.holds,
            (Step::Threshold, Answer::Within(w)) => {
                w == (self.rule.action != skat_core::Action::Pass)
            }
            _ => false,
        }
    }
}

/// Easy's shared rule for one declarer decision before the play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PreRule {
    /// Skataufnahme: Hand or pickup.
    Pickup(PickupRule),
    /// Drücken: which two cards go into the Skat.
    Discard(DiscardRule),
}

impl PreRule {
    /// Branch of the candidate game choice.
    pub fn branch(&self) -> GameBranch {
        match self {
            Self::Pickup(r) => r.branch,
            Self::Discard(r) => r.branch,
        }
    }

    /// Candidate game behind the rule; conditional, not a contract.
    pub fn game(&self) -> GameType {
        match self {
            Self::Pickup(r) => r.game,
            Self::Discard(r) => r.game,
        }
    }

    /// Actual selected action.
    pub fn action(&self) -> skat_core::Action {
        match self {
            Self::Pickup(r) => r.action,
            Self::Discard(r) => r.action,
        }
    }

    /// Easy's wanted action was not legal; the first legal action is shown.
    pub fn fallback(&self) -> bool {
        match self {
            Self::Pickup(r) => r.fallback,
            Self::Discard(r) => r.fallback,
        }
    }
}

/// A Skataufnahme or Drücken lesson from Easy's shared rule. Never passed to
/// the frontend before disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PreLesson {
    /// Actual shared Easy outcome and its inputs.
    pub rule: PreRule,
    /// Learner's seat.
    pub seat: Seat,
}

/// Every declarer Skataufnahme and Drücken decision is supported.
pub fn pre_lesson(view: &PlayerView) -> Option<PreLesson> {
    let rule = match view.phase {
        Phase::SkatAufnahme => PreRule::Pickup(policy::pickup_rule(view)?),
        Phase::Druecken => PreRule::Discard(policy::discard_rule(view)?),
        _ => return None,
    };
    Some(PreLesson {
        rule,
        seat: view.seat,
    })
}

impl PreLesson {
    /// Orientation asks for the candidate branch; the threshold answer is
    /// Easy's actual choice (a discard tied with Easy's cards also counts).
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        match (step, answer, self.rule) {
            (Step::Orient, Answer::Game(b), rule) => b == rule.branch(),
            (Step::Threshold, Answer::Within(w), PreRule::Pickup(r)) => {
                w == (r.action == skat_core::Action::Hand)
            }
            (Step::Threshold, Answer::Card(c), PreRule::Discard(r)) => r.eligible.contains(c),
            _ => false,
        }
    }
}

/// Optional frozen memory, using M6 count definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Memory {
    /// Own cards.
    pub own: CardSet,
    /// Known own Skat, otherwise unknown.
    pub skat: Option<CardSet>,
    /// Publicly played cards, including the current trick.
    pub played: CardSet,
    /// Unseen cards; no holder or unknown Skat assignment.
    pub unseen: CardSet,
    /// Exact Trumpf counts; absent in Null.
    pub trumpf: Option<skat_analysis::counting::TrumpfCount>,
}

/// Actual observed card with its public provenance, never a hypothetical line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Observation {
    /// Public card source including trick, seat, led card and played card.
    pub source: Source,
    /// Led follow class; a Bube belongs to Trumpf in Farbspiel.
    pub follow: Follow,
    /// Whether the card proves a void in that class at this play.
    pub proves_void: bool,
    /// Actual winner of the completed trick, if complete.
    pub winner: Option<Seat>,
}

/// Public play supplies the observation; hidden cards are not accepted.
pub fn observe(game: GameType, source: Source, winner: Option<Seat>) -> Option<Observation> {
    let Source::Trick { led, card, .. } = source else {
        return None;
    };
    Some(Observation {
        source,
        follow: game.follow(led),
        proves_void: game.follow(card) != game.follow(led),
        winner,
    })
}
