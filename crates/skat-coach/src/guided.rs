// SPDX-License-Identifier: EUPL-1.2

//! Bounded guided Farbspiel, Grand and opening/second/last-seat Null
//! declarer and bounded defender card play, Reizen,
//! Skataufnahme, Drücken, Ansage and Kontra/Re lessons. Exact
//! rule provenance, never search ranking.

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
    /// Identify the current winner, or the own role/lead at an empty table.
    Orient,
    /// Identify the remaining seat (none in last seat), or a side master on lead.
    Check,
    /// Try a legal card that serves this branch's immediate purpose.
    Narrow,
    /// Reizen: is the value at stake within Easy's ceiling for the own hand?
    /// Skataufnahme: does the strength reach Easy's Hand threshold?
    /// Drücken: name a card Easy puts away first.
    /// Ansage: does the preferred game's value reach the public bid?
    /// Kontra/Re: does the own count reach Easy's condition (Farbspiel,
    /// Grand), or does the generic rule apply at all (Null)?
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
    /// Identify the public lead and own role at an empty trick.
    Lead {
        /// Seat entitled to lead.
        seat: Seat,
        /// Whether the learner is declarer.
        declarer: bool,
    },
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
    /// Kontra/Re threshold in Null: Easy's Trumpf/strength rule supplies an
    /// assessment of this contract. Never a strength comparison.
    Applies(bool),
    /// Skataufnahme/Drücken/Ansage orientation: which branch builds the candidate game.
    Game(GameBranch),
}

/// A rule-backed lesson. Never passed to the frontend before disclosure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lesson {
    /// Actual shared Easy outcome.
    pub rule: CardRule,
    /// Exact public source: current winning card or an empty trick lead.
    pub source: Source,
    /// Frozen own/permitted card knowledge, also used by counting aid.
    pub knowledge: Knowledge,
    /// Announced contract's rules.
    pub game: GameType,
    /// Single seat after the learner; none in last seat or on a two-reply lead.
    pub remaining: Option<Seat>,
    /// Learner's seat.
    pub seat: Seat,
    /// Own public role at an empty-trick lesson; absent in earlier families.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lead_declarer: Option<bool>,
}

/// Select only tested branches, with real alternatives and candidates:
/// UnderPartner/WinLast in Farbspiel and non-Ouvert Grand, plus Farbspiel and Grand Ouvert
/// defender UnderPartner and non-Ouvert Farbspiel/Grand WinEarly and last-seat FeedPartner. Grand
/// UnderPartner needs a defender, the partner ahead, the declarer still to act
/// and at least one losing card; early FeedPartner (Farbspiel or Grand, including Ouvert) requires a follow-class master; Fallback stays excluded.
/// Null: only the declarer on its own turn, opening lead or second/last to the current trick, with
/// at least two pure card actions and at least one losing card. A defender may
/// lead a trick, act second after either player leads, or last with the declarer ahead.
/// Null Ouvert admits defender leads, second-seat play after a declarer lead,
/// and last-seat play with the declarer ahead,
/// and the declarer's opening and second/last-seat turns after the reveal.
/// `Branch::Null` is broader; other defender turns, other Ouvert turns and
/// restricted or inconsistent views stay excluded here. Grand Ouvert declarer and defender after the declarer's lead
/// WinEarly, Farbspiel defender WinEarly after the declarer's lead, Grand and Farbspiel declarer WinLast, and Ouvert defender last-seat FeedPartner admit validated views. LeadMaster (including Ouvert
/// Farbspiel and Grand defenders and declarers) and Farbspiel and Grand declarer DrawTrumpf (including
/// Ouvert) admit tested own leads with an exact core-legal action list.
pub fn lesson(view: &PlayerView) -> Option<Lesson> {
    let contract = view.contract?;
    let game = contract.game;
    let grand = game == GameType::Grand && !contract.ouvert;
    let ouvert_grand_lead = game == GameType::Grand
        && contract.ouvert
        && (view.declarer == Some(view.seat) && view.declarer_cards == Some(view.hand)
            || view.declarer.is_some_and(|declarer| {
                declarer != view.seat
                    && view.declarer_cards.is_some_and(|cards| {
                        cards.len() == usize::from(view.hand_sizes[declarer.index()])
                            && (cards & view.hand).is_empty()
                            && (cards & Knowledge::from_view(view).played).is_empty()
                    })
            }))
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view.current_trick.cards == [None; 3];
    let ouvert_last = (game == GameType::Grand || matches!(game, GameType::Farbe(_)))
        && contract.ouvert
        && view.declarer == Some(view.seat)
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view.hand_game == Some(true)
        && view.own_skat.is_none()
        && view.declarer_cards == Some(view.hand)
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), Some(_), None])
        && view.current_trick.leader.map(|s| s.next().next()) == Some(view.seat)
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    let ouvert_feed = (game == GameType::Grand || matches!(game, GameType::Farbe(_)))
        && contract.ouvert
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view.declarer.is_some_and(|d| d != view.seat)
        && TrickState::of(game, &view.current_trick)
            .best_seat
            .is_some_and(|s| Some(s) != view.declarer)
        && view.hand_game == Some(true)
        && view.own_skat.is_none()
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), Some(_), None])
        && view.current_trick.leader.map(|s| s.next().next()) == Some(view.seat)
        && view.declarer_cards.is_some_and(|cards| {
            cards.len() == usize::from(view.hand_sizes[view.declarer.unwrap().index()])
                && (cards & view.hand).is_empty()
                && view
                    .current_trick
                    .cards
                    .iter()
                    .flatten()
                    .all(|c| !cards.contains(*c))
                && view
                    .tricks
                    .iter()
                    .flat_map(|t| t.cards)
                    .all(|c| !cards.contains(c))
        })
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    let ouvert_grand_early = game == GameType::Grand
        && contract.ouvert
        && (view.declarer == Some(view.seat) && view.declarer_cards == Some(view.hand)
            || view.declarer == view.current_trick.leader
                && view.declarer.is_some_and(|d| d != view.seat)
                && view.declarer_cards.is_some_and(|cards| {
                    cards.len() == usize::from(view.hand_sizes[view.declarer.unwrap().index()])
                        && (cards & view.hand).is_empty()
                        && (cards & Knowledge::from_view(view).played).is_empty()
                }))
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view.hand_game == Some(true)
        && view.own_skat.is_none()
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), None, None])
        && view.current_trick.leader.map(Seat::next) == Some(view.seat)
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    let ouvert_null_defender = contract.ouvert
        && view.declarer.is_some_and(|d| d != view.seat)
        && ((matches!(view.current_trick.cards, [Some(_), None, None])
            && view.current_trick.leader == view.declarer)
            || (matches!(view.current_trick.cards, [Some(_), Some(_), None])
                && TrickState::of(game, &view.current_trick).best_seat == view.declarer))
        && view.declarer_cards.is_some_and(|cards| {
            cards.len() == usize::from(view.hand_sizes[view.declarer.unwrap().index()])
                && (cards & view.hand).is_empty()
                && view
                    .current_trick
                    .cards
                    .iter()
                    .flatten()
                    .all(|c| !cards.contains(*c))
                && view
                    .tricks
                    .iter()
                    .flat_map(|t| t.cards)
                    .all(|c| !cards.contains(c))
        });
    let ouvert_under = (game == GameType::Grand || matches!(game, GameType::Farbe(_)))
        && contract.ouvert
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view.hand_game == Some(true)
        && view.own_skat.is_none()
        && view.declarer == Some(view.seat.next())
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), None, None])
        && view.current_trick.leader.map(Seat::next) == Some(view.seat)
        && view.declarer_cards.is_some_and(|cards| {
            cards.len() == view.hand_sizes[view.seat.next().index()] as usize
                && (cards & view.hand).is_empty()
                && (cards & Knowledge::from_view(view).played).is_empty()
        })
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    let null = game == GameType::Null
        && (!contract.ouvert
            || view.current_trick.cards == [None; 3]
            || ouvert_null_defender
            || (view.declarer == Some(view.seat)
                && view.declarer_cards == Some(view.hand)
                && (matches!(view.current_trick.cards, [Some(_), None, None])
                    || matches!(view.current_trick.cards, [Some(_), Some(_), None]))));
    if view.phase != Phase::Spiel
        || !(matches!(game, GameType::Farbe(_))
            || grand
            || ouvert_grand_lead
            || ouvert_last
            || ouvert_feed
            || ouvert_grand_early
            || ouvert_under
            || null)
    {
        return None;
    }
    // Checked before the shared rule, which assumes a declarer during play.
    let null_defender = null
        && view.declarer.is_some_and(|d| d != view.seat)
        && (matches!(view.current_trick.cards, [None, None, None])
            || matches!(view.current_trick.cards, [Some(_), None, None])
                && view.current_trick.leader.map(Seat::next) == Some(view.seat)
            || matches!(view.current_trick.cards, [Some(_), Some(_), None])
                && TrickState::of(game, &view.current_trick).best_seat == view.declarer)
        && view.own_skat.is_none();
    if null
        && ((!null_defender && view.declarer != Some(view.seat))
            || view.hand_game != Some(contract.hand)
            || contract.schneider
            || contract.schwarz
            || (contract.hand && view.own_skat.is_some())
            || (!null_defender && !contract.hand && view.own_skat.is_none_or(|s| s.len() != 2))
            || view.current_seat != Some(view.seat)
            || view.current_trick.cards[2].is_some()
            || (view.current_trick.cards[0].is_some()
                && view
                    .current_trick
                    .leader
                    .map(|s| Seat::from_index(s.index() + view.current_trick.len()))
                    != Some(view.seat)))
    {
        return None;
    }
    if null
        && view.legal_actions
            != skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>()
    {
        return None;
    }
    // Later Null leads are possible only for a defender: a declarer who wins loses.
    if null && view.current_trick.cards[0].is_none() {
        if view.current_trick.cards != [None; 3]
            || if view.tricks.is_empty() {
                view.current_trick.leader.is_some()
                    || view.seat != Seat::Vorhand
                    || view.hand.len() != 10
                    || view.hand_sizes != [10; 3]
            } else {
                !null_defender
                    || view.current_trick.leader != Some(view.seat)
                    || view.tricks.last().is_none_or(|t| t.winner != view.seat)
                    || view.tricks.len() >= 10
                    || view.hand.len() != 10 - view.tricks.len()
                    || view.hand_sizes != [(10 - view.tricks.len()) as u8; 3]
            }
            || view.result.is_some()
            || if contract.ouvert {
                if null_defender {
                    view.declarer_cards.is_none_or(|cards| {
                        cards.len() != view.hand.len()
                            || !(cards & view.hand).is_empty()
                            || view
                                .tricks
                                .iter()
                                .flat_map(|t| t.cards)
                                .any(|c| cards.contains(c))
                    })
                } else {
                    view.declarer_cards != Some(view.hand)
                }
            } else {
                view.declarer_cards.is_some()
            }
            || view.own_skat.is_some_and(|s| !(s & view.hand).is_empty())
            || view.legal_actions.len() < 2
        {
            return None;
        }
        let rule = baseline::rule(view)?;
        if rule.branch != Branch::Null {
            return None;
        }
        return Some(Lesson {
            rule,
            source: Source::Lead {
                index: view.tricks.len() as u8,
                seat: view.seat,
            },
            knowledge: Knowledge::from_view(view),
            game,
            remaining: None,
            seat: view.seat,
            lead_declarer: Some(!null_defender),
        });
    }
    // Lead rules assume a valid own turn, declarer and an exact pure legal
    // list. Reject malformed/restricted input before invoking the baseline.
    let suit = matches!(game, GameType::Farbe(_))
        && (!contract.ouvert
            || (contract.hand
                && contract.schneider
                && contract.schwarz
                && if view.declarer == Some(view.seat) {
                    view.declarer_cards == Some(view.hand)
                } else {
                    view.declarer.is_some_and(|declarer| {
                        view.declarer_cards.is_some_and(|cards| {
                            cards.len() == usize::from(view.hand_sizes[declarer.index()])
                                && (cards & view.hand).is_empty()
                                && (cards & Knowledge::from_view(view).played).is_empty()
                        })
                    })
                }));
    if (grand || ouvert_grand_lead || suit) && view.current_trick.cards[0].is_none() {
        if view.current_trick.cards != [None; 3]
            || view.current_trick.leader != view.tricks.last().map(|t| t.winner)
            || view.tricks.last().is_some_and(|t| t.winner != view.seat)
            || view.current_seat != Some(view.seat)
            || view.declarer.is_none()
            || view.hand_game != Some(contract.hand)
            || (contract.hand && view.own_skat.is_some())
            || (view.declarer != Some(view.seat) && view.own_skat.is_some())
            || (!contract.hand
                && view.declarer == Some(view.seat)
                && view.own_skat.is_none_or(|s| s.len() != 2))
            || view.legal_actions.len() < 2
            || view.legal_actions
                != skat_core::legal_cards(game, view.hand, None)
                    .iter()
                    .map(Action::Play)
                    .collect::<Vec<_>>()
        {
            return None;
        }
        let rule = baseline::rule(view)?;
        // DrawTrumpf is the declarer's priority branch (own Trumpf, Trumpf
        // not accounted for); the baseline never selects it for a defender.
        let admitted = match rule.branch {
            Branch::LeadMaster => !game.is_trump(rule.card),
            Branch::DrawTrumpf => {
                (grand || ouvert_grand_lead || suit)
                    && view.declarer == Some(view.seat)
                    && game.is_trump(rule.card)
            }
            _ => false,
        };
        if !admitted {
            return None;
        }
        return Some(Lesson {
            rule,
            source: Source::Lead {
                index: view.tricks.len() as u8,
                seat: view.seat,
            },
            knowledge: Knowledge::from_view(view),
            game,
            remaining: None,
            seat: view.seat,
            lead_declarer: Some(view.declarer == Some(view.seat)),
        });
    }
    let ouvert_suit_defender_early = matches!(game, GameType::Farbe(_))
        && contract.ouvert
        && contract.hand
        && contract.schneider
        && contract.schwarz
        && view
            .declarer
            .is_some_and(|d| d != view.seat && Some(d) == view.current_trick.leader)
        && view.declarer_cards.is_some_and(|cards| {
            cards.len() == usize::from(view.hand_sizes[view.declarer.unwrap().index()])
                && (cards & view.hand).is_empty()
                && (cards & Knowledge::from_view(view).played).is_empty()
        });
    // Only the new Farbspiel WinEarly family uses this strict admission.
    // Preserve the existing families and their shared branch precedence.
    let suit_early = matches!(game, GameType::Farbe(_))
        && (!contract.ouvert
            || (view.declarer == Some(view.seat)
                && contract.hand
                && contract.schneider
                && contract.schwarz
                && view.declarer_cards == Some(view.hand))
            || ouvert_suit_defender_early)
        && view.current_seat == Some(view.seat)
        && view.current_trick.cards[0].is_some()
        && view.current_trick.cards[1..] == [None, None]
        && view.current_trick.leader.map(Seat::next) == Some(view.seat)
        && view.declarer.is_some()
        && view.hand_game == Some(contract.hand)
        && !(contract.hand && view.own_skat.is_some())
        && !(view.declarer != Some(view.seat) && view.own_skat.is_some())
        && (contract.hand
            || view.declarer != Some(view.seat)
            || view
                .own_skat
                .is_some_and(|s| s.len() == 2 && (s & view.hand).is_empty()))
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    // The new Farbspiel family requires a valid last-seat defender input.
    // In particular, baseline play assumes a declarer: never call it on a
    // missing-declarer synthetic view.
    let suit_feed = matches!(game, GameType::Farbe(_))
        && !contract.ouvert
        && view.declarer.is_some_and(|d| d != view.seat)
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), Some(_), None])
        && view.current_trick.leader.map(|s| s.next().next()) == Some(view.seat)
        && view.hand_game == Some(contract.hand)
        && view.own_skat.is_none()
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    // Early feeding is not last-seat feeding: the remaining declarer can
    // still trump a side master. Admit only a complete acting-defender view.
    let feed_early = (grand || suit || ouvert_under)
        && view.declarer.is_some_and(|d| d != view.seat)
        && view.current_seat == Some(view.seat)
        && matches!(view.current_trick.cards, [Some(_), None, None])
        && view.current_trick.leader.map(Seat::next) == Some(view.seat)
        && view.current_trick.leader != view.declarer
        && view.hand_game == Some(contract.hand)
        && view.own_skat.is_none()
        && view.legal_actions
            == skat_core::legal_cards(game, view.hand, view.current_trick.led())
                .iter()
                .map(Action::Play)
                .collect::<Vec<_>>();
    view.declarer?;
    let rule = baseline::rule(view)?;
    if (ouvert_feed && rule.branch != Branch::FeedPartner)
        || (ouvert_grand_early && rule.branch != Branch::WinEarly)
        || (ouvert_last && rule.branch != Branch::WinLast)
        || (contract.ouvert
            && matches!(game, GameType::Farbe(_))
            && view.declarer == Some(view.seat)
            && rule.branch == Branch::WinLast
            && !ouvert_last)
        || (ouvert_under
            && rule.branch != Branch::UnderPartner
            && !(rule.branch == Branch::FeedPartner && feed_early && rule.table_master))
        || (contract.ouvert
            && matches!(game, GameType::Farbe(_))
            && view.declarer != Some(view.seat)
            && rule.branch == Branch::UnderPartner
            && !ouvert_under)
        || view.legal_actions.len() < 2
        || rule.candidates.is_empty()
        || (rule.branch == Branch::UnderPartner && rule.losers.is_empty())
        || !(matches!(rule.branch, Branch::UnderPartner | Branch::WinLast)
            || ((grand || suit_early || ouvert_grand_early) && rule.branch == Branch::WinEarly)
            || (rule.branch == Branch::FeedPartner
                && (((grand || suit_feed || ouvert_feed) && rule.last)
                    || (feed_early && rule.partner_ahead && rule.table_master)))
            || (null
                && rule.branch == Branch::Null
                && (!rule.losers.is_empty()
                    || (null_defender && rule.winner != view.declarer && !rule.last))))
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
        lead_declarer: None,
    })
}

impl Lesson {
    /// Checks the immediate factual/policy task, not universal move quality.
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        if let Some(role) = self.lead_declarer {
            return match (step, answer) {
                (Step::Orient, Answer::Lead { seat, declarer }) => {
                    seat == self.seat && declarer == role
                }
                (Step::Check, Answer::Card(c)) => self.rule.candidates.contains(&c),
                (Step::Narrow, Answer::Card(c)) => c == self.rule.card,
                _ => false,
            };
        }
        match (step, answer) {
            (Step::Orient, Answer::Seat(seat)) => seat == self.rule.winner,
            (Step::Check, Answer::Seat(seat)) => seat == self.remaining,
            (Step::Narrow, Answer::Card(c))
                if self.game == GameType::Null
                    && self.rule.tie_break == baseline::TieBreak::LowestCostFirst =>
            {
                c == self.rule.card
            }
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

/// Condition on Easy's selected early winner, never on every candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EarlyCondition {
    /// A Trumpf (only Buben in Grand); a higher remaining Trumpf may overtrump.
    Trumpf,
    /// No unseen higher card of this follow class; trumping remains possible.
    FollowClassMaster,
}

impl Lesson {
    /// Explain the shared predicate only for the actual selected card.
    pub fn early_condition(&self) -> Option<EarlyCondition> {
        (self.rule.branch == Branch::WinEarly).then(|| {
            if self.game.is_trump(self.rule.card) {
                EarlyCondition::Trumpf
            } else {
                debug_assert!(self.knowledge.is_master(self.rule.card));
                EarlyCondition::FollowClassMaster
            }
        })
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

/// A Farbspiel, Grand or Null Kontra or immediate Re lesson (including Ouvert Re) from
/// Easy's shared rule. Never passed to the frontend before disclosure.
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
    /// A Null Ouvert contract after the reveal (Farbspiel/Grand Ouvert is supported).
    Ouvert,
    /// Absent contract or a restricted/inconsistent permitted view.
    Restricted,
}

/// A core-offered Kontra or the declarer's immediate Re reply. Uses the own cards, the own known Skat and
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
    let contract = view.contract.ok_or(KontraExclusion::Restricted)?;
    if view.legal_actions.contains(&Action::KeinKontra)
        && (!contract.ouvert || view.legal_actions != [Action::Kontra, Action::KeinKontra])
    {
        return Err(KontraExclusion::Restricted);
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

    /// Whether Easy's generic Trumpf/strength rule assesses this contract:
    /// not in Null or in the Ouvert round before the reveal.
    pub fn applies(&self) -> bool {
        !self.rule.null && !self.rule.legal.contains(&Action::KeinKontra)
    }

    /// Orientation asks for the factor after the call (public rule fact). The
    /// threshold answer is whether Easy's condition holds (Farbspiel, Grand)
    /// or, in Null, whether the generic rule applies at all.
    pub fn check(&self, step: Step, answer: Answer) -> bool {
        match (step, answer) {
            (Step::Orient, Answer::Factor(f)) => f == self.factor(),
            (Step::Threshold, Answer::Within(w)) if self.applies() => w == self.rule.wants,
            (Step::Threshold, Answer::Applies(a)) if !self.applies() => a == self.applies(),
            _ => false,
        }
    }

    /// The learner's own action at the original decision: what the next
    /// public transition is.
    pub fn awaits(&self, action: Action) -> Option<KontraAwait> {
        match (self.rule.call, action) {
            (KontraCall::Kontra, Action::Kontra) => Some(KontraAwait::Reply),
            (KontraCall::Kontra, Action::Play(_)) => Some(KontraAwait::Declined),
            (KontraCall::Kontra, Action::KeinKontra) => Some(KontraAwait::DeclinedCall),
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
    /// The learner declined in the Ouvert round.
    DeclinedCall,
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
    /// The learner declined before the Ouvert reveal.
    DeclinedCall,
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
        (KontraAwait::DeclinedCall, Event::KeinKontra { seat: s }) if s == seat => {
            Some(KontraObservation::DeclinedCall)
        }
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
    /// Led follow class; a Bube belongs to Trumpf in Farbspiel and Grand.
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
