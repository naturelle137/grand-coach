// SPDX-License-Identifier: EUPL-1.2

//! Information sets (D-23): what one seat may know, and the explicitly named
//! omniscient view for open mode and post-game analysis.

use crate::bidding::BidRecord;
use crate::card::CardSet;
use crate::game::{Action, CompletedTrick, Game, Phase, RuleConfig, Trick};
use crate::rules::{Contract, GameType};
use crate::scoring::{GameResult, KontraLevel};
use crate::seat::Seat;

/// Everything one seat may know at this point of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerView {
    /// The viewing seat.
    pub seat: Seat,
    /// Rule options.
    pub config: RuleConfig,
    /// Current phase.
    pub phase: Phase,
    /// Seat whose turn it is.
    pub current_seat: Option<Seat>,
    /// Own cards (twelve for the declarer while discarding).
    pub hand: CardSet,
    /// Number of cards each seat holds.
    pub hand_sizes: [u8; 3],
    /// Bidding history.
    pub bidding: Vec<BidRecord>,
    /// Current or final bid.
    pub bid: u16,
    /// Declarer, once bidding is over.
    pub declarer: Option<Seat>,
    /// `Some(true)` for Hand, `Some(false)` after Skat pickup.
    pub hand_game: Option<bool>,
    /// Announced contract.
    pub contract: Option<Contract>,
    /// The two discarded cards; declarer only, after Drücken.
    pub own_skat: Option<CardSet>,
    /// The declarer's current cards in an Ouvert game once revealed.
    pub declarer_cards: Option<CardSet>,
    /// Completed tricks.
    pub tricks: Vec<CompletedTrick>,
    /// Trick in progress.
    pub current_trick: Trick,
    /// Kontra/Re level.
    pub kontra: KontraLevel,
    /// Legal actions if it is this seat's turn, otherwise empty.
    pub legal_actions: Vec<Action>,
    /// Result once the game is finished.
    pub result: Option<GameResult>,
}

impl PlayerView {
    /// Follow classes each seat has publicly failed to serve, including the
    /// trick on the table. An empty set means no such evidence yet.
    pub fn proven_voids(&self) -> [CardSet; 3] {
        let mut voids = [CardSet::EMPTY; 3];
        if let Some(contract) = self.contract {
            for trick in &self.tricks {
                note_voids(contract.game, &mut voids, completed(trick));
            }
            note_voids(contract.game, &mut voids, self.current_trick);
        }
        voids
    }
}

fn completed(trick: &CompletedTrick) -> Trick {
    Trick {
        leader: Some(trick.leader),
        cards: trick.cards.map(Some),
    }
}

/// A seat that did not follow the led class holds no card of it.
fn note_voids(game: GameType, voids: &mut [CardSet; 3], trick: Trick) {
    let (Some(leader), Some(led)) = (trick.leader, trick.led()) else {
        return;
    };
    let class = game.follow(led);
    for (i, card) in trick.cards.iter().enumerate().skip(1) {
        if let Some(card) = card
            && game.follow(*card) != class
        {
            voids[Seat::from_index(leader.index() + i).index()] |= game.cards_of(class);
        }
    }
}

/// Full knowledge of the game. Only for open mode and post-game analysis;
/// its use is visible in signatures (D-23).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OmniscientView {
    /// Current cards of every seat.
    pub hands: [CardSet; 3],
    /// Current Skat (original, or discarded after Drücken; empty while the
    /// declarer holds twelve cards).
    pub skat: CardSet,
    /// Original Skat as dealt.
    pub original_skat: CardSet,
    /// Original hands as dealt.
    pub original_hands: [CardSet; 3],
    /// The view of each seat.
    pub seats: [PlayerView; 3],
}

impl Game {
    /// What `seat` may know.
    pub fn view(&self, seat: Seat) -> PlayerView {
        let is_declarer = self.declarer == Some(seat);
        PlayerView {
            seat,
            config: self.config,
            phase: self.phase,
            current_seat: self.current_seat(),
            hand: self.hands[seat.index()],
            hand_sizes: self.hands.map(|h| h.len() as u8),
            bidding: self.bidding.history.clone(),
            bid: self.bid(),
            declarer: self.declarer,
            hand_game: self.hand_game,
            contract: self.contract,
            own_skat: (is_declarer && self.discarded).then_some(self.skat),
            declarer_cards: self
                .revealed
                .then(|| self.hands[self.declarer.expect("revealed implies declarer").index()]),
            tricks: self.tricks.clone(),
            current_trick: self.current,
            kontra: self.kontra,
            legal_actions: if self.current_seat() == Some(seat) {
                self.legal_actions()
            } else {
                Vec::new()
            },
            result: self.result,
        }
    }

    /// Full knowledge. Use only for open mode and post-game analysis.
    pub fn omniscient_view(&self) -> OmniscientView {
        OmniscientView {
            hands: self.hands,
            skat: self.skat,
            original_skat: self.deal.skat(),
            original_hands: Seat::ALL.map(|s| self.deal.hand(s)),
            seats: Seat::ALL.map(|s| self.view(s)),
        }
    }
}
