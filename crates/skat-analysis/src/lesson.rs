// SPDX-License-Identifier: EUPL-1.2

//! Evidence contract for coach lessons (D-53, `docs/analysis.md`).
//!
//! The analysis already reports exact facts ([`Observation`]), sample
//! summaries of the opponent model ([`Inference`]) and estimates. This
//! module adds what a lesson needs on top, without a second game or belief
//! model and without another search:
//!
//! - [`Evidence`]: the facts and sample summaries of one decision, each with
//!   its class and provenance ([`Source`]) in a decision [`Snapshot`]. It
//!   names no action, so it may be shown before the move (D-49).
//! - [`Comparison`]: after the move, whether the model comparison of the
//!   chosen option with the best one is reliable. Only a complete analysis
//!   with both options evaluated on every distribution is. It also carries
//!   the exact [`Mechanism`]s of the trick after the chosen card.
//! - [`Lesson`]: a stable, versioned lesson definition with applicability,
//!   permitted evidence classes, conditions, exclusions and an explicit
//!   support status. [`Lesson::check`] is the predicate; the coach only
//!   selects among lessons that apply.
//!
//! Hard knowledge and sample summaries stay apart: conditions on facts
//! match only [`EvidenceClass::Exact`] items. A sampled minimum is not a
//! proven bound, and a positive expected Trumpf count does not prove that
//! a place holds Trumpf.

use std::borrow::Cow;

use skat_core::{Action, BidRecord, Call, Card, CardSet, GameType, Phase, PlayerView, Seat};

use crate::analysis::{Analysis, CardClass, Inference, Observation, Status, utility};
use crate::mechanism::{Mechanism, MechanismKind, mechanisms};

/// Version of this contract; raised when the meaning of a field changes.
pub const CONTRACT_VERSION: u16 = 4;

/// The deciding seat's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Role {
    /// Before the declarer is known.
    Bidder,
    /// Alleinspieler.
    Declarer,
    /// Gegenspieler.
    Defender,
}

impl Role {
    /// The role of `view.seat`.
    pub fn of(view: &PlayerView) -> Role {
        match view.declarer {
            None => Role::Bidder,
            Some(d) if d == view.seat => Role::Declarer,
            Some(_) => Role::Defender,
        }
    }
}

/// The kind of game, without the Trumpf suit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameKind {
    /// Farbspiel.
    Suit,
    /// Grand.
    Grand,
    /// Null.
    Null,
}

impl GameKind {
    /// The kind of `game`.
    pub fn of(game: GameType) -> GameKind {
        match game {
            GameType::Farbe(_) => GameKind::Suit,
            GameType::Grand => GameKind::Grand,
            GameType::Null => GameKind::Null,
        }
    }
}

/// The decision the evidence belongs to, as the deciding seat sees it.
/// Contains only what that seat may see.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snapshot {
    /// The deciding seat.
    pub seat: Seat,
    /// Phase of the decision.
    pub phase: Phase,
    /// The seat's role.
    pub role: Role,
    /// The announced game, if any.
    pub game: Option<GameType>,
    /// Bidding calls made before the decision.
    pub bids: u16,
    /// Bidding calls the deciding seat made before the decision.
    pub own_calls: u16,
    /// Completed tricks.
    pub tricks: u8,
    /// Cards in the trick in progress.
    pub trick_cards: u8,
    /// Own cards.
    pub own: CardSet,
    /// The own discarded Skat (declarer after Drücken).
    pub own_skat: Option<CardSet>,
    /// The Ouvert declarer's revealed cards, seen by a defender.
    pub revealed: Option<CardSet>,
}

impl Snapshot {
    /// The snapshot of the decision in `view`.
    pub fn of(view: &PlayerView) -> Snapshot {
        Snapshot {
            seat: view.seat,
            phase: view.phase,
            role: Role::of(view),
            game: view.contract.map(|c| c.game),
            bids: view.bidding.len() as u16,
            own_calls: view.bidding.iter().filter(|r| r.seat == view.seat).count() as u16,
            tricks: view.tricks.len() as u8,
            trick_cards: view.current_trick.len() as u8,
            own: view.hand,
            own_skat: view.own_skat,
            revealed: view
                .declarer_cards
                .filter(|_| view.declarer != Some(view.seat)),
        }
    }
}

/// Where a piece of evidence comes from: something the deciding seat saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Source {
    /// The own cards ([`Snapshot::own`]).
    OwnHand,
    /// The own discarded Skat ([`Snapshot::own_skat`]).
    OwnSkat,
    /// The Ouvert declarer's revealed cards ([`Snapshot::revealed`]).
    Revealed,
    /// The public play: every card played so far.
    Played,
    /// The card `seat` played in trick `index` (from 0; the trick in
    /// progress has the index [`Snapshot::tricks`]).
    Trick {
        /// The trick.
        index: u8,
        /// The seat.
        seat: Seat,
        /// The card led in that trick.
        led: Card,
        /// The card `seat` played.
        card: Card,
    },
    /// An empty trick: the acting seat has the lead, with no card or winner yet.
    Lead {
        /// Zero-based index of the empty trick.
        index: u8,
        /// Seat entitled to lead.
        seat: Seat,
    },
    /// The bidding call with this index in the bidding history.
    Bid {
        /// Index into `PlayerView::bidding`.
        index: u16,
    },
    /// The whole bidding history.
    Bidding,
    /// The rule configuration (Kontra/Re variant).
    Rules,
    /// The opponent model: the weighted distributions of the analysis.
    Model,
}

/// The class of a piece of evidence, from hard knowledge to claims about
/// option quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EvidenceClass {
    /// Exact knowledge from the view (card counting, rules, history).
    Exact,
    /// A summary of the sampled distributions; never a proven bound.
    Sample,
    /// A model comparison of two options ([`Comparison`]); after the move.
    Comparison,
    /// A validated strategic mechanism ([`LessonKind::Mechanism`]); after
    /// the move, only with a reliable comparison.
    Mechanism,
}

/// What a piece of evidence says.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Claim {
    /// An exact fact of the analysis.
    Fact(Observation),
    /// A sample summary of the analysis. Its `min` and `max` are the
    /// extremes of the evaluated distributions, not bounds, unless the
    /// inference is `certain`.
    Sample(Inference),
}

/// One piece of evidence with its provenance.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Item {
    /// What it says.
    pub claim: Claim,
    /// What the deciding seat saw that it rests on.
    pub sources: Vec<Source>,
}

impl Item {
    /// The evidence class: [`EvidenceClass::Exact`] for facts,
    /// [`EvidenceClass::Sample`] for sample summaries.
    pub fn class(&self) -> EvidenceClass {
        match self.claim {
            Claim::Fact(_) => EvidenceClass::Exact,
            Claim::Sample(_) => EvidenceClass::Sample,
        }
    }
}

/// The permitted evidence of one decision: facts and sample summaries from
/// one analysis of one seat's view. It names no action and ranks nothing,
/// so it may be shown before the move.
///
/// It depends only on the view, the analysis seed and the budget: hidden
/// cards that leave the view unchanged leave it unchanged. Exact items stay
/// valid when the search did not finish.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Evidence {
    /// [`CONTRACT_VERSION`] at creation.
    pub version: u16,
    /// The decision.
    pub snapshot: Snapshot,
    /// Status of the analysis the items come from.
    pub status: Status,
    /// Facts first, in the order of the analysis, then sample summaries.
    pub items: Vec<Item>,
}

/// The evidence of `analysis`, which must be the analysis of `view`. Reuses
/// its observations and inferences; computes no new strategy.
pub fn gather(view: &PlayerView, analysis: &Analysis) -> Evidence {
    let facts = analysis.observations.iter().map(|&o| Item {
        claim: Claim::Fact(o),
        sources: sources(view, &o),
    });
    let samples = analysis.inferences.iter().map(|&i| Item {
        claim: Claim::Sample(i),
        sources: vec![Source::Model],
    });
    Evidence {
        version: CONTRACT_VERSION,
        snapshot: Snapshot::of(view),
        status: analysis.status,
        items: facts.chain(samples).collect(),
    }
}

/// What a fact of `view` rests on.
fn sources(view: &PlayerView, observation: &Observation) -> Vec<Source> {
    match *observation {
        Observation::Void { seat, follow } => {
            let game = view.contract.map(|c| c.game);
            first_failure(view, seat, |led, card| {
                game.is_some_and(|g| g.follow(led) == follow && g.follow(card) != follow)
            })
            .into_iter()
            .collect()
        }
        Observation::TrumpsOut { .. } | Observation::AugenOut { .. } => {
            vec![Source::OwnHand, Source::Played]
        }
        Observation::TrumpfSets(sets) => {
            let mut out = vec![Source::OwnHand, Source::Played];
            if !sets.skat_unknown {
                out.push(Source::OwnSkat);
            }
            if view.declarer_cards.is_some() && view.declarer != Some(view.seat) {
                out.push(Source::Revealed);
            }
            out.extend(trick_cards(view).filter_map(|(index, seat, led, card)| {
                sets.played.contains(card).then_some(Source::Trick {
                    index,
                    seat,
                    led,
                    card,
                })
            }));
            out
        }
        Observation::AugenWon { .. } => vec![Source::Played],
        Observation::SkatKnown { .. } => vec![Source::OwnSkat],
        Observation::BidShown { seat, value } => {
            first_bid(&view.bidding, seat, value).into_iter().collect()
        }
        Observation::KontraEligible { .. } => vec![Source::Bidding, Source::Rules],
        Observation::Structure(_) => own_cards(view),
        Observation::GameValues(_) => {
            let mut out = own_cards(view);
            out.push(Source::Rules);
            out
        }
        Observation::Commitment { .. } => vec![Source::Bidding, Source::Rules],
    }
}

/// The own hand, and the own discarded Skat once known.
fn own_cards(view: &PlayerView) -> Vec<Source> {
    let mut out = vec![Source::OwnHand];
    if view.own_skat.is_some() {
        out.push(Source::OwnSkat);
    }
    out
}

/// Every card played so far as (trick index, seat, led card, card), in
/// order, the trick in progress last.
fn trick_cards(view: &PlayerView) -> impl Iterator<Item = (u8, Seat, Card, Card)> + '_ {
    let completed = view
        .tricks
        .iter()
        .map(|t| (Some(t.leader), t.cards.map(Some)));
    let current = std::iter::once((view.current_trick.leader, view.current_trick.cards));
    completed
        .chain(current)
        .enumerate()
        .flat_map(|(index, (leader, cards))| {
            (0..3).filter_map(move |i| {
                let (leader, led, card) = (leader?, cards[0]?, cards[i]?);
                Some((index as u8, Seat::from_index(leader.index() + i), led, card))
            })
        })
}

/// The first card `seat` played where `failed(led, card)` holds.
fn first_failure(
    view: &PlayerView,
    seat: Seat,
    failed: impl Fn(Card, Card) -> bool,
) -> Option<Source> {
    trick_cards(view).find_map(|(index, s, led, card)| {
        (s == seat && card != led && failed(led, card)).then_some(Source::Trick {
            index,
            seat,
            led,
            card,
        })
    })
}

/// The first call in which `seat` bid or held `value`.
fn first_bid(history: &[BidRecord], seat: Seat, value: u16) -> Option<Source> {
    history
        .iter()
        .position(|r| {
            r.seat == seat && matches!(r.call, Call::Bid(v) | Call::Hold(v) if v == value)
        })
        .map(|index| Source::Bid {
            index: index as u16,
        })
}

/// Whether a model comparison may back a claim about option quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Reliability {
    /// Complete analysis, both options evaluated on every distribution.
    Reliable,
    /// The analysis did not finish (budget or cancelled): provisional.
    Incomplete,
    /// An option was dropped after screening (Drücken): not compared.
    Screened,
    /// An option has no estimate.
    NotEvaluated,
}

/// The model comparison of a chosen option with the best one (after the
/// move). Carries no figures; those stay with the estimates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Comparison {
    /// The chosen option.
    pub chosen: Action,
    /// The option with the highest [`utility`] among those evaluated on
    /// the most distributions (ties: the first legal one); `None` if none
    /// was evaluated.
    pub best: Option<Action>,
    /// Whether the comparison may back a claim.
    pub reliability: Reliability,
    /// Exact mechanisms of the trick after the chosen card, from the view
    /// only ([`mechanisms`]); independent of the estimates. Only a mechanism
    /// lesson with a reliable comparison may cite them, and none of them
    /// says the chosen option is better or worse.
    pub mechanisms: Vec<Mechanism>,
}

/// The comparison of `chosen` with the best option in `analysis` of `view`.
/// `None` if `chosen` is not one of its options.
pub fn compare(view: &PlayerView, analysis: &Analysis, chosen: Action) -> Option<Comparison> {
    let option = analysis.options.iter().find(|o| o.action == chosen)?;
    let most = analysis
        .options
        .iter()
        .filter_map(|o| o.estimate.map(|e| e.samples))
        .max();
    let mut best: Option<(Action, f64)> = None;
    for o in &analysis.options {
        if let Some(e) = o.estimate.filter(|e| Some(e.samples) == most) {
            let value = utility(view, &e);
            if best.is_none_or(|(_, b)| value > b) {
                best = Some((o.action, value));
            }
        }
    }
    let reliability = if analysis.status != Status::Complete {
        Reliability::Incomplete
    } else {
        match option.estimate {
            None => Reliability::NotEvaluated,
            Some(e) if Some(e.samples) != most => Reliability::Screened,
            Some(_) => Reliability::Reliable,
        }
    };
    Some(Comparison {
        chosen,
        best: best.map(|(a, _)| a),
        reliability,
        mechanisms: mechanisms(view, chosen),
    })
}

/// Stable identifier of a lesson. A key is never reused for another
/// lesson; a changed meaning raises the version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LessonId {
    /// Stable key, also the stem of its translation keys.
    pub key: Cow<'static, str>,
    /// Version of its definition.
    pub version: u16,
}

/// What a lesson teaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LessonKind {
    /// A useful fact; claims nothing about which option is better. Valid
    /// with an incomplete search; may be shown before the move.
    Fact,
    /// A strategic mechanism explaining why options differ. After the move
    /// only, and only with a reliable [`Comparison`].
    Mechanism,
}

/// Whether a lesson may be taught at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Support {
    /// Backed by independently specified fixtures.
    Supported,
    /// Defined but not backed: never taught.
    Unsupported,
}

/// Where a lesson applies; an empty list means any.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Applicability {
    /// Phases of the decision.
    pub phases: Vec<Phase>,
    /// Roles of the deciding seat.
    pub roles: Vec<Role>,
    /// Kinds of the announced game; a non-empty list needs a game.
    pub games: Vec<GameKind>,
}

/// The kind of an exact fact ([`Observation`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FactKind {
    /// [`Observation::Void`].
    Void,
    /// [`Observation::TrumpsOut`].
    TrumpsOut,
    /// [`Observation::TrumpfSets`].
    TrumpfSets,
    /// [`Observation::AugenWon`].
    AugenWon,
    /// [`Observation::AugenOut`].
    AugenOut,
    /// [`Observation::SkatKnown`].
    SkatKnown,
    /// [`Observation::BidShown`].
    BidShown,
    /// [`Observation::KontraEligible`].
    KontraEligible,
    /// [`Observation::Structure`].
    Structure,
    /// [`Observation::GameValues`].
    GameValues,
    /// [`Observation::Commitment`].
    Commitment,
}

impl FactKind {
    /// The kind of `observation`.
    pub fn of(observation: &Observation) -> FactKind {
        match observation {
            Observation::Void { .. } => FactKind::Void,
            Observation::TrumpsOut { .. } => FactKind::TrumpsOut,
            Observation::TrumpfSets(_) => FactKind::TrumpfSets,
            Observation::AugenWon { .. } => FactKind::AugenWon,
            Observation::AugenOut { .. } => FactKind::AugenOut,
            Observation::SkatKnown { .. } => FactKind::SkatKnown,
            Observation::BidShown { .. } => FactKind::BidShown,
            Observation::KontraEligible { .. } => FactKind::KontraEligible,
            Observation::Structure(_) => FactKind::Structure,
            Observation::GameValues(_) => FactKind::GameValues,
            Observation::Commitment { .. } => FactKind::Commitment,
        }
    }
}

/// A predicate over the evidence of a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Condition {
    /// An exact fact of this kind is present (needs
    /// [`EvidenceClass::Exact`] permitted).
    Fact(FactKind),
    /// An exact fact of this kind rests on a card of the trick in progress
    /// or of the last completed trick: it is new since about the seat's
    /// last card. Cites only those facts.
    Recent(FactKind),
    /// A sample summary of this class is present (needs
    /// [`EvidenceClass::Sample`] permitted); proves nothing about the
    /// hidden cards.
    Sample(CardClass),
    /// The deciding seat knows its own discarded Skat.
    OwnSkat,
    /// The deciding seat sees an Ouvert declarer's cards.
    Revealed,
    /// The analysis finished.
    Complete,
    /// A bidding decision before the deciding seat's own first call.
    OpeningCall,
    /// The comparison of the chosen option carries an exact mechanism of
    /// this kind (needs [`EvidenceClass::Mechanism`] permitted; after the
    /// move only). Cites no evidence item.
    Mechanism(MechanismKind),
}

/// A lesson definition: when it applies and what it may rest on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lesson {
    /// Stable identifier.
    pub id: LessonId,
    /// Fact or mechanism.
    pub kind: LessonKind,
    /// Whether it may be taught.
    pub support: Support,
    /// Where it applies.
    pub applies: Applicability,
    /// The evidence classes it may cite.
    pub permitted: Vec<EvidenceClass>,
    /// All must hold.
    pub requires: Vec<Condition>,
    /// None may hold.
    pub excludes: Vec<Condition>,
}

/// The result of [`Lesson::check`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Check {
    /// The lesson applies, citing these items of the evidence (indices
    /// into [`Evidence::items`], ascending).
    Applies {
        /// The cited items.
        items: Vec<usize>,
    },
    /// The lesson does not apply.
    Fails(Unmet),
}

/// Why a lesson does not apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Unmet {
    /// The lesson is [`Support::Unsupported`].
    Unsupported,
    /// A mechanism before the move (no comparison).
    BeforeMove,
    /// A mechanism whose comparison is not reliable.
    Provisional(Reliability),
    /// A mechanism that does not permit [`EvidenceClass::Comparison`] and
    /// [`EvidenceClass::Mechanism`], or a fact lesson that permits them.
    Invalid,
    /// Another phase.
    Phase,
    /// Another role.
    Role,
    /// Another game, or none announced.
    Game,
    /// A required condition does not hold.
    Missing(Condition),
    /// An excluding condition holds.
    Excluded(Condition),
}

impl Lesson {
    /// Whether the lesson applies to `evidence`; `comparison` is the
    /// comparison of the chosen option after the move, `None` before it.
    pub fn check(&self, evidence: &Evidence, comparison: Option<&Comparison>) -> Check {
        if self.support == Support::Unsupported {
            return Check::Fails(Unmet::Unsupported);
        }
        let quality = [EvidenceClass::Comparison, EvidenceClass::Mechanism];
        let permits_quality = quality.iter().all(|c| self.permitted.contains(c));
        let cites_quality = quality.iter().any(|c| self.permitted.contains(c));
        match self.kind {
            LessonKind::Fact if cites_quality => return Check::Fails(Unmet::Invalid),
            LessonKind::Mechanism if !permits_quality => return Check::Fails(Unmet::Invalid),
            LessonKind::Fact => {}
            LessonKind::Mechanism => match comparison {
                None => return Check::Fails(Unmet::BeforeMove),
                Some(c) if c.reliability != Reliability::Reliable => {
                    return Check::Fails(Unmet::Provisional(c.reliability));
                }
                Some(_) => {}
            },
        }
        let s = &evidence.snapshot;
        let applies = &self.applies;
        if !applies.phases.is_empty() && !applies.phases.contains(&s.phase) {
            return Check::Fails(Unmet::Phase);
        }
        if !applies.roles.is_empty() && !applies.roles.contains(&s.role) {
            return Check::Fails(Unmet::Role);
        }
        if !applies.games.is_empty()
            && !s
                .game
                .is_some_and(|g| applies.games.contains(&GameKind::of(g)))
        {
            return Check::Fails(Unmet::Game);
        }
        let mut items = Vec::new();
        for &condition in &self.requires {
            match self.holds(evidence, comparison, condition) {
                Some(cited) => items.extend(cited),
                None => return Check::Fails(Unmet::Missing(condition)),
            }
        }
        if let Some(&condition) = self
            .excludes
            .iter()
            .find(|&&c| self.holds(evidence, comparison, c).is_some())
        {
            return Check::Fails(Unmet::Excluded(condition));
        }
        items.sort_unstable();
        items.dedup();
        Check::Applies { items }
    }

    /// The items citing `condition` if it holds (possibly none), `None` if
    /// it does not hold.
    fn holds(
        &self,
        evidence: &Evidence,
        comparison: Option<&Comparison>,
        condition: Condition,
    ) -> Option<Vec<usize>> {
        let cite = |class: EvidenceClass, matches: &dyn Fn(&Claim) -> bool| {
            let cited: Vec<usize> = evidence
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.class() == class && matches(&item.claim))
                .map(|(i, _)| i)
                .collect();
            (self.permitted.contains(&class) && !cited.is_empty()).then_some(cited)
        };
        match condition {
            Condition::Fact(kind) => cite(
                EvidenceClass::Exact,
                &|claim| matches!(claim, Claim::Fact(o) if FactKind::of(o) == kind),
            ),
            Condition::Recent(kind) => {
                let from = evidence.snapshot.tricks.saturating_sub(1);
                cite(
                    EvidenceClass::Exact,
                    &|claim| matches!(claim, Claim::Fact(o) if FactKind::of(o) == kind),
                )
                .map(|cited| {
                    cited
                        .into_iter()
                        .filter(|&i| {
                            evidence.items[i]
                                .sources
                                .iter()
                                .any(|s| matches!(s, Source::Trick { index, .. } if *index >= from))
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|cited| !cited.is_empty())
            }
            Condition::Sample(class) => cite(
                EvidenceClass::Sample,
                &|claim| matches!(claim, Claim::Sample(i) if i.class == class),
            ),
            Condition::OwnSkat => evidence.snapshot.own_skat.map(|_| Vec::new()),
            Condition::Revealed => evidence.snapshot.revealed.map(|_| Vec::new()),
            Condition::Complete => (evidence.status == Status::Complete).then(Vec::new),
            Condition::OpeningCall => {
                let s = &evidence.snapshot;
                (s.phase == Phase::Reizen && s.own_calls == 0).then(Vec::new)
            }
            Condition::Mechanism(kind) => (self.permitted.contains(&EvidenceClass::Mechanism)
                && comparison.is_some_and(|c| c.mechanisms.iter().any(|m| m.kind() == kind)))
            .then(Vec::new),
        }
    }
}

/// Key of the lesson on the remaining Trumpf ([`catalogue`]).
pub const REMAINING_TRUMPF: &str = "remaining-trumpf";

/// Key of the lesson on a void proven by not following suit ([`catalogue`]).
pub const PROVEN_VOID: &str = "proven-void";

/// Key of the mechanism lesson on a full side card an opponent playing
/// later can trump ([`catalogue`], [`crate::mechanism::Abstich`]).
pub const TRUMPF_VULNERABILITY: &str = "trumpf-vulnerability";

/// Key of the mechanism lesson on the partner beating the declarer's card
/// ([`catalogue`], [`crate::mechanism::Overtake`]).
pub const PARTNER_OVERTAKE: &str = "partner-overtake";

/// Key of the lesson on the values the game can have and the value it must
/// reach ([`catalogue`], [`crate::preplay`]).
pub const GAME_VALUE: &str = "game-value";

/// Key of the lesson on the own Buben, Trumpf and Spitzen per candidate
/// game ([`catalogue`], [`crate::preplay`]).
pub const HAND_STRUCTURE: &str = "hand-structure";

/// The lesson catalogue, in selection order (`docs/coach.md`). Backed by
/// the fixtures in `tests/exact_lessons.rs`, `tests/mechanisms.rs` and
/// `tests/preplay.rs`:
///
/// 1. [`TRUMPF_VULNERABILITY`] (mechanism): after the seat's Ass or Zehn of
///    the led side suit, winning so far, an opponent playing later in the
///    trick can trump it, possibly or certainly (Farbspiel and Grand,
///    declarer and defenders).
/// 2. [`PARTNER_OVERTAKE`] (mechanism): a defender played second to the
///    declarer's lead without taking the trick; the partner, playing last,
///    can beat the declarer's card, possibly or certainly (Farbspiel and
///    Grand).
/// 3. [`PROVEN_VOID`]: a seat did not follow the led class in the trick in
///    progress or the last completed trick, so it holds no card of that
///    class (Farbspiel, Grand, Null). It teaches an exclusion only.
/// 4. [`REMAINING_TRUMPF`]: a Trumpf was played in the trick in progress or
///    the last completed trick; where the Trumpf are, as far as the seat
///    knows (Farbspiel and Grand; Null has no Trumpf). Unseen Trumpf are
///    not assigned to a seat.
///
/// 5. [`GAME_VALUE`]: before card play, from the seat's second bidding call
///    on and when deciding Hand or pickup or announcing: the value range of
///    each contract with the own cards (an unknown Skat counts for the
///    Spitzen) and the value the game must reach to avoid Überreizen.
/// 6. [`HAND_STRUCTURE`]: at the seat's first bidding call and when
///    discarding: own Buben, Trumpf and Spitzen per Farbspiel and Grand.
///
/// Neither names a game, bid or discard as better; both are exact.
///
/// Mechanisms apply after the move only, with a reliable comparison, and
/// never because the chosen option scored higher: they need the exact
/// mechanism of the chosen card. Older facts do not trigger the fact
/// lessons again, so the same fact is not repeated at every card.
pub fn catalogue() -> Vec<Lesson> {
    let card_play = |games: Vec<GameKind>, kind: FactKind| Lesson {
        id: LessonId {
            key: Cow::Borrowed(match kind {
                FactKind::Void => PROVEN_VOID,
                _ => REMAINING_TRUMPF,
            }),
            version: 1,
        },
        kind: LessonKind::Fact,
        support: Support::Supported,
        applies: Applicability {
            phases: vec![Phase::Spiel],
            roles: Vec::new(),
            games,
        },
        permitted: vec![EvidenceClass::Exact],
        requires: vec![Condition::Recent(kind)],
        excludes: Vec::new(),
    };
    let pre_play = |key: &'static str, phases, requires, excludes| Lesson {
        id: LessonId {
            key: Cow::Borrowed(key),
            version: 1,
        },
        kind: LessonKind::Fact,
        support: Support::Supported,
        applies: Applicability {
            phases,
            roles: vec![Role::Bidder, Role::Declarer],
            games: Vec::new(),
        },
        permitted: vec![EvidenceClass::Exact],
        requires,
        excludes,
    };
    let mechanism = |roles: Vec<Role>, kind: MechanismKind| Lesson {
        id: LessonId {
            key: Cow::Borrowed(match kind {
                MechanismKind::Abstich => TRUMPF_VULNERABILITY,
                MechanismKind::Overtake => PARTNER_OVERTAKE,
            }),
            version: 1,
        },
        kind: LessonKind::Mechanism,
        support: Support::Supported,
        applies: Applicability {
            phases: vec![Phase::Spiel],
            roles,
            games: vec![GameKind::Suit, GameKind::Grand],
        },
        permitted: vec![
            EvidenceClass::Exact,
            EvidenceClass::Comparison,
            EvidenceClass::Mechanism,
        ],
        requires: vec![Condition::Mechanism(kind)],
        excludes: Vec::new(),
    };
    vec![
        mechanism(vec![Role::Declarer, Role::Defender], MechanismKind::Abstich),
        mechanism(vec![Role::Defender], MechanismKind::Overtake),
        card_play(
            vec![GameKind::Suit, GameKind::Grand, GameKind::Null],
            FactKind::Void,
        ),
        card_play(vec![GameKind::Suit, GameKind::Grand], FactKind::TrumpfSets),
        pre_play(
            GAME_VALUE,
            vec![Phase::Reizen, Phase::SkatAufnahme, Phase::Ansage],
            vec![
                Condition::Fact(FactKind::GameValues),
                Condition::Fact(FactKind::Commitment),
            ],
            vec![Condition::OpeningCall],
        ),
        pre_play(
            HAND_STRUCTURE,
            vec![Phase::Reizen, Phase::Druecken],
            vec![Condition::Fact(FactKind::Structure)],
            Vec::new(),
        ),
    ]
}
