// SPDX-License-Identifier: EUPL-1.2

//! The optional counting aid and in-game quiz (M6, D-64): settings, the
//! question state of a session and the view models. All facts come from
//! `skat_analysis::counting`, derived from the human's permitted view.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use skat_analysis::Knowledge;
use skat_analysis::counting::{
    AugenCount, CountingAid, Derivation, QuestionKind, TrumpfCount, Voids, applicable_questions,
    derive, retrieval_questions,
};
use skat_core::{Card, GameType, PlayerView, Seat};

use crate::coach::DecisionKey;
use crate::session::PlayerId;
use crate::sort::sort_hand;

/// Completed tricks after which the human's next card-play decision
/// carries a question: after the third and the sixth trick.
pub const QUESTION_AFTER_TRICKS: [u8; 2] = [3, 6];

/// Assistance settings. Both are off by default and independent of each
/// other and of the coach. Changing them never changes the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssistSettings {
    /// Show the counting aid.
    pub counting_aid: bool,
    /// Ask counting questions at the human's card-play decisions.
    pub quiz: bool,
    /// Restrict questions to remaining Trumpf and publicly proven voids (#48).
    #[serde(default)]
    pub retrieval: bool,
    /// Explicit support for focused retrieval; never inferred from answers.
    #[serde(default)]
    pub assistance: AssistanceLevel,
}

/// Support chosen by the player for focused retrieval.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssistanceLevel {
    /// Show the exact aid alongside the question.
    VisibleAid,
    /// Recall mentally, then request the exact aid or submit an answer.
    RevealOnRequest,
    /// Enter an answer before feedback; help can always be restored explicitly.
    #[default]
    AnswerBeforeReveal,
}

/// Stable opportunity identity within a seeded series. Not a mastery ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuestionId {
    /// Independent quiz stream seed, not the deal seed.
    pub series: u64,
    /// Game number (one-based).
    pub game: u32,
    /// Completed tricks at the opportunity.
    pub trick: u8,
}

/// Once-only transition. Deduplication key: question ID and variant tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrievalAction {
    /// The frozen question was offered.
    Opportunity,
    /// Exact help was explicitly restored (including a settings change).
    Help,
    /// A numeric response, scored against the frozen snapshot.
    Answer {
        /// Submitted count.
        given: u8,
        /// Exact match.
        correct: bool,
    },
    /// No answer; disabling the quiz is also an explicit skip.
    Skip {
        /// Quiz was disabled rather than the skip button used.
        disabled: bool,
    },
}

/// Plain in-memory event shared with future progress consumers. No storage
/// dependency; no inference about prior exposure, retention or mastery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalEvent {
    /// Stable opportunity identifier.
    pub id: QuestionId,
    /// Frozen question (never contains its answer before reveal).
    pub question: QuestionKind,
    /// Once-only action within this opportunity.
    pub action: RetrievalAction,
    /// Explicitly selected support at this transition.
    pub assistance: AssistanceLevel,
    /// Exact help was available during this opportunity; sticky even if hidden again.
    pub assisted: bool,
}

/// How a question ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuizOutcome {
    /// The answer equals the exact one.
    Correct,
    /// The answer differs from the exact one.
    Incorrect {
        /// What the player answered.
        given: u8,
    },
    /// The player skipped the question.
    Skipped,
}

/// The result of a question, shown after it was answered or skipped. The
/// derivation holds the numbers behind the exact answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizFeedback {
    /// The question.
    pub kind: QuestionKind,
    /// How it ended.
    pub outcome: QuizOutcome,
    /// The exact answer and how it follows from what the player knew.
    pub derivation: Derivation,
}

/// What the aid shows. Exact facts of the human's permitted view only; see
/// `skat_analysis::counting` for the definitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountingView {
    /// The announced game, if any.
    pub game: Option<GameType>,
    /// Cards played so far including the trick in progress, in display
    /// order.
    pub played: Vec<Card>,
    /// Trumpf counts; `None` before the announcement and in Null.
    pub trumpf: Option<TrumpfCount>,
    /// Augen counts.
    pub augen: AugenCount,
    /// Proven voids of each player (index: [`PlayerId`]); the human's
    /// entry is empty.
    pub voids: [Voids; 3],
}

/// State of the counting aid in the table view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountingState {
    /// Switched off.
    Off,
    /// Switched on but hidden while a question is pending, so that it
    /// cannot show the answer.
    Withheld,
    /// Switched on.
    Shown(Box<CountingView>),
}

/// The quiz part of the table view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizView {
    /// The pending question. The decision is held until it is answered,
    /// skipped or the quiz is switched off.
    pub question: Option<QuestionKind>,
    /// ID of the pending opportunity.
    pub id: Option<QuestionId>,
    /// Exact frozen help, only after an explicit reveal or at VisibleAid.
    pub help: Option<Derivation>,
    /// The result of the last question of this decision.
    pub feedback: Option<QuizFeedback>,
}

/// The assistance part of the table view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistView {
    /// Current settings.
    pub settings: AssistSettings,
    /// The counting aid.
    pub counting: CountingState,
    /// The quiz.
    pub quiz: QuizView,
}

/// A question as it was asked: its kind and the knowledge it may be
/// answered from. The expected answer is derived from the snapshot only.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pending {
    kind: QuestionKind,
    snapshot: Knowledge,
    id: QuestionId,
    assisted: bool,
    revealed: bool,
}

/// Quiz state of a session.
#[derive(Clone, Debug)]
pub(crate) struct QuizState {
    /// Own stream of the series seed: reveals nothing about the deals.
    seed: u64,
    pub(crate) settings: AssistSettings,
    pending: Option<Pending>,
    /// Trick counts at which a question was already asked in this game.
    asked: Vec<u8>,
    feedback: Option<QuizFeedback>,
    events: Vec<RetrievalEvent>,
}

impl QuizState {
    pub(crate) fn new(seed: u64) -> QuizState {
        QuizState {
            seed,
            settings: AssistSettings::default(),
            pending: None,
            asked: Vec::new(),
            feedback: None,
            events: Vec::new(),
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// A new game: nothing carries over.
    pub(crate) fn game_changed(&mut self) {
        self.pending = None;
        self.asked.clear();
        self.feedback = None;
    }

    /// The decision changed: feedback of the last one goes away.
    pub(crate) fn decision_changed(&mut self) {
        self.feedback = None;
    }

    pub(crate) fn set(&mut self, settings: AssistSettings) {
        let changed_level = self.settings.assistance != settings.assistance;
        self.settings = settings;
        if changed_level
            && settings.assistance != AssistanceLevel::VisibleAid
            && let Some(p) = self.pending.as_mut()
        {
            p.revealed = false;
        }
        if !settings.quiz {
            if let Some(p) = self.pending.take() {
                self.record(p, RetrievalAction::Skip { disabled: true });
            }
        } else if settings.retrieval && settings.assistance == AssistanceLevel::VisibleAid {
            self.restore_help();
        }
    }

    fn record(&mut self, p: Pending, action: RetrievalAction) {
        self.events.push(RetrievalEvent {
            id: p.id,
            question: p.kind,
            action,
            assistance: self.settings.assistance,
            assisted: p.assisted,
        });
    }

    pub(crate) fn take_events(&mut self) -> Vec<RetrievalEvent> {
        std::mem::take(&mut self.events)
    }

    /// Restoring help does not release the decision hold. Repeated requests
    /// are idempotent and never create duplicate help events.
    pub(crate) fn restore_help(&mut self) -> bool {
        let Some(mut p) = self.pending else {
            return false;
        };
        if !p.revealed {
            p.revealed = true;
            let first_help = !p.assisted;
            p.assisted = true;
            self.pending = Some(p);
            if first_help {
                self.record(p, RetrievalAction::Help);
            }
        }
        true
    }

    /// Suppress all auxiliary answer sources, including late coach results
    /// and historical review panels. Explicit restoration only exposes facts.
    pub(crate) fn withhold_coach(&self) -> bool {
        self.is_pending()
    }

    /// Asks the question due at the human's decision `key`, if any: with
    /// the quiz on, in card play, after a trick count of
    /// [`QUESTION_AFTER_TRICKS`] not yet asked about in this game. The
    /// kind is chosen from the applicable ones with a seed of the series,
    /// the game number and the trick count.
    pub(crate) fn refresh(&mut self, key: Option<DecisionKey>, view: &PlayerView) {
        let Some(key) = key else {
            return;
        };
        let tricks = view.tricks.len() as u8;
        if !self.settings.quiz
            || self.pending.is_some()
            || view.phase != skat_core::Phase::Spiel
            || !QUESTION_AFTER_TRICKS.contains(&tricks)
            || self.asked.contains(&tricks)
        {
            return;
        }
        let snapshot = Knowledge::from_view(view);
        let kinds = if self.settings.retrieval {
            retrieval_questions(&snapshot)
        } else {
            applicable_questions(&snapshot)
        };
        if kinds.is_empty() {
            return;
        }
        let mut rng = ChaCha8Rng::seed_from_u64(
            self.seed
                ^ u64::from(key.number).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ u64::from(tricks).wrapping_mul(0xBF58_476D_1CE4_E5B9),
        );
        let kind = kinds[rng.next_u32() as usize % kinds.len()];
        self.asked.push(tricks);
        let assisted =
            self.settings.retrieval && self.settings.assistance == AssistanceLevel::VisibleAid;
        let p = Pending {
            kind,
            snapshot,
            id: QuestionId {
                series: self.seed,
                game: key.number,
                trick: tricks,
            },
            assisted,
            revealed: assisted,
        };
        self.pending = Some(p);
        self.record(p, RetrievalAction::Opportunity);
    }

    /// Settles the pending question with `answer` (`None`: skipped).
    pub(crate) fn settle(&mut self, answer: Option<u8>) -> Option<QuizFeedback> {
        let pending = self.pending.take()?;
        let derivation = derive(&pending.snapshot, pending.kind).expect("asked only if answerable");
        let outcome = match answer {
            None => QuizOutcome::Skipped,
            Some(given) if given == derivation.expected() => QuizOutcome::Correct,
            Some(given) => QuizOutcome::Incorrect { given },
        };
        let feedback = QuizFeedback {
            kind: pending.kind,
            outcome,
            derivation,
        };
        self.record(
            pending,
            match answer {
                Some(given) => RetrievalAction::Answer {
                    given,
                    correct: given == derivation.expected(),
                },
                None => RetrievalAction::Skip { disabled: false },
            },
        );
        self.feedback = Some(feedback);
        Some(feedback)
    }

    pub(crate) fn view(&self, view: &PlayerView, dealer: PlayerId) -> AssistView {
        let revealed = self.pending.is_some_and(|p| p.revealed);
        let counting = if revealed {
            let p = self.pending.expect("revealed pending question");
            CountingState::Shown(Box::new(counting_view(
                &CountingAid::from_knowledge(&p.snapshot),
                dealer,
            )))
        } else if !self.settings.counting_aid {
            CountingState::Off
        } else if self.withhold_coach() {
            CountingState::Withheld
        } else {
            let aid = CountingAid::from_knowledge(&Knowledge::from_view(view));
            CountingState::Shown(Box::new(counting_view(&aid, dealer)))
        };
        AssistView {
            settings: self.settings,
            counting,
            quiz: QuizView {
                question: self.pending.map(|p| p.kind),
                id: self.pending.map(|p| p.id),
                help: self
                    .pending
                    .filter(|p| p.revealed)
                    .and_then(|p| derive(&p.snapshot, p.kind)),
                feedback: self.feedback,
            },
        }
    }
}

fn counting_view(aid: &CountingAid, dealer: PlayerId) -> CountingView {
    let mut voids = [Voids::default(); 3];
    for seat in Seat::ALL {
        voids[PlayerId::on_seat(seat, dealer).index()] = aid.voids[seat.index()];
    }
    CountingView {
        game: aid.game,
        played: sort_hand(aid.played, aid.game),
        trumpf: aid.trumpf,
        augen: aid.augen,
        voids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{Action, Announcement, Game, Phase, RuleConfig};

    #[test]
    fn help_and_delayed_grading_keep_the_original_snapshot_even_with_new_input() {
        let mut g = Game::new(1, RuleConfig::default());
        while g.phase() != Phase::Spiel {
            let a = match g.phase() {
                Phase::Reizen if g.bid() == 0 => Action::Bid(18),
                Phase::Reizen => Action::Pass,
                Phase::SkatAufnahme => Action::Hand,
                Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
                _ => g.legal_actions()[0],
            };
            g.apply(a).unwrap();
        }
        while g.view(Seat::Vorhand).tricks.len() < 3 {
            g.apply(g.legal_actions()[0]).unwrap();
        }
        let mut q = QuizState::new(4);
        q.set(AssistSettings {
            quiz: true,
            retrieval: true,
            counting_aid: true,
            ..AssistSettings::default()
        });
        let seat = g.current_seat().unwrap();
        q.refresh(
            Some(DecisionKey {
                lineage: 0,
                number: 1,
                actions: 20,
            }),
            &g.view(seat),
        );
        assert!(q.restore_help());
        let frozen = q.view(&g.view(seat), PlayerId::HUMAN);
        // Production Session holds this action. Injecting later *legal* core
        // input isolates the quiz's snapshot contract from that separate guard.
        for _ in 0..6 {
            g.apply(g.legal_actions()[0]).unwrap();
        }
        q.refresh(
            Some(DecisionKey {
                lineage: 0,
                number: 1,
                actions: 26,
            }),
            &g.view(seat),
        );
        let after = q.view(&g.view(seat), PlayerId::HUMAN);
        assert_eq!(after, frozen);
        let expected = frozen.quiz.help.unwrap().expected();
        assert_eq!(
            q.settle(Some(expected)).unwrap().outcome,
            QuizOutcome::Correct
        );
        assert_eq!(q.feedback.unwrap().derivation, frozen.quiz.help.unwrap());
    }
}
