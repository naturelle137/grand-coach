// SPDX-License-Identifier: EUPL-1.2

//! Focused counting training outside a game (M8 foundation, D-71).
//!
//! The learner chooses one of the two focused-retrieval topics (remaining
//! Trumpf, publicly proven voids) and gets a fixed, reproducible position:
//! a seeded deal from `skat-core` with a recorded legal action prefix,
//! seen from one seat. Only that seat's [`PlayerView`] is kept; the
//! question, the expected answer, help and feedback all come from its
//! [`Knowledge`] through `skat_analysis::counting`, exactly as the in-game
//! quiz does. The full deal is dropped right after the prefix is replayed.
//!
//! A training never touches a session, score sheet or recording. Each
//! presentation of a task is its own learning-history attempt
//! ([`AttemptKind::Training`]); its events carry the [`TrainingTask`]
//! (fixture and version) as their opportunity identity. Fixed fixtures and
//! retries are practice, never new transfer or mastery.

use serde::{Deserialize, Serialize};
use skat_analysis::Knowledge;
use skat_analysis::counting::{Derivation, QuestionKind, derive, retrieval_questions};
use skat_core::{
    Action, Announcement, Card, CardSet, Contract, Follow, Game, GameType, Phase, PlayerView, Rank,
    RuleConfig, Seat, Suit,
};

use crate::learning::AttemptKind;
use crate::quiz::{
    AssistanceLevel, OpportunityId, QuizFeedback, QuizOutcome, RetrievalAction, RetrievalEvent,
};
use crate::sort::sort_hand;

/// A training topic: one of the two focused-retrieval families (#48).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrainingTopic {
    /// Remaining Trumpf. Farbspiel and Grand only: Null has no Trumpf.
    TrumpfOutside,
    /// Publicly proven voids.
    ProvenVoids,
}

impl TrainingTopic {
    /// Both topics in display order.
    pub const ALL: [TrainingTopic; 2] = [TrainingTopic::TrumpfOutside, TrainingTopic::ProvenVoids];

    /// The fixtures of this topic in presentation order.
    pub fn fixtures(self) -> Vec<TrainingFixture> {
        TrainingFixture::ALL
            .into_iter()
            .filter(|f| f.topic() == self)
            .collect()
    }
}

/// A fixed training position. The variant name is its stable identity in
/// the learning history; a changed position gets a new [`Self::version`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrainingFixture {
    /// Declarer of a Karo game after three tricks, own Skat known.
    TrumpfDeclarerFarbspiel,
    /// Defender in a Grand during the third trick, Skat unknown.
    TrumpfDefenderGrand,
    /// Declarer of a Grand after two tricks; both defenders failed to
    /// follow Trumpf.
    VoidsDeclarerGrand,
    /// Defender in a Kreuz game right after the declarer failed to follow
    /// Herz; one card earlier nothing was proven (boundary).
    VoidsDefenderFarbspiel,
}

/// A training task as recorded in the learning history: fixture and
/// version, so later changes of a position never merge with older events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainingTask {
    /// The fixture.
    pub fixture: TrainingFixture,
    /// Its version.
    pub version: u32,
}

const fn c(suit: Suit, rank: Rank) -> Card {
    Card::new(suit, rank)
}

fn play(cards: &[Card]) -> impl Iterator<Item = Action> + '_ {
    cards.iter().map(|&card| Action::Play(card))
}

impl TrainingFixture {
    /// Every fixture.
    pub const ALL: [TrainingFixture; 4] = [
        TrainingFixture::TrumpfDeclarerFarbspiel,
        TrainingFixture::TrumpfDefenderGrand,
        TrainingFixture::VoidsDeclarerGrand,
        TrainingFixture::VoidsDefenderFarbspiel,
    ];

    /// The topic it trains.
    pub fn topic(self) -> TrainingTopic {
        match self {
            TrainingFixture::TrumpfDeclarerFarbspiel | TrainingFixture::TrumpfDefenderGrand => {
                TrainingTopic::TrumpfOutside
            }
            TrainingFixture::VoidsDeclarerGrand | TrainingFixture::VoidsDefenderFarbspiel => {
                TrainingTopic::ProvenVoids
            }
        }
    }

    /// Version of the position and question. Version 1 for all fixtures.
    pub fn version(self) -> u32 {
        1
    }

    /// The task identity recorded in the learning history.
    pub fn task(self) -> TrainingTask {
        TrainingTask {
            fixture: self,
            version: self.version(),
        }
    }

    /// Seed of the deal: [`Game::new`] with [`RuleConfig::default`].
    pub fn seed(self) -> u64 {
        match self {
            TrainingFixture::TrumpfDeclarerFarbspiel => 1,
            TrainingFixture::TrumpfDefenderGrand => 19,
            TrainingFixture::VoidsDeclarerGrand => 0,
            TrainingFixture::VoidsDefenderFarbspiel => 3,
        }
    }

    /// The seat whose view the task shows. It is that seat's turn.
    pub fn seat(self) -> Seat {
        match self {
            TrainingFixture::TrumpfDeclarerFarbspiel => Seat::Hinterhand,
            TrainingFixture::TrumpfDefenderGrand | TrainingFixture::VoidsDeclarerGrand => {
                Seat::Vorhand
            }
            TrainingFixture::VoidsDefenderFarbspiel => Seat::Hinterhand,
        }
    }

    /// The question asked.
    pub fn question(self) -> QuestionKind {
        match self {
            TrainingFixture::TrumpfDeclarerFarbspiel | TrainingFixture::TrumpfDefenderGrand => {
                QuestionKind::TrumpfOutside
            }
            TrainingFixture::VoidsDeclarerGrand => QuestionKind::ProvenVoids {
                class: Follow::Trumpf,
            },
            TrainingFixture::VoidsDefenderFarbspiel => QuestionKind::ProvenVoids {
                class: Follow::Farbe(Suit::Herz),
            },
        }
    }

    /// The recorded action prefix from the deal to the position: bidding,
    /// Skat, announcement and card play. Every action is validated by
    /// [`Game::apply`] when the position is built.
    pub fn actions(self) -> Vec<Action> {
        use Rank::*;
        use Suit::*;
        let announce = |game| Action::Announce(Announcement::plain(game));
        let discard = |a: Card, b: Card| Action::Discard(CardSet::from_iter([a, b]));
        match self {
            TrainingFixture::TrumpfDeclarerFarbspiel => [
                Action::Pass,
                Action::Bid(18),
                Action::Pass,
                Action::TakeSkat,
                discard(c(Herz, Acht), c(Herz, Neun)),
                announce(GameType::Farbe(Karo)),
            ]
            .into_iter()
            .chain(play(&[
                c(Herz, Sieben),
                c(Herz, Koenig),
                c(Herz, Zehn),
                c(Pik, Bube),
                c(Kreuz, Bube),
                c(Karo, Acht),
                c(Karo, Sieben),
                c(Karo, Dame),
                c(Karo, Koenig),
            ]))
            .collect(),
            TrainingFixture::TrumpfDefenderGrand => [
                Action::Pass,
                Action::Bid(18),
                Action::Pass,
                Action::TakeSkat,
                discard(c(Kreuz, Neun), c(Karo, Sieben)),
                announce(GameType::Grand),
            ]
            .into_iter()
            .chain(play(&[
                c(Kreuz, Dame),
                c(Kreuz, Zehn),
                c(Kreuz, Ass),
                c(Kreuz, Bube),
                c(Herz, Dame),
                c(Pik, Bube),
                c(Herz, Bube),
            ]))
            .collect(),
            TrainingFixture::VoidsDeclarerGrand => [
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::TakeSkat,
                discard(c(Kreuz, Acht), c(Pik, Acht)),
                announce(GameType::Grand),
            ]
            .into_iter()
            .chain(play(&[
                c(Kreuz, Bube),
                c(Herz, Koenig),
                c(Pik, Bube),
                c(Herz, Bube),
                c(Karo, Neun),
                c(Pik, Sieben),
            ]))
            .collect(),
            TrainingFixture::VoidsDefenderFarbspiel => [
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::TakeSkat,
                discard(c(Herz, Acht), c(Herz, Neun)),
                announce(GameType::Farbe(Kreuz)),
            ]
            .into_iter()
            .chain(play(&[
                c(Herz, Ass),
                c(Herz, Zehn),
                c(Herz, Koenig),
                c(Herz, Dame),
                c(Karo, Sieben),
            ]))
            .collect(),
        }
    }

    /// Replays the prefix through the rule engine and returns the seat's
    /// view; the full game is dropped. Fails if an action is illegal, the
    /// position is not the seat's card-play turn or the question is not a
    /// focused-retrieval question of that view.
    pub fn view(self) -> Result<PlayerView, TrainingError> {
        let mut game = Game::new(self.seed(), RuleConfig::default());
        for action in self.actions() {
            game.apply(action)
                .map_err(|_| TrainingError::InvalidFixture)?;
        }
        let view = game.view(self.seat());
        check_position(&view, self.question())?;
        Ok(view)
    }
}

/// A position is usable when it is the seat's card-play turn and the
/// question is one of its focused-retrieval questions.
fn check_position(view: &PlayerView, question: QuestionKind) -> Result<(), TrainingError> {
    let usable = view.phase == Phase::Spiel
        && view.current_seat == Some(view.seat)
        && retrieval_questions(&Knowledge::from_view(view)).contains(&question);
    if usable {
        Ok(())
    } else {
        Err(TrainingError::InvalidFixture)
    }
}

/// Why a training command was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingError {
    /// The task is already answered or skipped.
    Settled,
    /// The task is not answered or skipped yet.
    NotSettled,
    /// A fixture does not replay to a usable position (a defect).
    InvalidFixture,
}

/// What the learner can do in a training.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingCommand {
    /// Switch to the first task of another topic; an unsettled task is left
    /// without answer or skip.
    Topic(TrainingTopic),
    /// Choose the support for the current and following tasks.
    Assistance(AssistanceLevel),
    /// Show the exact help; the presentation then counts as assisted.
    Help,
    /// Answer the question.
    Answer(u8),
    /// Skip the question; its answer is shown.
    Skip,
    /// Present the same task again, after it was settled.
    Retry,
    /// Present the next task of the topic, after the current one was
    /// settled. With two tasks per topic, it cycles.
    Next,
}

/// A completed trick in display form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingTrick {
    /// Cards in play order with the seat that played them.
    pub cards: [(Seat, Card); 3],
    /// The seat that won it.
    pub winner: Seat,
}

/// The training as the frontend shows it. Everything comes from the
/// intended seat's view: no other hand and no unseen Skat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingView {
    /// The chosen topic.
    pub topic: TrainingTopic,
    /// The task and its version.
    pub task: TrainingTask,
    /// Position of the task in its topic (from 1) and the topic's size.
    pub position: (usize, usize),
    /// Presentation of this task within the entered training, from 1;
    /// higher numbers are retries or repeats.
    pub presentation: u32,
    /// The seat whose view this is.
    pub seat: Seat,
    /// The declarer.
    pub declarer: Seat,
    /// The game being played.
    pub contract: Contract,
    /// The winning bid.
    pub bid: u16,
    /// The seat's hand, sorted for the game.
    pub hand: Vec<Card>,
    /// The seat's own discarded Skat (declarer after picking it up).
    pub own_skat: Option<Vec<Card>>,
    /// Completed tricks in order.
    pub tricks: Vec<TrainingTrick>,
    /// The trick in progress, in play order.
    pub current_trick: Vec<(Seat, Card)>,
    /// The question.
    pub question: QuestionKind,
    /// The chosen support.
    pub assistance: AssistanceLevel,
    /// Exact help while unsettled, after a request or with visible aid.
    pub help: Option<Derivation>,
    /// Help was available during this presentation (sticky).
    pub assisted: bool,
    /// The result once answered or skipped.
    pub feedback: Option<QuizFeedback>,
}

/// The presented task.
#[derive(Clone, Debug)]
struct Presented {
    fixture: TrainingFixture,
    view: PlayerView,
    snapshot: Knowledge,
    presentation: u32,
    assisted: bool,
    revealed: bool,
    feedback: Option<QuizFeedback>,
}

/// An entered focused counting training. Plain state: no session, no
/// files; the frontend records [`Self::take_events`] in the learning
/// history under [`Self::attempt_kind`], one attempt per presentation.
#[derive(Clone, Debug)]
pub struct TrainingSession {
    topic: TrainingTopic,
    index: usize,
    assistance: AssistanceLevel,
    /// Presentations per fixture (index: position in [`TrainingFixture::ALL`]).
    shown: [u32; TrainingFixture::ALL.len()],
    current: Presented,
    events: Vec<RetrievalEvent>,
}

impl TrainingSession {
    /// Enters a training of `topic` with the first task presented.
    pub fn new(topic: TrainingTopic, assistance: AssistanceLevel) -> TrainingSession {
        let fixture = topic.fixtures()[0];
        let mut session = TrainingSession {
            topic,
            index: 0,
            assistance,
            shown: [0; TrainingFixture::ALL.len()],
            current: present(fixture, 0, assistance),
            events: Vec::new(),
        };
        session.present(fixture);
        session
    }

    fn present(&mut self, fixture: TrainingFixture) {
        let slot = TrainingFixture::ALL
            .iter()
            .position(|&f| f == fixture)
            .expect("fixture listed");
        self.shown[slot] += 1;
        self.current = present(fixture, self.shown[slot], self.assistance);
        self.record(RetrievalAction::Opportunity);
    }

    fn record(&mut self, action: RetrievalAction) {
        self.events.push(RetrievalEvent {
            id: OpportunityId::Training(self.current.fixture.task()),
            question: self.current.fixture.question(),
            action,
            assistance: self.assistance,
            assisted: self.current.assisted,
        });
    }

    fn reveal(&mut self) {
        let p = &mut self.current;
        if !p.revealed {
            p.revealed = true;
            let first = !p.assisted;
            p.assisted = true;
            if first {
                self.record(RetrievalAction::Help);
            }
        }
    }

    fn settled(&self) -> bool {
        self.current.feedback.is_some()
    }

    fn settle(&mut self, answer: Option<u8>) -> Result<(), TrainingError> {
        if self.settled() {
            return Err(TrainingError::Settled);
        }
        let p = &self.current;
        let derivation = derive(&p.snapshot, p.fixture.question()).expect("checked fixture");
        let expected = derivation.expected();
        let (outcome, action) = match answer {
            None => (
                QuizOutcome::Skipped,
                RetrievalAction::Skip { disabled: false },
            ),
            Some(given) => (
                if given == expected {
                    QuizOutcome::Correct
                } else {
                    QuizOutcome::Incorrect { given }
                },
                RetrievalAction::Answer {
                    given,
                    correct: given == expected,
                },
            ),
        };
        self.current.feedback = Some(QuizFeedback {
            kind: p.fixture.question(),
            outcome,
            derivation,
        });
        self.record(action);
        Ok(())
    }

    /// Executes `command`. A refused command changes nothing.
    pub fn execute(&mut self, command: TrainingCommand) -> Result<(), TrainingError> {
        match command {
            TrainingCommand::Topic(topic) => {
                self.topic = topic;
                self.index = 0;
                self.present(topic.fixtures()[0]);
            }
            TrainingCommand::Assistance(level) => {
                let changed = level != self.assistance;
                self.assistance = level;
                if !self.settled() {
                    if level == AssistanceLevel::VisibleAid {
                        self.reveal();
                    } else if changed {
                        self.current.revealed = false;
                    }
                }
            }
            TrainingCommand::Help => {
                if self.settled() {
                    return Err(TrainingError::Settled);
                }
                self.reveal();
            }
            TrainingCommand::Answer(given) => self.settle(Some(given))?,
            TrainingCommand::Skip => self.settle(None)?,
            TrainingCommand::Retry | TrainingCommand::Next => {
                if !self.settled() {
                    return Err(TrainingError::NotSettled);
                }
                if command == TrainingCommand::Next {
                    self.index = (self.index + 1) % self.topic.fixtures().len();
                }
                self.present(self.topic.fixtures()[self.index]);
            }
        }
        Ok(())
    }

    /// The events since the last call, all of the current presentation.
    pub fn take_events(&mut self) -> Vec<RetrievalEvent> {
        std::mem::take(&mut self.events)
    }

    /// The learning-history attempt kind of the current presentation.
    pub fn attempt_kind(&self) -> AttemptKind {
        AttemptKind::Training {
            attempt: self.current.presentation,
        }
    }

    /// The current task and its presentation number; a change means a new
    /// learning-history attempt.
    pub fn presentation(&self) -> (TrainingFixture, u32) {
        (self.current.fixture, self.current.presentation)
    }

    /// What the frontend shows.
    pub fn view(&self) -> TrainingView {
        let p = &self.current;
        let v = &p.view;
        let contract = v.contract.expect("checked fixture is in card play");
        let game = Some(contract.game);
        let fixtures = self.topic.fixtures();
        let current_trick = match v.current_trick.leader {
            Some(leader) => v
                .current_trick
                .cards
                .iter()
                .flatten()
                .enumerate()
                .map(|(i, &card)| (Seat::from_index(leader.index() + i), card))
                .collect(),
            None => Vec::new(),
        };
        TrainingView {
            topic: self.topic,
            task: p.fixture.task(),
            position: (self.index + 1, fixtures.len()),
            presentation: p.presentation,
            seat: v.seat,
            declarer: v.declarer.expect("checked fixture has a declarer"),
            contract,
            bid: v.bid,
            hand: sort_hand(v.hand, game),
            own_skat: v.own_skat.map(|s| sort_hand(s, game)),
            tricks: v
                .tricks
                .iter()
                .map(|t| TrainingTrick {
                    cards: [0, 1, 2].map(|i| (t.seat_of(i), t.cards[i])),
                    winner: t.winner,
                })
                .collect(),
            current_trick,
            question: p.fixture.question(),
            assistance: self.assistance,
            help: (p.revealed && p.feedback.is_none())
                .then(|| derive(&p.snapshot, p.fixture.question()))
                .flatten(),
            assisted: p.assisted,
            feedback: p.feedback,
        }
    }
}

/// A fresh presentation of `fixture` from its view only.
fn present(fixture: TrainingFixture, presentation: u32, assistance: AssistanceLevel) -> Presented {
    let view = fixture.view().expect("fixtures are validated by tests");
    from_view(fixture, view, presentation, assistance)
}

fn from_view(
    fixture: TrainingFixture,
    view: PlayerView,
    presentation: u32,
    assistance: AssistanceLevel,
) -> Presented {
    let visible = assistance == AssistanceLevel::VisibleAid;
    Presented {
        fixture,
        snapshot: Knowledge::from_view(&view),
        view,
        presentation,
        assisted: visible,
        revealed: visible,
        feedback: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::Deal;

    /// Swaps `a` and `b` wherever they lie in the original deal.
    fn swapped(deal: (&[CardSet; 3], CardSet), a: Card, b: Card) -> Deal {
        let swap = |set: CardSet| {
            let mut out = set;
            if set.contains(a) {
                out = (out - CardSet::from_iter([a])) | CardSet::from_iter([b]);
            }
            if set.contains(b) {
                out = (out - CardSet::from_iter([b])) | CardSet::from_iter([a]);
            }
            out
        };
        Deal::new(deal.0.map(swap), swap(deal.1)).unwrap()
    }

    /// Hidden-card substitution: two unseen, unplayed cards of one follow
    /// class change places between the two other hands. The prefix stays
    /// legal, the seat's view is identical, and so are the question, help
    /// and every feedback, although the full deal differs.
    #[test]
    fn hidden_substitutions_with_identical_views_keep_questions_and_feedback() {
        for fixture in TrainingFixture::ALL {
            let mut game = Game::new(fixture.seed(), RuleConfig::default());
            for action in fixture.actions() {
                game.apply(action).unwrap();
            }
            let all = game.omniscient_view();
            let seat = fixture.seat();
            let game_type = game.contract().unwrap().game;
            let [x, y] = [1, 2].map(|i| Seat::from_index(seat.index() + i));
            let pair = all.hands[x.index()].iter().find_map(|a| {
                all.hands[y.index()]
                    .iter()
                    .find(|&b| game_type.follow(b) == game_type.follow(a))
                    .map(|b| (a, b))
            });
            let (a, b) = pair.expect("a swappable pair exists");
            let mut other = Game::from_deal(
                swapped((&all.original_hands, all.original_skat), a, b),
                RuleConfig::default(),
            );
            for action in fixture.actions() {
                other.apply(action).expect("substituted prefix stays legal");
            }
            assert_ne!(other.omniscient_view().hands, all.hands, "{fixture:?}");
            let (view, other_view) = (game.view(seat), other.view(seat));
            assert_eq!(view, other_view, "{fixture:?}");
            for answer in [None, Some(0), Some(1), Some(2), Some(3)] {
                for level in [
                    AssistanceLevel::VisibleAid,
                    AssistanceLevel::AnswerBeforeReveal,
                ] {
                    let run = |v: PlayerView| {
                        let mut s = TrainingSession::new(fixture.topic(), level);
                        s.current = from_view(fixture, v, 1, level);
                        s.execute(TrainingCommand::Help).unwrap();
                        let help = s.view();
                        match answer {
                            Some(given) => s.execute(TrainingCommand::Answer(given)),
                            None => s.execute(TrainingCommand::Skip),
                        }
                        .unwrap();
                        (help, s.view(), s.take_events())
                    };
                    assert_eq!(run(view.clone()), run(other_view.clone()), "{fixture:?}");
                }
            }
        }
    }
}
