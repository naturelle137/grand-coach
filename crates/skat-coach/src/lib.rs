// SPDX-License-Identifier: EUPL-1.2

//! Live coach (M5): turns one [`Analysis`] of `skat-analysis` into a
//! structured [`Explanation`] (D-13): what to think about, which options
//! there are and how they compare, with the reasons the estimates give.
//!
//! The coach does not compute strategy of its own ("coach and AI do not
//! compute separately"): every fact is an analysis observation, every
//! inferred distribution an analysis inference, every comparison an
//! analysis estimate. The coach only ranks, groups and labels them by the
//! documented thresholds below (`docs/coach.md`, D-48). It returns no
//! texts; frontends translate the structure.
//!
//! No telling ahead (D-49): before the move a frontend shows only
//! [`Hints`], revealed stage by stage and never naming the best option;
//! after the move [`review`] judges the chosen action, and at the end of
//! the game [`moments`] picks zero to three teachable moments, one per
//! [`Concept`]. The player may choose one concept as focus; the coach
//! never diagnoses one.
//!
//! Information boundary: [`explain`] takes the deciding seat's
//! [`PlayerView`] and the analysis of exactly that view (D-23).
//!
//! Lessons (D-53): the analysis gathers the permitted [`Evidence`] and
//! checks lesson predicates ([`Lesson::check`]); the coach only selects a
//! lesson with [`teach`] and falls back to the model comparison when none
//! applies. [`lesson_content`] hands the cited facts of a taught lesson,
//! or the exact mechanisms of the chosen card, to the frontend as plain
//! data. Before card play these are the own hand structure and the legal
//! game values ([`LessonContent::HandStructure`],
//! [`LessonContent::GameValue`]), never a preferred bid, game or discard.

use skat_analysis::lesson::{catalogue, compare as compare_options, gather};
use skat_analysis::{Budget, Cancel, analyze, utility};
pub use skat_core::Spitzen;
use skat_core::{Action, Card, Contract, GameType, KontraLevel, Phase, PlayerView, Seat};

pub use skat_analysis::lesson::{
    Check, Claim, Comparison, Evidence, EvidenceClass, GAME_VALUE, HAND_STRUCTURE, Item, Lesson,
    LessonId, LessonKind, PARTNER_OVERTAKE, PROVEN_VOID, REMAINING_TRUMPF, Reliability, Role,
    Snapshot, Source, TRUMPF_VULNERABILITY,
};
pub use skat_analysis::mechanism::{Abstich, Certainty, Mechanism, MechanismKind, Overtake};
pub use skat_analysis::preplay::{ContractValue, GameValues, Range, Reach, SkatState, Structure};

pub use skat_analysis::sampling::Place;
pub use skat_analysis::{
    Analysis, Assumption, CardClass, Estimate, Inference, Observation, Status, TrumpfSets,
};
pub use skat_core::Follow;

/// Differences up to this many Seeger-Fabian points count as equal
/// (double-dummy values resolve scoring classes, so options often tie).
pub const TIE_POINTS: f64 = 1.0;

/// An option is weak, worth a warning, when it is expected to cost at
/// least this many Seeger-Fabian points against the best option, and more
/// than its noise margin.
pub const WEAK_POINTS: f64 = 10.0;

/// The noise margin of a comparison is this many combined standard errors.
pub const MARGIN_SE: f64 = 2.0;

/// Win chances or throw-in probabilities that differ by less than this are
/// not mentioned as a reason.
pub const CHANCE_STEP: f64 = 0.05;

/// A worse choice counts as a bad moment of the debriefing only if it is
/// expected to cost at least this many Seeger-Fabian points; smaller
/// losses are a neutral moment at most and get the note after the move.
pub const MOMENT_POINTS: f64 = 5.0;

/// At most this many primary moments in the end-of-game debriefing, at
/// most one per [`Concept`] (see [`moments`]). A product default to test
/// with learners, not a research-established optimum.
pub const PRIMARY_MOMENTS: usize = 3;

/// A teaching concept: one per supported lesson of the catalogue. The
/// player may choose one as the optional focus of a learning game; the
/// coach never chooses or diagnoses one by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Concept {
    /// [`HAND_STRUCTURE`]: own Buben, Trumpf and Spitzen.
    HandStructure,
    /// [`GAME_VALUE`]: legal game values against the bid.
    GameValue,
    /// [`PROVEN_VOID`]: a seat that did not follow holds no such card.
    ProvenVoid,
    /// [`REMAINING_TRUMPF`]: where the Trumpf are, as far as known.
    RemainingTrumpf,
    /// [`TRUMPF_VULNERABILITY`]: a later seat can trump a full side card.
    TrumpfVulnerability,
    /// [`PARTNER_OVERTAKE`]: the partner, playing last, can beat the
    /// declarer's card.
    PartnerOvertake,
}

impl Concept {
    /// Every concept, in the order of a game: before card play first.
    pub const ALL: [Concept; 6] = [
        Concept::HandStructure,
        Concept::GameValue,
        Concept::ProvenVoid,
        Concept::RemainingTrumpf,
        Concept::TrumpfVulnerability,
        Concept::PartnerOvertake,
    ];

    /// The key of its lesson ([`LessonId::key`]).
    pub fn key(self) -> &'static str {
        match self {
            Concept::HandStructure => HAND_STRUCTURE,
            Concept::GameValue => GAME_VALUE,
            Concept::ProvenVoid => PROVEN_VOID,
            Concept::RemainingTrumpf => REMAINING_TRUMPF,
            Concept::TrumpfVulnerability => TRUMPF_VULNERABILITY,
            Concept::PartnerOvertake => PARTNER_OVERTAKE,
        }
    }

    /// The concept of a lesson; `None` for a lesson outside the catalogue.
    pub fn of(id: &LessonId) -> Option<Concept> {
        Concept::ALL.into_iter().find(|c| c.key() == id.key)
    }

    /// A mechanism lesson (after the move, with a reliable comparison).
    pub fn mechanism(self) -> bool {
        matches!(
            self,
            Concept::TrumpfVulnerability | Concept::PartnerOvertake
        )
    }
}

/// A lesson that applies at a decision, with what it says (D-53).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Taught {
    /// Its concept.
    pub concept: Concept,
    /// The lesson and the evidence it cites.
    pub teaching: Teaching,
    /// What it says.
    pub content: LessonContent,
}

/// Every lesson of `lessons` that applies to `evidence` (with the chosen
/// option's `comparison` after the move, `None` before it), has a
/// [`Concept`] and content, in the order of `lessons`.
pub fn taught(
    lessons: &[Lesson],
    evidence: &Evidence,
    comparison: Option<&Comparison>,
) -> Vec<Taught> {
    lessons
        .iter()
        .filter_map(|lesson| {
            let Check::Applies { items } = lesson.check(evidence, comparison) else {
                return None;
            };
            let concept = Concept::of(&lesson.id)?;
            let teaching = Teaching::Lesson {
                id: lesson.id.clone(),
                items,
            };
            let content = lesson_content(&teaching, evidence, comparison)?;
            Some(Taught {
                concept,
                teaching,
                content,
            })
        })
        .collect()
}

/// The lesson to show of `taught`: the one of `focus` if it applies,
/// otherwise the first.
pub fn pick(taught: &[Taught], focus: Option<Concept>) -> Option<&Taught> {
    taught
        .iter()
        .find(|t| Some(t.concept) == focus)
        .or_else(|| taught.first())
}

/// The kind of decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Decision {
    /// Say a bid value or pass (asking seat).
    Bid,
    /// Hold or pass (answering seat).
    Answer,
    /// Play Hand or pick up the Skat.
    Pickup,
    /// Put two cards into the Skat (Drücken).
    Discard,
    /// Announce the game.
    Announce,
    /// Play a card; Kontra may be one of the options.
    Card,
    /// Kontra or not, in the Ouvert decision round.
    Kontra,
    /// Answer Kontra with Re or not.
    Re,
}

/// What the declarer has to achieve in this game (rule facts of the
/// contract); the defenders try to prevent it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Objective {
    /// No game announced yet: the analysis values each option by how the
    /// game that follows is likely to end.
    Undecided,
    /// At least this many Augen (61, or 90 with Schneider announced).
    Augen {
        /// Augen the declarer needs.
        target: u8,
    },
    /// Every trick (Schwarz announced).
    AllTricks,
    /// No trick at all (Null).
    NoTrick,
}

/// Something worth thinking about, with its kind of evidence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Consideration {
    /// A fact the seat knows exactly (card counting).
    Known(Observation),
    /// An estimate about hidden cards from the opponent model.
    Inferred(Inferred),
}

/// An inference with its qualitative reading.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferred {
    /// The analysis inference.
    pub inference: Inference,
    /// The expected count in words.
    pub amount: Amount,
}

/// Qualitative reading of an expected number of cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Amount {
    /// Expected fewer than 0.5.
    Hardly,
    /// Expected 0.5 to below 1.5.
    AboutOne,
    /// Expected 1.5 or more.
    Several,
}

impl Amount {
    /// Reading of an expected count.
    pub fn of(expected: f64) -> Amount {
        if expected < 0.5 {
            Amount::Hardly
        } else if expected < 1.5 {
            Amount::AboutOne
        } else {
            Amount::Several
        }
    }
}

/// How an option compares with the best one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Judgement {
    /// The highest value.
    Best,
    /// Equal within [`TIE_POINTS`] or within the noise margin.
    Equivalent,
    /// Worse beyond the noise margin, by less than [`WEAK_POINTS`].
    Worse,
    /// Worse beyond the noise margin and by at least [`WEAK_POINTS`].
    Weak,
    /// Evaluated on fewer distributions than the best options (Drücken
    /// screening); not compared.
    Screened,
    /// No distribution finished for it.
    NotEvaluated,
}

/// Why one option differs from another, read off the two estimates.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Reason {
    /// The deciding seat's party wins with this probability (this option)
    /// against `other` (the compared option).
    WinChance {
        /// This option.
        this: f64,
        /// The compared option.
        other: f64,
    },
    /// Expected value for the seat in Seeger-Fabian points (for a defender
    /// including half of the declarer's lost game points, see
    /// `skat_analysis::utility`).
    Points {
        /// This option.
        this: f64,
        /// The compared option.
        other: f64,
    },
    /// Expected game points of the declarer's list entry (with Kontra/Re);
    /// the defenders want them low.
    DeclarerPoints {
        /// This option.
        this: f64,
        /// The compared option.
        other: f64,
    },
    /// Probability that all pass and the deal is thrown in.
    ThrownIn {
        /// This option.
        this: f64,
        /// The compared option.
        other: f64,
    },
    /// Both options lead to the same value (within [`TIE_POINTS`]).
    SameValue,
    /// The difference is within the noise margin of the estimates.
    WithinUncertainty,
}

/// One legal option, compared.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComparedOption {
    /// The action.
    pub action: Action,
    /// Whether a player would ordinarily weigh it ([`natural`]).
    pub natural: bool,
    /// Its standing.
    pub judgement: Judgement,
    /// The analysis estimate, unchanged.
    pub estimate: Option<Estimate>,
    /// Value used for ranking (`skat_analysis::utility`).
    pub value: Option<f64>,
    /// How much lower its value is than the best option's (0 for the best).
    pub gap: f64,
    /// Noise margin of that gap: [`MARGIN_SE`] combined standard errors.
    pub margin: f64,
    /// The option the reasons compare with: the best option, or for the
    /// best option the runner-up.
    pub compared_with: Option<Action>,
    /// Differences to `compared_with`.
    pub reasons: Vec<Reason>,
}

/// Overall reading of the comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Verdict {
    /// Only one legal option.
    Forced,
    /// The best option is better than every other beyond the noise margin.
    Clear,
    /// Several options are equivalent to the best.
    Close {
        /// Number of options equivalent to the best (without it).
        equivalent: u8,
    },
    /// No option has an estimate.
    Unknown,
}

/// What the estimates rest on.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Basis {
    /// Distributions evaluated completely.
    pub finished: u32,
    /// Distributions planned.
    pub planned: u32,
    /// `true` if every consistent distribution was enumerated.
    pub enumerated: bool,
    /// Effective sample size of the bidding-evidence weights.
    pub effective_samples: f64,
}

/// The coach's structured explanation of one decision.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Explanation {
    /// The deciding seat.
    pub seat: Seat,
    /// The kind of decision.
    pub decision: Decision,
    /// The seat's role.
    pub role: Role,
    /// What the declarer has to achieve.
    pub objective: Objective,
    /// Status of the analysis. Only [`Status::Complete`] backs a
    /// recommendation, an important-decision mark or a warning.
    pub status: Status,
    /// The legal actions in the order of the view: the neutral order in
    /// which [`Hints`] list them, independent of their values.
    pub actions: Vec<Action>,
    /// The bid value reached when deciding (to name "hold" later).
    pub bid: u16,
    /// Hand game, once decided (to name announcements later).
    pub hand_game: Option<bool>,
    /// The trick being played, from 1, during card play.
    pub trick: Option<u8>,
    /// Facts and inferences worth thinking about, facts first.
    pub considerations: Vec<Consideration>,
    /// Every legal option, best first.
    pub options: Vec<ComparedOption>,
    /// Overall reading.
    pub verdict: Verdict,
    /// Whether this is an important decision (D-48): complete, and a
    /// natural option is weak.
    pub important: bool,
    /// What the estimates rest on.
    pub basis: Basis,
    /// Model assumptions behind the estimates.
    pub assumptions: Vec<Assumption>,
    /// The permitted evidence of the decision, with provenance.
    pub evidence: Evidence,
    /// The comparison of each legal action with the best, in the order of
    /// `actions`.
    pub comparisons: Vec<Comparison>,
}

/// The lesson the coach teaches at a decision, if any (D-53).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Teaching {
    /// A supported lesson applies.
    Lesson {
        /// The lesson.
        id: LessonId,
        /// The evidence it cites (indices into [`Evidence::items`]).
        items: Vec<usize>,
    },
    /// No supported lesson applies: only the model comparison is
    /// available, no strategic reason.
    Fallback,
}

/// The first lesson of `lessons` that applies to `evidence` (with the
/// chosen option's `comparison` after the move, `None` before it), or the
/// fallback.
pub fn teach(lessons: &[Lesson], evidence: &Evidence, comparison: Option<&Comparison>) -> Teaching {
    lessons
        .iter()
        .find_map(|lesson| match lesson.check(evidence, comparison) {
            Check::Applies { items } => Some(Teaching::Lesson {
                id: lesson.id.clone(),
                items,
            }),
            Check::Fails(_) => None,
        })
        .unwrap_or(Teaching::Fallback)
}

/// What a taught lesson says, as plain data for the frontend's copy
/// (`docs/coach.md`): the attention question before the move, the
/// evidence, exception and transfer question after it. Taken from the
/// cited exact evidence or the comparison's exact mechanisms only; nothing
/// is computed here.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LessonContent {
    /// [`REMAINING_TRUMPF`]: where the Trumpf are as far as the seat knows.
    RemainingTrumpf {
        /// The game (Farbspiel or Grand).
        game: GameType,
        /// The Trumpf by what the seat knows; unseen ones are not assigned
        /// to a seat.
        sets: TrumpfSets,
    },
    /// [`PROVEN_VOID`]: seats that did not follow the led class.
    ProvenVoid {
        /// The game.
        game: GameType,
        /// The new voids, each with the card that proved it.
        voids: Vec<ProvenVoid>,
    },
    /// [`TRUMPF_VULNERABILITY`]: after the move only. Opponents playing
    /// later in the trick who can trump the chosen side card, each possibly
    /// or certainly; never that they will.
    TrumpfVulnerability {
        /// The game (Farbspiel or Grand).
        game: GameType,
        /// One entry per opponent, in play order.
        threats: Vec<Abstich>,
    },
    /// [`PARTNER_OVERTAKE`]: after the move only. The partner, playing
    /// last, can beat the declarer's card, possibly or certainly.
    PartnerOvertake {
        /// The game (Farbspiel or Grand).
        game: GameType,
        /// The mechanism.
        overtake: Overtake,
    },
    /// [`HAND_STRUCTURE`]: own Buben, Trumpf and Spitzen per Farbspiel and
    /// Grand, in the order Kreuz, Pik, Herz, Karo, Grand. Names no game as
    /// better.
    HandStructure {
        /// One entry per candidate game.
        games: Vec<Structure>,
    },
    /// [`GAME_VALUE`]: the value the game must reach and every contract
    /// still possible with its value range, in the order Kreuz, Pik, Herz,
    /// Karo, Grand, Null. Reaching the value only avoids Überreizen; it
    /// says nothing about winning and names no contract as better.
    GameValue {
        /// The value the game must reach.
        value: u16,
        /// During bidding: `value` is what a Bid or Hold commits to, not
        /// the bid reached.
        bidding: bool,
        /// Kontra/Re is configured; it does not change the game value.
        kontra_re: bool,
        /// What the seat knows about the Skat.
        skat: SkatState,
        /// Per game type, its contracts measured against `value`.
        games: Vec<(GameType, Vec<ContractValue>)>,
    },
}

/// A void proven by not following suit, with its evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenVoid {
    /// The seat that did not follow.
    pub seat: Seat,
    /// The class it holds no card of.
    pub follow: Follow,
    /// The trick, from 1.
    pub trick: u8,
    /// The card led.
    pub led: Card,
    /// The card the seat played instead.
    pub card: Card,
}

/// The content of `teaching` from the items of `evidence` it cites, or for
/// a mechanism from the mechanisms of the chosen option's `comparison`
/// (only if it is reliable); `None` for the fallback, an unknown lesson or
/// missing evidence (then the frontend shows no lesson).
pub fn lesson_content(
    teaching: &Teaching,
    evidence: &Evidence,
    comparison: Option<&Comparison>,
) -> Option<LessonContent> {
    let Teaching::Lesson { id, items } = teaching else {
        return None;
    };
    let cited = || items.iter().filter_map(|&i| evidence.items.get(i));
    match id.key.as_ref() {
        HAND_STRUCTURE => {
            let games: Vec<Structure> = cited()
                .filter_map(|item| match item.claim {
                    Claim::Fact(Observation::Structure(s)) => Some(s),
                    _ => None,
                })
                .collect();
            return (!games.is_empty()).then_some(LessonContent::HandStructure { games });
        }
        GAME_VALUE => {
            let (value, kontra_re) = cited().find_map(|item| match item.claim {
                Claim::Fact(Observation::Commitment { value, kontra_re }) => {
                    Some((value, kontra_re))
                }
                _ => None,
            })?;
            let values: Vec<GameValues> = cited()
                .filter_map(|item| match item.claim {
                    Claim::Fact(Observation::GameValues(v)) => Some(v),
                    _ => None,
                })
                .collect();
            let skat = values.first()?.skat;
            return Some(LessonContent::GameValue {
                value,
                bidding: evidence.snapshot.phase == Phase::Reizen,
                kontra_re,
                skat,
                games: values.iter().map(|v| (v.game, v.against(value))).collect(),
            });
        }
        _ => {}
    }
    let game = evidence.snapshot.game?;
    let mechanisms = || {
        comparison
            .filter(|c| c.reliability == Reliability::Reliable)
            .map_or(&[][..], |c| c.mechanisms.as_slice())
            .iter()
    };
    match id.key.as_ref() {
        TRUMPF_VULNERABILITY => {
            let threats: Vec<Abstich> = mechanisms()
                .filter_map(|m| match m {
                    Mechanism::Abstich(a) => Some(*a),
                    Mechanism::Overtake(_) => None,
                })
                .collect();
            (!threats.is_empty()).then_some(LessonContent::TrumpfVulnerability { game, threats })
        }
        PARTNER_OVERTAKE => mechanisms().find_map(|m| match m {
            Mechanism::Overtake(o) => Some(LessonContent::PartnerOvertake { game, overtake: *o }),
            Mechanism::Abstich(_) => None,
        }),
        REMAINING_TRUMPF => cited().find_map(|item| match item.claim {
            Claim::Fact(Observation::TrumpfSets(sets)) => {
                Some(LessonContent::RemainingTrumpf { game, sets })
            }
            _ => None,
        }),
        PROVEN_VOID => {
            let voids: Vec<ProvenVoid> = cited()
                .filter_map(|item| {
                    let Claim::Fact(Observation::Void { seat, follow }) = item.claim else {
                        return None;
                    };
                    item.sources.iter().find_map(|s| match *s {
                        Source::Trick {
                            index,
                            seat: s,
                            led,
                            card,
                        } if s == seat => Some(ProvenVoid {
                            seat,
                            follow,
                            trick: index + 1,
                            led,
                            card,
                        }),
                        _ => None,
                    })
                })
                .collect();
            (!voids.is_empty()).then_some(LessonContent::ProvenVoid { game, voids })
        }
        _ => None,
    }
}

/// Why the coach warns before an action. It names no alternative (D-49):
/// the better option is shown only after the game.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Warning {
    /// The chosen, weak option; `gap` is the expected loss.
    pub chosen: ComparedOption,
}

/// How much of the analysis the coach reveals before the move (D-49), one
/// stage per request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stage {
    /// Objective, sure facts and likely inferences.
    Considerations,
    /// Plus the options in neutral order and a qualitative hint.
    Situation,
    /// Plus every option's figures, still in neutral order.
    Figures,
}

impl Stage {
    /// The stage revealed after this one.
    pub fn next(self) -> Option<Stage> {
        match self {
            Stage::Considerations => Some(Stage::Situation),
            Stage::Situation => Some(Stage::Figures),
            Stage::Figures => None,
        }
    }
}

/// Qualitative reading of a decision that names no option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Situation {
    /// Only one legal option.
    Forced,
    /// One option is better than all others beyond the noise margin.
    OneStandsOut,
    /// This many options (two or more) are about equally good, the best
    /// among them.
    SeveralEqual {
        /// Number of options about as good as the best, the best included.
        count: u8,
    },
    /// No reliable reading (analysis incomplete or nothing evaluated).
    Unclear,
}

/// Stage [`Stage::Situation`]: the options and a hint, no judgement.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SituationHint {
    /// The qualitative reading.
    pub situation: Situation,
    /// Important decision: a natural option costs clearly (D-48).
    pub important: bool,
    /// The legal options in neutral order.
    pub options: Vec<Action>,
}

/// Stage [`Stage::Figures`]: one option's estimate, without judgement.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionFigures {
    /// The option.
    pub action: Action,
    /// The analysis estimate, if any distribution finished for it.
    pub estimate: Option<Estimate>,
}

/// What the coach shows before the move: the revealed stages only. It
/// never ranks or labels the options (D-49).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hints {
    /// The revealed stage.
    pub stage: Stage,
    /// The kind of decision.
    pub decision: Decision,
    /// The seat's role.
    pub role: Role,
    /// What the declarer has to achieve.
    pub objective: Objective,
    /// Status of the analysis.
    pub status: Status,
    /// Facts and inferences worth thinking about, facts first.
    pub considerations: Vec<Consideration>,
    /// From [`Stage::Situation`] on.
    pub situation: Option<SituationHint>,
    /// From [`Stage::Figures`] on, in the order of `situation.options`.
    pub figures: Option<Vec<OptionFigures>>,
    /// What the estimates rest on.
    pub basis: Basis,
    /// Model assumptions behind the estimates.
    pub assumptions: Vec<Assumption>,
    /// The permitted evidence; it names no action.
    pub evidence: Evidence,
    /// A fact lesson at most: mechanisms need the move first. With a
    /// focus, the focused lesson if it applies.
    pub teaching: Teaching,
    /// What that lesson says; before the move a frontend shows only its
    /// attention question.
    pub lesson: Option<LessonContent>,
    /// The concept of that lesson.
    pub concept: Option<Concept>,
    /// From [`Stage::Situation`] on: ask the player to check every option
    /// against what they know. The question is the same for every option
    /// and every ranking (D-49), so it implies no preferred action.
    pub compare_question: bool,
}

impl Hints {
    /// Whether a supported lesson backs these hints (pedagogical
    /// eligibility, separate from [`SituationHint::important`]).
    pub fn teachable(&self) -> bool {
        self.lesson.is_some()
    }
}

/// The coach's hints for `e` up to `stage`, without a focus.
pub fn hints(e: &Explanation, stage: Stage) -> Hints {
    hints_for(e, stage, None)
}

/// The coach's hints for `e` up to `stage`. With a `focus`, its lesson is
/// shown when it applies; otherwise the first lesson that applies. Only
/// fact lessons can apply before the move, and they depend on the evidence
/// alone, never on the ranking (D-49).
pub fn hints_for(e: &Explanation, stage: Stage, focus: Option<Concept>) -> Hints {
    let all = taught(&catalogue(), &e.evidence, None);
    let shown = pick(&all, focus);
    let situation = (stage >= Stage::Situation).then(|| SituationHint {
        situation: match e.verdict {
            Verdict::Forced => Situation::Forced,
            _ if !e.complete() => Situation::Unclear,
            Verdict::Clear => Situation::OneStandsOut,
            Verdict::Close { equivalent } => Situation::SeveralEqual {
                count: equivalent.saturating_add(1),
            },
            Verdict::Unknown => Situation::Unclear,
        },
        important: e.important,
        options: e.actions.clone(),
    });
    let figures = (stage >= Stage::Figures).then(|| {
        e.actions
            .iter()
            .map(|&action| OptionFigures {
                action,
                estimate: e.option(action).and_then(|o| o.estimate),
            })
            .collect()
    });
    Hints {
        stage,
        decision: e.decision,
        role: e.role,
        objective: e.objective,
        status: e.status,
        considerations: e.considerations.clone(),
        situation,
        figures,
        basis: e.basis,
        assumptions: e.assumptions.clone(),
        evidence: e.evidence.clone(),
        teaching: shown.map_or(Teaching::Fallback, |t| t.teaching.clone()),
        lesson: shown.map(|t| t.content.clone()),
        concept: shown.map(|t| t.concept),
        compare_question: stage >= Stage::Situation,
    }
}

/// The short note on an own move right after it (D-49).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Note {
    /// The best option was chosen.
    Good,
    /// An option about as good as the best.
    AboutEqual,
    /// Somewhat weaker than the best ([`Judgement::Worse`]).
    SomewhatWeaker,
    /// Clearly weaker than the best ([`Judgement::Weak`]).
    ClearlyWeaker,
    /// There was no choice.
    Forced,
    /// No reliable judgement (analysis incomplete, option not compared).
    Uncertain,
}

/// One own decision judged after the move.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveReview {
    /// The kind of decision.
    pub decision: Decision,
    /// The seat's role.
    pub role: Role,
    /// Status of the analysis.
    pub status: Status,
    /// The bid value reached when deciding.
    pub bid: u16,
    /// Hand game, once decided.
    pub hand_game: Option<bool>,
    /// The trick, from 1, during card play.
    pub trick: Option<u8>,
    /// Whether the decision was important (D-48).
    pub important: bool,
    /// Whether there was only one legal option.
    pub forced: bool,
    /// What the seat knew and could infer then.
    pub considerations: Vec<Consideration>,
    /// The chosen option.
    pub chosen: ComparedOption,
    /// The best option, if one was evaluated.
    pub best: Option<ComparedOption>,
    /// The costliest natural weak option, which a good decision avoided.
    pub trap: Option<ComparedOption>,
    /// What the estimates rest on.
    pub basis: Basis,
    /// Model assumptions behind the estimates.
    pub assumptions: Vec<Assumption>,
    /// The permitted evidence of the decision.
    pub evidence: Evidence,
    /// The chosen option compared with the best.
    pub comparison: Comparison,
    /// The first lesson that applies, or the fallback to the model
    /// comparison.
    pub teaching: Teaching,
    /// What that lesson says.
    pub lesson: Option<LessonContent>,
    /// Every lesson that applies, in catalogue order (the first is
    /// `teaching`); a focus picks among them ([`MoveReview::lesson_for`]).
    pub taught: Vec<Taught>,
}

impl MoveReview {
    /// The lesson to show with `focus` ([`pick`]); `None` without a
    /// supported lesson.
    pub fn lesson_for(&self, focus: Option<Concept>) -> Option<&Taught> {
        pick(&self.taught, focus)
    }

    /// The short note on the move.
    pub fn note(&self) -> Note {
        if self.forced {
            return Note::Forced;
        }
        if self.status != Status::Complete {
            return Note::Uncertain;
        }
        match self.chosen.judgement {
            Judgement::Best => Note::Good,
            Judgement::Equivalent => Note::AboutEqual,
            Judgement::Worse => Note::SomewhatWeaker,
            Judgement::Weak => Note::ClearlyWeaker,
            Judgement::Screened | Judgement::NotEvaluated => Note::Uncertain,
        }
    }
}

/// Judges `chosen` in the decision `e` explains. `None` if `chosen` is not
/// one of its options.
pub fn review(e: &Explanation, chosen: Action) -> Option<MoveReview> {
    let comparison = e.comparisons.iter().find(|c| c.chosen == chosen)?.clone();
    let taught = taught(&catalogue(), &e.evidence, Some(&comparison));
    let teaching = taught
        .first()
        .map_or(Teaching::Fallback, |t| t.teaching.clone());
    let chosen = e.option(chosen)?.clone();
    let trap = e
        .options
        .iter()
        .filter(|o| o.natural && o.judgement == Judgement::Weak)
        .max_by(|a, b| a.gap.total_cmp(&b.gap))
        .cloned();
    Some(MoveReview {
        decision: e.decision,
        role: e.role,
        status: e.status,
        bid: e.bid,
        hand_game: e.hand_game,
        trick: e.trick,
        important: e.important,
        forced: e.verdict == Verdict::Forced,
        considerations: e.considerations.clone(),
        chosen,
        best: e.best().cloned(),
        trap,
        basis: e.basis,
        assumptions: e.assumptions.clone(),
        evidence: e.evidence.clone(),
        lesson: taught.first().map(|t| t.content.clone()),
        comparison,
        teaching,
        taught,
    })
}

/// How the model rated the choice at a moment of the debriefing. It
/// orders moments with the same lesson weight; it is never a quota.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MomentKind {
    /// An important decision decided well: a costly natural option avoided
    /// (complete analysis).
    Good,
    /// A clearly or somewhat weaker option chosen, losing at least
    /// [`MOMENT_POINTS`] (complete analysis).
    Bad,
    /// Neither: about equal, a small difference, or a provisional analysis.
    /// The lesson is the point of the moment.
    Neutral,
}

/// One primary moment of the end-of-game debriefing.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Moment {
    /// The model's reading of the choice.
    pub kind: MomentKind,
    /// The concept the moment teaches; no other moment has it.
    pub concept: Concept,
    /// `concept` is the player's focus.
    pub focused: bool,
    /// What the lesson says.
    pub lesson: LessonContent,
    /// The decision.
    pub review: MoveReview,
    /// Position of the decision in the `reviews` passed to [`moments`], so
    /// a caller can attach what it kept about that decision (issue #47).
    pub index: usize,
}

/// The primary moments of a game's own decisions (`reviews` in game
/// order), zero to [`PRIMARY_MOMENTS`], shown in game order.
///
/// Eligible is a decision with a choice and a supported lesson
/// ([`MoveReview::lesson_for`] with `focus`); a score gap or an avoided
/// trap alone never is. Fact lessons are exact, so a provisional analysis
/// stays eligible, as a [`MomentKind::Neutral`] moment. Eligible moments
/// are taken in this order, at most one per concept:
///
/// 1. the lesson of `focus`;
/// 2. a complete analysis rating the choice clearly ([`MomentKind::Good`]
///    or [`MomentKind::Bad`]);
/// 3. a mechanism lesson (exact, after the move) before a fact lesson;
/// 4. the larger loss or avoided trap;
/// 5. game order.
///
/// No minimum of good or bad moments; a game without an eligible decision
/// has none.
pub fn moments(reviews: &[MoveReview], focus: Option<Concept>) -> Vec<Moment> {
    let mut eligible: Vec<(usize, MomentKind, f64, &Taught)> = reviews
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.forced)
        .filter_map(|(i, r)| {
            let taught = r.lesson_for(focus)?;
            let (kind, signal) = signal(r);
            Some((i, kind, signal, taught))
        })
        .collect();
    eligible.sort_by(|a, b| {
        let focused = |t: &Taught| Some(t.concept) == focus;
        focused(b.3)
            .cmp(&focused(a.3))
            .then((b.1 != MomentKind::Neutral).cmp(&(a.1 != MomentKind::Neutral)))
            .then(b.3.concept.mechanism().cmp(&a.3.concept.mechanism()))
            .then(b.2.total_cmp(&a.2))
            .then(a.0.cmp(&b.0))
    });
    let mut chosen: Vec<(usize, MomentKind, &Taught)> = Vec::new();
    for (i, kind, _, taught) in eligible {
        if chosen.len() == PRIMARY_MOMENTS {
            break;
        }
        if chosen.iter().all(|c| c.2.concept != taught.concept) {
            chosen.push((i, kind, taught));
        }
    }
    chosen.sort_by_key(|c| c.0);
    chosen
        .into_iter()
        .map(|(i, kind, taught)| Moment {
            kind,
            concept: taught.concept,
            focused: Some(taught.concept) == focus,
            lesson: taught.content.clone(),
            review: reviews[i].clone(),
            index: i,
        })
        .collect()
}

/// The model's reading of a review and its weight: the loss of a weaker
/// choice or the loss of the trap avoided; only complete analyses count.
fn signal(r: &MoveReview) -> (MomentKind, f64) {
    if r.status != Status::Complete {
        return (MomentKind::Neutral, 0.0);
    }
    let gap = r.chosen.gap;
    match r.chosen.judgement {
        Judgement::Worse | Judgement::Weak if gap >= MOMENT_POINTS => (MomentKind::Bad, gap),
        Judgement::Best | Judgement::Equivalent if r.important => match &r.trap {
            Some(t) => (MomentKind::Good, t.gap),
            None => (MomentKind::Neutral, 0.0),
        },
        _ => (MomentKind::Neutral, 0.0),
    }
}

impl Explanation {
    /// `true` if the analysis finished; otherwise the explanation is
    /// provisional at best.
    pub fn complete(&self) -> bool {
        self.status == Status::Complete
    }

    /// The best option, if any was evaluated.
    pub fn best(&self) -> Option<&ComparedOption> {
        self.options
            .first()
            .filter(|o| o.judgement == Judgement::Best)
    }

    /// The comparison of `action`.
    pub fn option(&self, action: Action) -> Option<&ComparedOption> {
        self.options.iter().find(|o| o.action == action)
    }

    /// Pedagogical eligibility before the move: a supported (fact) lesson
    /// applies, with a `focus` that lesson. Separate from
    /// [`Explanation::important`] and from warnings, which stay engine
    /// judgements; automatic teaching needs this.
    pub fn teachable(&self, focus: Option<Concept>) -> bool {
        let all = taught(&catalogue(), &self.evidence, None);
        match focus {
            Some(c) => all.iter().any(|t| t.concept == c),
            None => !all.is_empty(),
        }
    }

    /// The warning before `action`: only for a complete analysis in which
    /// `action` is [`Judgement::Weak`].
    pub fn warning(&self, action: Action) -> Option<Warning> {
        if !self.complete() {
            return None;
        }
        let chosen = self.option(action)?;
        (chosen.judgement == Judgement::Weak).then(|| Warning {
            chosen: chosen.clone(),
        })
    }
}

/// Explains the decision of `view.seat` from `analysis`, which must be the
/// analysis of `view`. `None` if it is not this seat's turn or the analysis
/// belongs to another decision.
pub fn explain(view: &PlayerView, analysis: &Analysis) -> Option<Explanation> {
    if view.legal_actions.is_empty()
        || analysis.status == Status::NoDecision
        || analysis.seat != view.seat
        || analysis.phase != view.phase
        || analysis
            .options
            .iter()
            .map(|o| o.action)
            .ne(view.legal_actions.iter().copied())
    {
        return None;
    }
    let decision = decision(view);
    let role = Role::of(view);
    let options = compare(view, analysis, role);
    let verdict = verdict(&options);
    let important = analysis.status == Status::Complete
        && options
            .iter()
            .filter(|o| {
                matches!(
                    o.judgement,
                    Judgement::Best | Judgement::Equivalent | Judgement::Worse | Judgement::Weak
                )
            })
            .count()
            >= 2
        && options
            .iter()
            .any(|o| o.judgement == Judgement::Weak && natural(view, o.action));
    Some(Explanation {
        seat: view.seat,
        decision,
        role,
        objective: view.contract.map_or(Objective::Undecided, objective),
        status: analysis.status,
        actions: view.legal_actions.clone(),
        bid: view.bid,
        hand_game: view.hand_game,
        trick: (view.phase == Phase::Spiel).then(|| view.tricks.len() as u8 + 1),
        considerations: considerations(view, analysis, decision, role),
        options,
        verdict,
        important,
        basis: Basis {
            finished: analysis.effort.finished,
            planned: analysis.effort.planned,
            enumerated: analysis.effort.enumerated,
            effective_samples: analysis.effort.effective_samples,
        },
        assumptions: analysis.assumptions.clone(),
        evidence: gather(view, analysis),
        comparisons: view
            .legal_actions
            .iter()
            .filter_map(|&a| compare_options(view, analysis, a))
            .collect(),
    })
}

/// Analyses `view` with `skat-analysis` and explains it: the one call a
/// live coach needs. `None` if it is not this seat's turn or the view is
/// inconsistent.
pub fn advise(
    view: &PlayerView,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
) -> Option<Explanation> {
    let analysis = analyze(view, budget, seed, cancel).ok()?;
    explain(view, &analysis)
}

fn decision(view: &PlayerView) -> Decision {
    let has = |a: Action| view.legal_actions.contains(&a);
    match view.phase {
        Phase::Reizen if has(Action::Hold) => Decision::Answer,
        Phase::Reizen => Decision::Bid,
        Phase::SkatAufnahme => Decision::Pickup,
        Phase::Druecken => Decision::Discard,
        Phase::Ansage => Decision::Announce,
        _ if has(Action::Re) => Decision::Re,
        _ if has(Action::KeinKontra) => Decision::Kontra,
        _ => Decision::Card,
    }
}

fn objective(contract: Contract) -> Objective {
    match contract.game {
        GameType::Null => Objective::NoTrick,
        _ if contract.schwarz => Objective::AllTricks,
        _ if contract.schneider => Objective::Augen { target: 90 },
        _ => Objective::Augen { target: 61 },
    }
}

/// Options a player would ordinarily weigh: every option except jump bids
/// above the next bid value and Schneider or Schwarz announcements. Only
/// these make a decision important (D-48); warnings cover every option.
pub fn natural(view: &PlayerView, action: Action) -> bool {
    match action {
        Action::Bid(value) => {
            view.legal_actions
                .iter()
                .filter_map(|a| match a {
                    Action::Bid(v) => Some(*v),
                    _ => None,
                })
                .min()
                == Some(value)
        }
        Action::Announce(a) => !a.schneider && !a.schwarz,
        _ => true,
    }
}

fn considerations(
    view: &PlayerView,
    analysis: &Analysis,
    decision: Decision,
    role: Role,
) -> Vec<Consideration> {
    let playing = view.phase == Phase::Spiel;
    let mut out: Vec<Consideration> = analysis
        .observations
        .iter()
        .filter(|o| match o {
            Observation::Void { .. } | Observation::TrumpsOut { .. } => playing,
            // Shown by the remaining-Trumpf lesson only.
            Observation::TrumpfSets(_) => false,
            Observation::AugenWon {
                declarer,
                defenders,
            } => playing && declarer + defenders > 0,
            Observation::AugenOut { .. } => playing,
            Observation::SkatKnown { .. } => true,
            Observation::BidShown { seat, .. } => *seat != view.seat,
            Observation::KontraEligible { .. } => {
                role == Role::Declarer
                    || decision == Decision::Kontra
                    || view.legal_actions.contains(&Action::Kontra)
            }
            // Shown by the hand-structure and game-value lessons only.
            Observation::Structure(_)
            | Observation::GameValues(_)
            | Observation::Commitment { .. } => false,
        })
        .copied()
        .map(Consideration::Known)
        .collect();
    out.extend(analysis.inferences.iter().map(|&inference| {
        Consideration::Inferred(Inferred {
            inference,
            amount: Amount::of(inference.expected),
        })
    }));
    out
}

/// Ranks the options: fully evaluated ones by value, then screened ones,
/// then unevaluated ones; ties keep the order of the legal actions.
fn compare(view: &PlayerView, analysis: &Analysis, role: Role) -> Vec<ComparedOption> {
    let most = analysis
        .options
        .iter()
        .filter_map(|o| o.estimate.map(|e| e.samples))
        .max()
        .unwrap_or(0);
    let tier = |e: &Option<Estimate>| match e {
        Some(e) if e.samples == most => 0,
        Some(_) => 1,
        None => 2,
    };
    let mut ranked: Vec<(usize, u8, f64)> = analysis
        .options
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let value = o.estimate.map_or(f64::NEG_INFINITY, |e| utility(view, &e));
            (i, tier(&o.estimate), value)
        })
        .collect();
    ranked.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.total_cmp(&a.2)).then(a.0.cmp(&b.0)));
    let best = ranked
        .first()
        .filter(|r| r.1 == 0)
        .map(|r| analysis.options[r.0]);
    let runner_up = ranked
        .get(1)
        .filter(|r| r.1 == 0)
        .map(|r| analysis.options[r.0]);
    ranked
        .iter()
        .enumerate()
        .map(|(rank, &(i, tier, value))| {
            let option = analysis.options[i];
            let Some(estimate) = option.estimate else {
                return ComparedOption {
                    action: option.action,
                    natural: natural(view, option.action),
                    judgement: Judgement::NotEvaluated,
                    estimate: None,
                    value: None,
                    gap: 0.0,
                    margin: 0.0,
                    compared_with: None,
                    reasons: Vec::new(),
                };
            };
            let reference = if rank == 0 { runner_up } else { best };
            let reference = reference.and_then(|r| r.estimate.map(|e| (r.action, e)));
            let best_value = best
                .and_then(|b| b.estimate)
                .map_or(value, |e| utility(view, &e));
            let gap = (best_value - value).max(0.0);
            let margin = best.and_then(|b| b.estimate).map_or(0.0, |b| {
                MARGIN_SE * (b.score_std_error.powi(2) + estimate.score_std_error.powi(2)).sqrt()
            });
            let judgement = if tier == 1 {
                Judgement::Screened
            } else if rank == 0 {
                Judgement::Best
            } else if gap <= TIE_POINTS || gap <= margin {
                Judgement::Equivalent
            } else if gap >= WEAK_POINTS {
                Judgement::Weak
            } else {
                Judgement::Worse
            };
            let reasons = match reference {
                Some((_, other)) if tier == 0 => reasons(view, role, &estimate, &other),
                _ => Vec::new(),
            };
            ComparedOption {
                action: option.action,
                natural: natural(view, option.action),
                judgement,
                estimate: Some(estimate),
                value: Some(value),
                gap,
                margin,
                compared_with: reference.filter(|_| tier == 0).map(|(a, _)| a),
                reasons,
            }
        })
        .collect()
}

/// Differences between two estimates worth naming.
fn reasons(view: &PlayerView, role: Role, this: &Estimate, other: &Estimate) -> Vec<Reason> {
    let mut out = Vec::new();
    if (this.win_chance - other.win_chance).abs() >= CHANCE_STEP {
        out.push(Reason::WinChance {
            this: this.win_chance,
            other: other.win_chance,
        });
    }
    if (this.thrown_in - other.thrown_in).abs() >= CHANCE_STEP {
        out.push(Reason::ThrownIn {
            this: this.thrown_in,
            other: other.thrown_in,
        });
    }
    if role == Role::Defender
        && (this.expected_game_points - other.expected_game_points).abs() > TIE_POINTS
    {
        out.push(Reason::DeclarerPoints {
            this: this.expected_game_points,
            other: other.expected_game_points,
        });
    }
    let (a, b) = (utility(view, this), utility(view, other));
    let margin = MARGIN_SE * (this.score_std_error.powi(2) + other.score_std_error.powi(2)).sqrt();
    if (a - b).abs() <= TIE_POINTS {
        out.push(Reason::SameValue);
    } else {
        out.push(Reason::Points { this: a, other: b });
        if (a - b).abs() <= margin {
            out.push(Reason::WithinUncertainty);
        }
    }
    out
}

fn verdict(options: &[ComparedOption]) -> Verdict {
    let compared: Vec<&ComparedOption> = options
        .iter()
        .filter(|o| !matches!(o.judgement, Judgement::Screened | Judgement::NotEvaluated))
        .collect();
    if options.len() == 1 {
        return Verdict::Forced;
    }
    if compared.is_empty() {
        return Verdict::Unknown;
    }
    let equivalent = compared
        .iter()
        .filter(|o| o.judgement == Judgement::Equivalent)
        .count();
    if equivalent == 0 {
        Verdict::Clear
    } else {
        Verdict::Close {
            equivalent: equivalent.min(usize::from(u8::MAX)) as u8,
        }
    }
}

/// Sampling seed of the coach's analysis of `view`: `base` (the coach's own
/// seed, never a deal seed) mixed with public facts of the view, so the
/// same view and base give the same advice.
pub fn decision_seed(base: u64, view: &PlayerView) -> u64 {
    let kontra = match view.kontra {
        KontraLevel::None => 0,
        KontraLevel::Kontra => 1,
        KontraLevel::Re => 2,
    };
    let facts = [
        view.seat.index() as u64,
        view.phase as u64,
        u64::from(view.hand.bits()),
        view.bidding.len() as u64,
        view.tricks.len() as u64,
        view.current_trick.len() as u64,
        kontra,
        view.legal_actions.len() as u64,
    ];
    facts.iter().fold(base, |h, &f| mix(h, f))
}

/// SplitMix64 finaliser.
fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
