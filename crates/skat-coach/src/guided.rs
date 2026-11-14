// SPDX-License-Identifier: EUPL-1.2

//! Bounded guided Farbspiel, Reizen, Skataufnahme, Drücken, Ansage and
//! Farbspiel Kontra/Re lessons. Exact rule provenance, never search ranking.

use crate::Source;
pub use skat_analysis::baseline::{Branch, CardRule, KontraCall, KontraRule};
pub use skat_analysis::policy::{
    AnnounceCandidate, AnnounceRule, BidRule, DiscardRule, GameBranch, Keep, PickupRule,
};
use skat_analysis::{Knowledge, TrickState, baseline, policy};
use skat_core::{
    Action, Card, CardSet, Event, Follow, GameType, KontraLevel, Phase, PlayerView, Seat,
};

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
    /// Ansage: does the preferred game's value reach the public bid?
    /// Kontra/Re: does the own count reach Easy's condition?
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
    /// Ansage threshold: the preferred game's value reaches the bid.
    /// Kontra/Re threshold: the count reaches Easy's Kontra or Re condition.
    Within(bool),
    /// Kontra/Re orientation: factor on the game points after the call.
    Factor(i32),
    /// Skataufnahme/Drücken/Ansage orientation: which branch builds the candidate game.
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
    /// Ansage: which game is announced.
    Announce(AnnounceRule),
}

impl PreRule {
    /// Branch of the candidate game choice.
    pub fn branch(&self) -> GameBranch {
        match self {
            Self::Pickup(r) => r.branch,
            Self::Discard(r) => r.branch,
            Self::Announce(r) => r.branch,
        }
    }

    /// Candidate game behind the rule; conditional, not a contract (Ansage:
    /// the preferred game, which the bid may still rule out).
    pub fn game(&self) -> GameType {
        match self {
            Self::Pickup(r) => r.game,
            Self::Discard(r) => r.game,
            Self::Announce(r) => r.preferred,
        }
    }

    /// Actual selected action.
    pub fn action(&self) -> skat_core::Action {
        match self {
            Self::Pickup(r) => r.action,
            Self::Discard(r) => r.action,
            Self::Announce(r) => r.action,
        }
    }

    /// Easy's wanted action was not legal; the first legal action is shown.
    pub fn fallback(&self) -> bool {
        match self {
            Self::Pickup(r) => r.fallback,
            Self::Discard(r) => r.fallback,
            Self::Announce(r) => r.fallback,
        }
    }
}

/// A Skataufnahme, Drücken or Ansage lesson from Easy's shared rule. Never passed to
/// the frontend before disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PreLesson {
    /// Actual shared Easy outcome and its inputs.
    pub rule: PreRule,
    /// Learner's seat.
    pub seat: Seat,
}

/// Every declarer Skataufnahme, Drücken and Ansage decision is supported.
pub fn pre_lesson(view: &PlayerView) -> Option<PreLesson> {
    let rule = match view.phase {
        Phase::SkatAufnahme => PreRule::Pickup(policy::pickup_rule(view)?),
        Phase::Druecken => PreRule::Discard(policy::discard_rule(view)?),
        Phase::Ansage => PreRule::Announce(policy::announce_rule(view)?),
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
            (Step::Threshold, Answer::Within(w), PreRule::Announce(r)) => w == r.fits,
            _ => false,
        }
    }
}

/// A non-Ouvert Farbspiel Kontra or immediate Re lesson from Easy's shared
/// rule. Never passed to the frontend before disclosure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KontraLesson {
    /// Actual shared Easy outcome and its inputs.
    pub rule: KontraRule,
    /// Learner's seat.
    pub seat: Seat,
}

/// Why a core-offered Kontra/Re decision has no guided lesson.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KontraExclusion {
    /// The separate Ouvert round before the reveal (`KeinKontra` offered).
    OuvertRound,
    /// An Ouvert contract after the reveal.
    Ouvert,
    /// Grand: not taught by this lesson.
    Grand,
    /// Null: Easy never says Kontra; not taught by this lesson.
    Null,
    /// Absent contract or a restricted/inconsistent permitted view.
    Restricted,
}

/// Only a core-offered Kontra at a non-Ouvert Farbspiel card turn or the
/// declarer's immediate Re reply. Uses the own cards, the own known Skat and
/// public facts only; restricted or inconsistent views get no lesson.
pub fn kontra_lesson(view: &PlayerView) -> Option<KontraLesson> {
    kontra_support(view).ok()
}

/// The exclusion of a Kontra/Re offer without a lesson; none if the view
/// offers no Kontra, Re or KeinKontra, or if the lesson is supported.
pub fn kontra_exclusion(view: &PlayerView) -> Option<KontraExclusion> {
    let offered = view
        .legal_actions
        .iter()
        .any(|a| matches!(a, Action::Kontra | Action::KeinKontra | Action::Re));
    offered.then(|| kontra_support(view).err()).flatten()
}

fn kontra_support(view: &PlayerView) -> Result<KontraLesson, KontraExclusion> {
    if view.legal_actions.contains(&Action::KeinKontra) {
        return Err(KontraExclusion::OuvertRound);
    }
    let contract = view.contract.ok_or(KontraExclusion::Restricted)?;
    match contract.game {
        _ if contract.ouvert => return Err(KontraExclusion::Ouvert),
        GameType::Grand => return Err(KontraExclusion::Grand),
        GameType::Null => return Err(KontraExclusion::Null),
        GameType::Farbe(_) => (),
    }
    let rule = baseline::kontra_rule(view).ok_or(KontraExclusion::Restricted)?;
    let declarer = view.declarer.ok_or(KontraExclusion::Restricted)?;
    let consistent = match rule.call {
        KontraCall::Kontra => declarer != view.seat && view.kontra == KontraLevel::None,
        KontraCall::Re => declarer == view.seat && view.kontra == KontraLevel::Kontra,
    };
    if !consistent || rule.fallback || view.current_seat != Some(view.seat) {
        return Err(KontraExclusion::Restricted);
    }
    Ok(KontraLesson {
        rule,
        seat: view.seat,
    })
}

impl KontraLesson {
    /// Factor on the game points if the learner makes the call.
    pub fn factor(&self) -> i32 {
        match self.rule.call {
            KontraCall::Kontra => KontraLevel::Kontra.factor(),
            KontraCall::Re => KontraLevel::Re.factor(),
        }
    }

    /// Orientation asks for the factor after the call (public rule fact); the
    /// threshold answer is whether Easy's condition holds.
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        match (step, answer) {
            (Step::Orient, Answer::Factor(f)) => f == self.factor(),
            (Step::Threshold, Answer::Within(w)) => w == self.rule.wants,
            _ => false,
        }
    }

    /// The learner's own action at the original decision: what the next
    /// public transition is.
    pub fn awaits(&self, action: Action) -> Option<KontraAwait> {
        match (self.rule.call, action) {
            (KontraCall::Kontra, Action::Kontra) => Some(KontraAwait::Reply),
            (KontraCall::Kontra, Action::Play(_)) => Some(KontraAwait::Declined),
            (KontraCall::Re, Action::Re | Action::KeinRe) => Some(KontraAwait::Resumed),
            _ => None,
        }
    }
}

/// The next public transition the lesson observes after the learner acted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KontraAwait {
    /// The learner said Kontra: the declarer's Re or KeinRe.
    Reply,
    /// The learner declined Kontra by playing a card: that card itself.
    Declined,
    /// The learner answered with Re or KeinRe: the next card played.
    Resumed,
}

/// The actual next public transition, never a hypothetical line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KontraObservation {
    /// The declarer's actual reply to the learner's Kontra.
    Reply {
        /// Declarer.
        seat: Seat,
        /// Re (true) or KeinRe.
        re: bool,
        /// Resulting public level.
        level: KontraLevel,
    },
    /// Card play resumed after the learner's Re or KeinRe.
    Resumed {
        /// Seat that played next.
        seat: Seat,
        /// Its actual card.
        card: Card,
        /// Public level, unchanged by the card.
        level: KontraLevel,
    },
    /// The learner declined Kontra by playing a card; Kontra is no longer
    /// offered to it after its first card.
    Declined {
        /// Learner's seat.
        seat: Seat,
        /// The actual card.
        card: Card,
        /// Zero-based trick index.
        trick: u8,
        /// Actual trick winner if the card completed the trick.
        winner: Option<Seat>,
        /// Public level, unchanged.
        level: KontraLevel,
    },
}

/// The awaited transition in core `events` after one applied action;
/// `level` is the public level after it and `trick` the index of the trick
/// the events belong to.
pub fn observe_kontra(
    awaits: KontraAwait,
    seat: Seat,
    events: &[Event],
    level: KontraLevel,
    trick: u8,
) -> Option<KontraObservation> {
    let winner = events.iter().find_map(|e| match e {
        Event::TrickWon { trick } => Some(trick.winner),
        _ => None,
    });
    events.iter().find_map(|&e| match (awaits, e) {
        (KontraAwait::Reply, Event::Re { seat }) => Some(KontraObservation::Reply {
            seat,
            re: true,
            level,
        }),
        (KontraAwait::Reply, Event::KeinRe { seat }) => Some(KontraObservation::Reply {
            seat,
            re: false,
            level,
        }),
        (KontraAwait::Resumed, Event::CardPlayed { seat, card }) => {
            Some(KontraObservation::Resumed { seat, card, level })
        }
        (KontraAwait::Declined, Event::CardPlayed { seat: s, card }) if s == seat => {
            Some(KontraObservation::Declined {
                seat,
                card,
                trick,
                winner,
                level,
            })
        }
        _ => None,
    })
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
