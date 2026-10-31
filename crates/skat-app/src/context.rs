// SPDX-License-Identifier: EUPL-1.2

//! What the human saw at one of their own decisions (issue #47): the
//! permitted snapshot behind a debriefing moment.

use serde::{Deserialize, Serialize};
use skat_core::{Action, Call, Card, Contract, KontraLevel, Phase, PlayerView, Seat};

use crate::coach::DecisionKey;
use crate::session::PlayerId;
use crate::sort::sort_hand;
use crate::view::{PlayedCard, TrickView, current_trick, trick_view};

/// The human's own view of the table at one decision, kept for the
/// debriefing. It is built from the human's [`PlayerView`] when the
/// decision is taken and never changes afterwards: no card of another hand,
/// no unrevealed Skat and nothing that happened later (D-23). The own Skat
/// and an Ouvert declarer's cards appear only if the human could see them
/// at that moment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Number of the game, from 1.
    pub game_number: u32,
    /// Actions applied in that game before the decision.
    pub actions: u32,
    /// Dealer of that game.
    pub dealer: PlayerId,
    /// Seat of each player in that game (index: [`PlayerId`]).
    pub seats: [Seat; 3],
    /// Phase of the decision.
    pub phase: Phase,
    /// The human's cards then, in display order (twelve while discarding).
    pub hand: Vec<Card>,
    /// Cards each player held then (index: [`PlayerId`]).
    pub hand_sizes: [u8; 3],
    /// Bidding so far.
    pub bidding: Vec<(PlayerId, Call)>,
    /// Bid value reached so far.
    pub bid: u16,
    /// Declarer, once bidding was over.
    pub declarer: Option<PlayerId>,
    /// `Some(true)` for Hand, `Some(false)` after Skat pickup, once decided.
    pub hand_game: Option<bool>,
    /// Announced contract, once announced.
    pub contract: Option<Contract>,
    /// Kontra/Re level then.
    pub kontra: KontraLevel,
    /// The two cards the human had discarded as declarer, after Drücken.
    pub own_skat: Option<Vec<Card>>,
    /// An Ouvert declarer's open cards, once revealed, as they lay then.
    pub declarer_cards: Option<Vec<Card>>,
    /// Completed tricks before the decision, in order.
    pub tricks: Vec<TrickView>,
    /// The trick in progress before the decision.
    pub current_trick: Vec<PlayedCard>,
    /// The action the human chose.
    pub action: Action,
}

impl DecisionContext {
    /// The snapshot of `view`, the human's view before `action` is applied
    /// at decision `key` in a game dealt by `dealer`.
    pub(crate) fn new(
        key: DecisionKey,
        dealer: PlayerId,
        view: &PlayerView,
        action: Action,
    ) -> DecisionContext {
        let game = view.contract.map(|c| c.game);
        let player = |seat: Seat| PlayerId::on_seat(seat, dealer);
        DecisionContext {
            game_number: key.number,
            actions: key.actions as u32,
            dealer,
            seats: PlayerId::ALL.map(|p| p.seat(dealer)),
            phase: view.phase,
            hand: sort_hand(view.hand, game),
            hand_sizes: PlayerId::ALL.map(|p| view.hand_sizes[p.seat(dealer).index()]),
            bidding: view
                .bidding
                .iter()
                .map(|r| (player(r.seat), r.call))
                .collect(),
            bid: view.bid,
            declarer: view.declarer.map(player),
            hand_game: view.hand_game,
            contract: view.contract,
            kontra: view.kontra,
            own_skat: view.own_skat.map(|s| sort_hand(s, game)),
            declarer_cards: view.declarer_cards.map(|s| sort_hand(s, game)),
            tricks: view.tricks.iter().map(|t| trick_view(t, dealer)).collect(),
            current_trick: current_trick(view, dealer),
            action,
        }
    }

    /// Seat of `player` in that game.
    pub fn seat(&self, player: PlayerId) -> Seat {
        self.seats[player.index()]
    }

    /// Player on `seat` in that game.
    pub fn player(&self, seat: Seat) -> PlayerId {
        PlayerId::on_seat(seat, self.dealer)
    }

    /// The trick of the decision, from 1, during card play.
    pub fn trick(&self) -> Option<u8> {
        (self.phase == Phase::Spiel).then_some(self.tricks.len() as u8 + 1)
    }
}
