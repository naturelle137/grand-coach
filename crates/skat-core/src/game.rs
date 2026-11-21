// SPDX-License-Identifier: EUPL-1.2

//! The game state machine (D-21): one [`Game`] with a [`Phase`], offering
//! [`Game::legal_actions`] and [`Game::apply`].

use crate::bidding::{BidOutcome, BidRecord, Bidding, Stage, is_reizwert, reizwerte};
use crate::card::{Card, CardSet};
use crate::deal::Deal;
use crate::rules::{Announcement, Contract, GameType, legal_cards, trick_winner};
use crate::scoring::{GameOutcome, GameResult, KontraLevel, settle, spitzen};
use crate::seat::Seat;

/// Rule options (architecture: "further options only when decided").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RuleConfig {
    /// Kontra/Re per D-03 and D-26. Off by default (not part of the ISkO).
    pub kontra_re: bool,
}

/// Phase of a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Phase {
    /// Bidding.
    Reizen,
    /// Declarer chooses Hand or picks up the Skat.
    SkatAufnahme,
    /// Declarer discards two cards.
    Druecken,
    /// Declarer announces the game.
    Ansage,
    /// Card play, including Kontra/Re decisions.
    Spiel,
    /// Game finished and settled.
    Beendet,
    /// All passed; the hand is thrown in and a new deal follows (D-04).
    Eingepasst,
}

/// An action of the seat whose turn it is ([`Game::current_seat`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    /// Say a bid value (asking seat), or play at 18 as Vorhand after both
    /// other seats passed without a bid.
    Bid(u16),
    /// Hold the current bid (answering seat).
    Hold,
    /// Pass in the bidding.
    Pass,
    /// Play Hand, without picking up the Skat.
    Hand,
    /// Pick up the Skat.
    TakeSkat,
    /// Drücken: put exactly two cards into the Skat.
    Discard(CardSet),
    /// Announce the game.
    Announce(Announcement),
    /// Play a card.
    Play(Card),
    /// Say Kontra (defender).
    Kontra,
    /// Decline Kontra in the Ouvert decision round (defender).
    KeinKontra,
    /// Answer Kontra with Re (declarer).
    Re,
    /// Answer Kontra without Re (declarer).
    KeinRe,
}

/// What happened. Events contain only public information: hidden cards
/// (Skat, discards, hands) never appear in them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    /// A seat bid a value.
    Bid {
        /// Bidding seat.
        seat: Seat,
        /// Value.
        value: u16,
    },
    /// A seat held a value.
    Held {
        /// Holding seat.
        seat: Seat,
        /// Value.
        value: u16,
    },
    /// A seat passed.
    Passed {
        /// Passing seat.
        seat: Seat,
    },
    /// Bidding is over.
    BiddingWon {
        /// Declarer.
        declarer: Seat,
        /// Final bid.
        bid: u16,
    },
    /// All seats passed.
    Eingepasst,
    /// The declarer plays Hand.
    HandChosen {
        /// Declarer.
        declarer: Seat,
    },
    /// The declarer picked up the Skat.
    SkatTaken {
        /// Declarer.
        declarer: Seat,
    },
    /// The declarer discarded two cards.
    Discarded {
        /// Declarer.
        declarer: Seat,
    },
    /// The game was announced.
    Announced {
        /// Declarer.
        declarer: Seat,
        /// Contract.
        contract: Contract,
    },
    /// A defender said Kontra.
    Kontra {
        /// Defender.
        seat: Seat,
    },
    /// A defender declined Kontra (Ouvert round).
    KeinKontra {
        /// Defender.
        seat: Seat,
    },
    /// The declarer said Re.
    Re {
        /// Declarer.
        seat: Seat,
    },
    /// The declarer declined Re.
    KeinRe {
        /// Declarer.
        seat: Seat,
    },
    /// Ouvert: the declarer's ten cards are laid open.
    Revealed {
        /// Declarer.
        declarer: Seat,
        /// The revealed cards.
        cards: CardSet,
    },
    /// A card was played.
    CardPlayed {
        /// Playing seat.
        seat: Seat,
        /// Card.
        card: Card,
    },
    /// A trick was completed.
    TrickWon {
        /// The trick.
        trick: CompletedTrick,
    },
    /// The game is over.
    GameFinished {
        /// Settled result.
        result: GameResult,
    },
}

/// Why an action was rejected. The state is unchanged after an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RuleError {
    /// The game is over (finished or thrown in).
    GameOver,
    /// This kind of action is not possible at this point.
    UnexpectedAction {
        /// Current phase.
        phase: Phase,
    },
    /// Not a valid bid value, or (Vorhand alone) not 18.
    InvalidBidValue {
        /// Rejected value.
        value: u16,
    },
    /// The bid does not exceed the current bid.
    BidTooLow {
        /// Rejected value.
        value: u16,
        /// Current bid.
        current: u16,
    },
    /// Discard is not exactly two cards of the declarer's hand.
    InvalidDiscard,
    /// Combination of game type, Schneider, Schwarz, Ouvert and Hand is not
    /// allowed.
    InvalidAnnouncement,
    /// A Null game whose fixed value is below the bid (ISkO 3.4.4).
    NullValueBelowBid {
        /// Value of the announced Null game.
        value: u16,
        /// Final bid.
        bid: u16,
    },
    /// The card is not in the seat's hand.
    CardNotInHand {
        /// Rejected card.
        card: Card,
    },
    /// The seat must follow suit (Bedienen).
    MustFollowSuit {
        /// Rejected card.
        card: Card,
    },
    /// Kontra is switched off, the seat is not eligible, Kontra was already
    /// said, or the seat already played a card.
    KontraNotAllowed,
}

/// A trick in progress, cards in play order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trick {
    /// Seat that led; `None` before the first card of a game is led.
    pub leader: Option<Seat>,
    /// Cards played so far, in play order.
    pub cards: [Option<Card>; 3],
}

impl Trick {
    /// Number of cards played.
    pub fn len(&self) -> usize {
        self.cards.iter().flatten().count()
    }

    /// `true` if no card is played yet.
    pub fn is_empty(&self) -> bool {
        self.cards[0].is_none()
    }

    /// The led card.
    pub fn led(&self) -> Option<Card> {
        self.cards[0]
    }
}

/// A finished trick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompletedTrick {
    /// Seat that led.
    pub leader: Seat,
    /// Cards in play order.
    pub cards: [Card; 3],
    /// Seat that won the trick.
    pub winner: Seat,
}

impl CompletedTrick {
    /// Card points in the trick.
    pub fn augen(&self) -> u8 {
        self.cards.iter().map(|c| c.augen()).sum()
    }

    /// Seat that played the card at play position `i`.
    pub fn seat_of(&self, i: usize) -> Seat {
        Seat::from_index(self.leader.index() + i)
    }
}

/// Sub-step of the [`Phase::Spiel`] phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Step {
    /// Normal card play.
    Play,
    /// Ouvert: this defender decides Kontra or KeinKontra before the reveal.
    OuvertKontra(Seat),
    /// The declarer answers Kontra.
    AwaitRe,
}

#[cfg(feature = "serde")]
mod wire;

/// Full state of one game. Holds everything, including hidden cards; hand it
/// to AI or coach only through [`Game::view`] (D-23). Deserialising rejects
/// states that no sequence of actions can reach.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "wire::GameData")
)]
pub struct Game {
    pub(crate) config: RuleConfig,
    pub(crate) deal: Deal,
    pub(crate) hands: [CardSet; 3],
    pub(crate) skat: CardSet,
    pub(crate) phase: Phase,
    pub(crate) bidding: Bidding,
    pub(crate) declarer: Option<Seat>,
    pub(crate) bid: u16,
    pub(crate) hand_game: Option<bool>,
    pub(crate) discarded: bool,
    pub(crate) contract: Option<Contract>,
    pub(crate) tricks: Vec<CompletedTrick>,
    pub(crate) current: Trick,
    pub(crate) played: [bool; 3],
    pub(crate) kontra: KontraLevel,
    pub(crate) kontra_by: Option<Seat>,
    step: Step,
    pub(crate) revealed: bool,
    pub(crate) result: Option<GameResult>,
}

impl Game {
    /// New game with the deal for `seed` (D-22).
    pub fn new(seed: u64, config: RuleConfig) -> Game {
        Game::from_deal(Deal::from_seed(seed), config)
    }

    /// New game with a fixed deal.
    pub fn from_deal(deal: Deal, config: RuleConfig) -> Game {
        Game {
            config,
            deal,
            hands: Seat::ALL.map(|s| deal.hand(s)),
            skat: deal.skat(),
            phase: Phase::Reizen,
            bidding: Bidding::new(),
            declarer: None,
            bid: 0,
            hand_game: None,
            discarded: false,
            contract: None,
            tricks: Vec::with_capacity(10),
            current: Trick::default(),
            played: [false; 3],
            kontra: KontraLevel::None,
            kontra_by: None,
            step: Step::Play,
            revealed: false,
            result: None,
        }
    }

    /// Rule options of this game.
    pub fn config(&self) -> RuleConfig {
        self.config
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Declarer, once bidding is over.
    pub fn declarer(&self) -> Option<Seat> {
        self.declarer
    }

    /// Announced contract.
    pub fn contract(&self) -> Option<Contract> {
        self.contract
    }

    /// Result once the game is finished.
    pub fn result(&self) -> Option<&GameResult> {
        self.result.as_ref()
    }

    /// Bidding history.
    pub fn bidding_history(&self) -> &[BidRecord] {
        &self.bidding.history
    }

    /// Completed tricks.
    pub fn tricks(&self) -> &[CompletedTrick] {
        &self.tricks
    }

    /// Trick in progress.
    pub fn current_trick(&self) -> Trick {
        self.current
    }

    /// Current bid (0 before the first bid).
    pub fn bid(&self) -> u16 {
        if self.phase == Phase::Reizen {
            self.bidding.bid
        } else {
            self.bid
        }
    }

    /// Kontra/Re level so far.
    pub fn kontra(&self) -> KontraLevel {
        self.kontra
    }

    /// The defender who said Kontra, if any.
    pub fn kontra_by(&self) -> Option<Seat> {
        self.kontra_by
    }

    /// `true` once an Ouvert declarer's cards are face up.
    pub fn is_revealed(&self) -> bool {
        self.revealed
    }

    /// Seat whose turn it is; `None` when the game is over.
    pub fn current_seat(&self) -> Option<Seat> {
        match self.phase {
            Phase::Reizen => Some(self.bidding.current_seat()),
            Phase::SkatAufnahme | Phase::Druecken | Phase::Ansage => self.declarer,
            Phase::Spiel => Some(match self.step {
                Step::Play => self.next_to_play(),
                Step::OuvertKontra(seat) => seat,
                Step::AwaitRe => self.declarer?,
            }),
            Phase::Beendet | Phase::Eingepasst => None,
        }
    }

    fn next_to_play(&self) -> Seat {
        let leader = self.current.leader.unwrap_or(Seat::Vorhand);
        Seat::from_index(leader.index() + self.current.len())
    }

    fn declarer_seat(&self) -> Seat {
        self.declarer.expect("declarer is set after bidding")
    }

    fn game_type(&self) -> GameType {
        self.contract.expect("contract is set during play").game
    }

    /// Whether `seat` may say Kontra right now during card play (D-26.2).
    fn kontra_allowed_in_play(&self, seat: Seat) -> bool {
        self.config.kontra_re
            && self.kontra == KontraLevel::None
            && !self.contract.is_some_and(|c| c.ouvert)
            && Some(seat) != self.declarer
            && !self.played[seat.index()]
            && self.bidding.kontra_eligible(seat)
    }

    /// Next defender after `after` (or the first one if `None`) that takes
    /// part in the Ouvert Kontra round, in seat order.
    fn next_ouvert_decider(&self, after: Option<Seat>) -> Option<Seat> {
        if !self.config.kontra_re {
            return None;
        }
        let start = after.map_or(0, |s| s.index() + 1);
        Seat::ALL[start.min(3)..]
            .iter()
            .copied()
            .find(|&s| Some(s) != self.declarer && self.bidding.kontra_eligible(s))
    }

    /// All legal actions of the current seat. Empty when the game is over.
    pub fn legal_actions(&self) -> Vec<Action> {
        match self.phase {
            Phase::Reizen => {
                let b = &self.bidding;
                if b.stage == Stage::VorhandAlone {
                    vec![Action::Bid(18), Action::Pass]
                } else if b.awaiting_holder {
                    vec![Action::Hold, Action::Pass]
                } else {
                    let mut actions: Vec<Action> = reizwerte()
                        .into_iter()
                        .filter(|&v| v > b.bid)
                        .map(Action::Bid)
                        .collect();
                    actions.push(Action::Pass);
                    actions
                }
            }
            Phase::SkatAufnahme => vec![Action::Hand, Action::TakeSkat],
            Phase::Druecken => {
                let cards: Vec<Card> = self.hands[self.declarer_seat().index()].iter().collect();
                let mut actions = Vec::with_capacity(66);
                for (i, &a) in cards.iter().enumerate() {
                    for &b in &cards[i + 1..] {
                        actions.push(Action::Discard(CardSet::single(a) | CardSet::single(b)));
                    }
                }
                actions
            }
            Phase::Ansage => {
                let hand = self.hand_game == Some(true);
                Announcement::all(hand)
                    .into_iter()
                    .filter(|&a| {
                        Contract::new(a, hand)
                            .null_value()
                            .is_none_or(|v| v >= self.bid)
                    })
                    .map(Action::Announce)
                    .collect()
            }
            Phase::Spiel => match self.step {
                Step::AwaitRe => vec![Action::Re, Action::KeinRe],
                Step::OuvertKontra(_) => vec![Action::Kontra, Action::KeinKontra],
                Step::Play => {
                    let seat = self.next_to_play();
                    let mut actions: Vec<Action> = legal_cards(
                        self.game_type(),
                        self.hands[seat.index()],
                        self.current.led(),
                    )
                    .iter()
                    .map(Action::Play)
                    .collect();
                    if self.kontra_allowed_in_play(seat) {
                        actions.push(Action::Kontra);
                    }
                    actions
                }
            },
            Phase::Beendet | Phase::Eingepasst => Vec::new(),
        }
    }

    /// Checks an action without changing anything.
    pub fn validate(&self, action: Action) -> Result<(), RuleError> {
        let unexpected = Err(RuleError::UnexpectedAction { phase: self.phase });
        match (self.phase, action) {
            (Phase::Beendet | Phase::Eingepasst, _) => Err(RuleError::GameOver),
            (Phase::Reizen, Action::Bid(value)) => {
                let b = &self.bidding;
                if b.awaiting_holder && b.stage != Stage::VorhandAlone {
                    unexpected
                } else if !is_reizwert(value) || (b.stage == Stage::VorhandAlone && value != 18) {
                    Err(RuleError::InvalidBidValue { value })
                } else if value <= b.bid {
                    Err(RuleError::BidTooLow {
                        value,
                        current: b.bid,
                    })
                } else {
                    Ok(())
                }
            }
            (Phase::Reizen, Action::Hold) => {
                let b = &self.bidding;
                if b.awaiting_holder && b.stage != Stage::VorhandAlone {
                    Ok(())
                } else {
                    unexpected
                }
            }
            (Phase::Reizen, Action::Pass) => Ok(()),
            (Phase::SkatAufnahme, Action::Hand | Action::TakeSkat) => Ok(()),
            (Phase::Druecken, Action::Discard(cards)) => {
                let hand = self.hands[self.declarer_seat().index()];
                if cards.len() == 2 && cards.is_subset(hand) {
                    Ok(())
                } else {
                    Err(RuleError::InvalidDiscard)
                }
            }
            (Phase::Ansage, Action::Announce(announcement)) => {
                let hand = self.hand_game == Some(true);
                if !announcement.is_valid(hand) {
                    return Err(RuleError::InvalidAnnouncement);
                }
                match Contract::new(announcement, hand).null_value() {
                    Some(value) if value < self.bid => Err(RuleError::NullValueBelowBid {
                        value,
                        bid: self.bid,
                    }),
                    _ => Ok(()),
                }
            }
            (Phase::Spiel, _) => self.validate_spiel(action),
            _ => unexpected,
        }
    }

    fn validate_spiel(&self, action: Action) -> Result<(), RuleError> {
        let unexpected = Err(RuleError::UnexpectedAction { phase: self.phase });
        match (self.step, action) {
            (Step::AwaitRe, Action::Re | Action::KeinRe) => Ok(()),
            (Step::OuvertKontra(_), Action::Kontra | Action::KeinKontra) => Ok(()),
            (Step::Play, Action::Play(card)) => {
                let seat = self.next_to_play();
                let hand = self.hands[seat.index()];
                if !hand.contains(card) {
                    Err(RuleError::CardNotInHand { card })
                } else if !legal_cards(self.game_type(), hand, self.current.led()).contains(card) {
                    Err(RuleError::MustFollowSuit { card })
                } else {
                    Ok(())
                }
            }
            (Step::Play, Action::Kontra) => {
                if self.kontra_allowed_in_play(self.next_to_play()) {
                    Ok(())
                } else {
                    Err(RuleError::KontraNotAllowed)
                }
            }
            _ => unexpected,
        }
    }

    /// Applies an action of the current seat. Validates first; on error the
    /// state is unchanged (D-21).
    pub fn apply(&mut self, action: Action) -> Result<Vec<Event>, RuleError> {
        self.validate(action)?;
        let seat = self.current_seat().expect("validated: game not over");
        let mut events = Vec::new();
        match action {
            Action::Bid(value) => {
                events.push(Event::Bid { seat, value });
                let outcome = self.bidding.bid(value);
                self.after_bidding_step(outcome, &mut events);
            }
            Action::Hold => {
                events.push(Event::Held {
                    seat,
                    value: self.bidding.bid,
                });
                let outcome = self.bidding.hold();
                self.after_bidding_step(outcome, &mut events);
            }
            Action::Pass => {
                events.push(Event::Passed { seat });
                let outcome = self.bidding.pass();
                self.after_bidding_step(outcome, &mut events);
            }
            Action::Hand => {
                self.hand_game = Some(true);
                self.phase = Phase::Ansage;
                events.push(Event::HandChosen { declarer: seat });
            }
            Action::TakeSkat => {
                self.hand_game = Some(false);
                self.hands[seat.index()] |= self.skat;
                self.skat = CardSet::EMPTY;
                self.phase = Phase::Druecken;
                events.push(Event::SkatTaken { declarer: seat });
            }
            Action::Discard(cards) => {
                self.hands[seat.index()] -= cards;
                self.skat = cards;
                self.discarded = true;
                self.phase = Phase::Ansage;
                events.push(Event::Discarded { declarer: seat });
            }
            Action::Announce(announcement) => {
                let contract = Contract::new(announcement, self.hand_game == Some(true));
                self.contract = Some(contract);
                self.phase = Phase::Spiel;
                events.push(Event::Announced {
                    declarer: seat,
                    contract,
                });
                if contract.ouvert {
                    match self.next_ouvert_decider(None) {
                        Some(decider) => self.step = Step::OuvertKontra(decider),
                        None => self.reveal(&mut events),
                    }
                }
            }
            Action::Kontra => {
                self.kontra = KontraLevel::Kontra;
                self.kontra_by = Some(seat);
                self.step = Step::AwaitRe;
                events.push(Event::Kontra { seat });
            }
            Action::KeinKontra => {
                events.push(Event::KeinKontra { seat });
                match self.next_ouvert_decider(Some(seat)) {
                    Some(decider) => self.step = Step::OuvertKontra(decider),
                    None => self.reveal(&mut events),
                }
            }
            Action::Re | Action::KeinRe => {
                if action == Action::Re {
                    self.kontra = KontraLevel::Re;
                    events.push(Event::Re { seat });
                } else {
                    events.push(Event::KeinRe { seat });
                }
                if self.contract.is_some_and(|c| c.ouvert) && !self.revealed {
                    self.reveal(&mut events);
                } else {
                    self.step = Step::Play;
                }
            }
            Action::Play(card) => self.play(seat, card, &mut events),
        }
        Ok(events)
    }

    fn after_bidding_step(&mut self, outcome: BidOutcome, events: &mut Vec<Event>) {
        match outcome {
            BidOutcome::Continue => {}
            BidOutcome::Declarer(declarer, bid) => {
                self.declarer = Some(declarer);
                self.bid = bid;
                self.phase = Phase::SkatAufnahme;
                events.push(Event::BiddingWon { declarer, bid });
            }
            BidOutcome::Eingepasst => {
                self.phase = Phase::Eingepasst;
                events.push(Event::Eingepasst);
            }
        }
    }

    fn reveal(&mut self, events: &mut Vec<Event>) {
        let declarer = self.declarer_seat();
        self.revealed = true;
        self.step = Step::Play;
        events.push(Event::Revealed {
            declarer,
            cards: self.hands[declarer.index()],
        });
    }

    fn play(&mut self, seat: Seat, card: Card, events: &mut Vec<Event>) {
        self.hands[seat.index()].remove(card);
        self.played[seat.index()] = true;
        let position = self.current.len();
        if position == 0 {
            self.current.leader = Some(seat);
        }
        self.current.cards[position] = Some(card);
        events.push(Event::CardPlayed { seat, card });
        if position < 2 {
            return;
        }
        let leader = self.current.leader.expect("trick has a leader");
        let cards = self.current.cards.map(|c| c.expect("trick is complete"));
        let winner = Seat::from_index(leader.index() + trick_winner(self.game_type(), cards));
        let trick = CompletedTrick {
            leader,
            cards,
            winner,
        };
        self.tricks.push(trick);
        self.current = Trick {
            leader: Some(winner),
            cards: [None; 3],
        };
        events.push(Event::TrickWon { trick });
        let null_lost = self.game_type() == GameType::Null && winner == self.declarer_seat();
        if null_lost || self.tricks.len() == 10 {
            self.finish(events);
        }
    }

    fn finish(&mut self, events: &mut Vec<Event>) {
        let result = settle(&self.game_outcome());
        self.result = Some(result);
        self.phase = Phase::Beendet;
        events.push(Event::GameFinished { result });
    }

    /// The facts the settlement is computed from.
    pub(crate) fn game_outcome(&self) -> GameOutcome {
        let declarer = self.declarer_seat();
        let contract = self.contract.expect("contract is set during play");
        let won: Vec<&CompletedTrick> = self
            .tricks
            .iter()
            .filter(|t| t.winner == declarer)
            .collect();
        GameOutcome {
            declarer,
            contract,
            bid: self.bid,
            spitzen: spitzen(contract.game, self.deal.hand(declarer) | self.deal.skat()),
            declarer_augen: won.iter().map(|t| t.augen()).sum::<u8>() + self.skat.augen(),
            declarer_tricks: won.len() as u8,
            tricks_played: self.tricks.len() as u8,
            kontra: self.kontra,
        }
    }
}
