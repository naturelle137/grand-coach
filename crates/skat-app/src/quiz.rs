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
    derive,
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
}

impl QuizState {
    pub(crate) fn new(seed: u64) -> QuizState {
        QuizState {
            seed,
            settings: AssistSettings::default(),
            pending: None,
            asked: Vec::new(),
            feedback: None,
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
        self.settings = settings;
        if !settings.quiz {
            self.pending = None;
        }
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
        let kinds = applicable_questions(&snapshot);
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
        self.pending = Some(Pending { kind, snapshot });
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
        self.feedback = Some(feedback);
        Some(feedback)
    }

    pub(crate) fn view(&self, view: &PlayerView, dealer: PlayerId) -> AssistView {
        let counting = if !self.settings.counting_aid {
            CountingState::Off
        } else if self.pending.is_some() {
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
