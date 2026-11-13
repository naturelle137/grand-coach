// SPDX-License-Identifier: EUPL-1.2

//! View model of the table for the human player.

use serde::{Deserialize, Serialize};
use skat_core::{
    Action, Call, Card, CompletedTrick, Contract, GameResult, GameType, KontraLevel,
    OmniscientView, Phase, PlayerView, Seat,
};

use crate::coach::CoachView;
use crate::figures::SeriesCast;
use crate::quiz::AssistView;
use crate::series::{ScoreSheet, SeriesResult};
use crate::session::{PlayerId, SessionConfig};
use crate::sort::sort_hand;

/// Everything a frontend shows. Built from the human's [`PlayerView`], so
/// it never contains hidden cards of the opponents or an unrevealed Skat
/// (D-23); the one exception is [`TableView::open_cards`] with an open
/// display or in God Mode (D-62, D-68).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableView {
    /// Settings of the session.
    pub config: SessionConfig,
    /// Number of the current game, from 1.
    pub game_number: u32,
    /// Dealer of the current game.
    pub dealer: PlayerId,
    /// The three players (index: [`PlayerId`]).
    pub players: [SeatInfo; 3],
    /// Current phase of the game.
    pub phase: Phase,
    /// Player whose turn it is; `None` when the game is over.
    pub acting: Option<PlayerId>,
    /// `true` if an opponent has to act ([`crate::Command::AdvanceOpponent`]).
    pub opponent_to_move: bool,
    /// Current or final bid.
    pub bid: u16,
    /// Bidding history.
    pub bidding: Vec<(PlayerId, Call)>,
    /// Declarer, once bidding is over.
    pub declarer: Option<PlayerId>,
    /// `Some(true)` for Hand, `Some(false)` after Skat pickup.
    pub hand_game: Option<bool>,
    /// Announced contract.
    pub contract: Option<Contract>,
    /// Kontra/Re level.
    pub kontra: KontraLevel,
    /// The human's cards in display order.
    pub hand: Vec<Card>,
    /// The two cards the human discarded as declarer.
    pub own_skat: Option<Vec<Card>>,
    /// Open cards of an Ouvert declarer, in display order.
    pub declarer_cards: Option<Vec<Card>>,
    /// Every card of the deal; only with an open display or in God Mode
    /// (D-62, D-68), `None` otherwise. Display only.
    pub open_cards: Option<OpenCards>,
    /// What the live coach analyses (D-68), independent of `open_cards`.
    pub coach_basis: CoachBasis,
    /// Trick in progress.
    pub current_trick: Vec<PlayedCard>,
    /// Last completed trick.
    pub last_trick: Option<TrickView>,
    /// Augen each player won in the completed tricks of this game (index:
    /// [`PlayerId`]): public, permitted knowledge. A Skat is never counted,
    /// not even the human's own one. Frontends conceal these running
    /// totals by default (issue #134).
    pub trick_augen: [u8; 3],
    /// Number of completed tricks.
    pub tricks_played: u8,
    /// Legal actions of the human; empty when it is not the human's turn.
    pub legal_actions: Vec<Action>,
    /// Result once the game is finished.
    pub result: Option<ResultView>,
    /// Score sheet of the series.
    pub sheet: ScoreSheet,
    /// The named opponents of the series by physical seat (D-72).
    pub cast: SeriesCast,
    /// `true` once the series is over.
    pub series_finished: bool,
    /// Final result once the series is over.
    pub series_result: Option<SeriesResult>,
    /// [`crate::Command::NextGame`] is possible.
    pub can_next_game: bool,
    /// [`crate::Command::EndSeries`] is possible.
    pub can_end_series: bool,
    /// The live coach.
    pub coach: CoachView,
    /// The optional counting aid and quiz.
    pub assist: AssistView,
    /// `true` in an isolated practice attempt ([`crate::Session::start_practice`]):
    /// one unscored game, never part of a series.
    pub practice: bool,
}

/// One player at the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatInfo {
    /// The player.
    pub player: PlayerId,
    /// Seat in the current deal.
    pub seat: Seat,
    /// Number of cards held.
    pub cards: u8,
    /// Last call in the bidding.
    pub last_call: Option<Call>,
    /// Tricks won in this game.
    pub tricks: u8,
    /// Seeger-Fabian total of the series.
    pub total: i32,
}

/// Every card of the deal as the human sees it with an open display or in
/// God Mode (D-62, D-68): the one place where the view model shows hidden
/// cards. Plain display data; no analysis accepts it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCards {
    /// Current cards of each player (index: [`PlayerId`]), in display
    /// order.
    pub hands: [Vec<Card>; 3],
    /// The Skat: as dealt, or as discarded after Drücken; empty while the
    /// declarer holds twelve cards.
    pub skat: Vec<Card>,
}

/// What the live coach analyses (D-68).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoachBasis {
    /// Only the human's own permitted view, as at a closed table: also with
    /// an open display.
    OwnView,
    /// The real deal with every card: only in God Mode (D-62).
    RealDeal,
}

/// The open cards of `all` for the players of a deal by `dealer`, sorted
/// for `game`.
pub(crate) fn open_cards(
    all: &OmniscientView,
    dealer: PlayerId,
    game: Option<GameType>,
) -> OpenCards {
    OpenCards {
        hands: PlayerId::ALL.map(|p| sort_hand(all.hands[p.seat(dealer).index()], game)),
        skat: sort_hand(all.skat, game),
    }
}

/// A card played by a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedCard {
    /// Player who played it.
    pub player: PlayerId,
    /// The card.
    pub card: Card,
}

/// A completed trick.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrickView {
    /// Cards in play order.
    pub cards: Vec<PlayedCard>,
    /// Player who won it.
    pub winner: PlayerId,
    /// Card points in the trick.
    pub augen: u8,
}

/// A finished game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultView {
    /// Declarer.
    pub declarer: PlayerId,
    /// Settled result from `skat-core`.
    pub result: GameResult,
    /// Seeger-Fabian points of this game per player.
    pub points: [i32; 3],
}

/// A completed trick with players instead of seats.
pub(crate) fn trick_view(t: &CompletedTrick, dealer: PlayerId) -> TrickView {
    TrickView {
        cards: (0..3)
            .map(|i| PlayedCard {
                player: PlayerId::on_seat(t.seat_of(i), dealer),
                card: t.cards[i],
            })
            .collect(),
        winner: PlayerId::on_seat(t.winner, dealer),
        augen: t.augen(),
    }
}

/// The cards of the trick in progress with the players who played them.
pub(crate) fn current_trick(view: &PlayerView, dealer: PlayerId) -> Vec<PlayedCard> {
    let Some(leader) = view.current_trick.leader else {
        return Vec::new();
    };
    view.current_trick
        .cards
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            c.map(|card| PlayedCard {
                player: PlayerId::on_seat(Seat::from_index(leader.index() + i), dealer),
                card,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_view(
    config: &SessionConfig,
    game_number: u32,
    dealer: PlayerId,
    sheet: &ScoreSheet,
    series_finished: bool,
    opponent_to_move: bool,
    view: &PlayerView,
    open_cards: Option<OpenCards>,
    practice: bool,
    coach: CoachView,
    assist: AssistView,
) -> TableView {
    let player = |seat: Seat| PlayerId::on_seat(seat, dealer);
    let game = view.contract.map(|c| c.game);
    let totals = sheet.totals();
    let players = PlayerId::ALL.map(|p| {
        let seat = p.seat(dealer);
        SeatInfo {
            player: p,
            seat,
            cards: view.hand_sizes[seat.index()],
            last_call: view
                .bidding
                .iter()
                .rev()
                .find(|r| r.seat == seat)
                .map(|r| r.call),
            tricks: view.tricks.iter().filter(|t| t.winner == seat).count() as u8,
            total: totals[p.index()],
        }
    });
    let game_over = view.current_seat.is_none();
    let result = view.result.map(|result| {
        let by_seat = result.seeger_fabian();
        ResultView {
            declarer: player(result.outcome.declarer),
            result,
            points: PlayerId::ALL.map(|p| by_seat[p.seat(dealer).index()]),
        }
    });
    TableView {
        config: *config,
        game_number,
        dealer,
        players,
        phase: view.phase,
        acting: view.current_seat.map(player),
        opponent_to_move,
        bid: view.bid,
        bidding: view
            .bidding
            .iter()
            .map(|r| (player(r.seat), r.call))
            .collect(),
        declarer: view.declarer.map(player),
        hand_game: view.hand_game,
        contract: view.contract,
        kontra: view.kontra,
        hand: sort_hand(view.hand, game),
        own_skat: view.own_skat.map(|s| sort_hand(s, game)),
        declarer_cards: view.declarer_cards.map(|s| sort_hand(s, game)),
        open_cards,
        coach_basis: config.coach_basis(),
        current_trick: current_trick(view, dealer),
        last_trick: view.tricks.last().map(|t| trick_view(t, dealer)),
        trick_augen: PlayerId::ALL.map(|p| {
            let seat = p.seat(dealer);
            view.tricks
                .iter()
                .filter(|t| t.winner == seat)
                .map(|t| t.augen())
                .sum()
        }),
        tricks_played: view.tricks.len() as u8,
        legal_actions: if series_finished {
            Vec::new()
        } else {
            view.legal_actions.clone()
        },
        result,
        sheet: sheet.clone(),
        cast: SeriesCast::for_seed(config.seed),
        series_finished,
        series_result: series_finished.then(|| sheet.result()),
        can_next_game: game_over && !series_finished && !practice,
        can_end_series: game_over
            && !series_finished
            && !practice
            && config.series == crate::session::SeriesLength::Free,
        coach,
        assist,
        practice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::CoachSettings;
    use crate::quiz::QuizState;
    use skat_core::{Announcement, CardSet, Deal, Game, Rank, RuleConfig, Suit};

    /// The human is Vorhand: player 2 deals.
    const DEALER: PlayerId = PlayerId(2);

    fn c(text: &str) -> Card {
        let suit = match &text[..1] {
            "C" => Suit::Kreuz,
            "S" => Suit::Pik,
            "H" => Suit::Herz,
            _ => Suit::Karo,
        };
        let rank = match &text[1..] {
            "7" => Rank::Sieben,
            "8" => Rank::Acht,
            "9" => Rank::Neun,
            "10" => Rank::Zehn,
            "J" => Rank::Bube,
            "Q" => Rank::Dame,
            "K" => Rank::Koenig,
            _ => Rank::Ass,
        };
        Card::new(suit, rank)
    }

    fn set(text: &str) -> CardSet {
        text.split_whitespace().map(c).collect()
    }

    fn table(game: &Game) -> TableView {
        let config = SessionConfig::default();
        let view = game.view(PlayerId::HUMAN.seat(DEALER));
        build_view(
            &config,
            1,
            DEALER,
            &ScoreSheet::default(),
            false,
            false,
            &view,
            None,
            false,
            CoachView::withheld(CoachSettings::default()),
            QuizState::new(0).view(&view, DEALER),
        )
    }

    /// Hand Grand of the human (Vorhand) with the Skat H10 DK (14 Augen),
    /// which nobody but the dealt deal knows; then the tricks as listed in
    /// play order, each led by the previous winner. Returns the running
    /// trick Augen after each trick.
    fn running(human: &str, left: &str, right: &str, tricks: &[&str]) -> Vec<[u8; 3]> {
        let deal = Deal::new([set(human), set(left), set(right)], set("H10 DK")).unwrap();
        let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
        // Mittelhand and Hinterhand pass, the human plays at 18 Hand.
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Bid(18)).unwrap();
        game.apply(Action::Hand).unwrap();
        game.apply(Action::Announce(Announcement::plain(GameType::Grand)))
            .unwrap();
        assert_eq!(table(&game).trick_augen, [0; 3]);
        let mut totals = Vec::new();
        for trick in tricks {
            for card in trick.split_whitespace() {
                game.apply(Action::Play(c(card))).unwrap();
            }
            totals.push(table(&game).trick_augen);
        }
        totals
    }

    #[test]
    fn trick_augen_count_completed_tricks_per_player_without_the_skat() {
        let totals = running(
            "CJ SJ HJ DJ CA SA HA DA C10 S10",
            "CK CQ C9 C8 C7 SK SQ S9 S8 S7",
            "HK HQ H9 H8 H7 D10 DQ D9 D8 D7",
            &[
                "CJ C7 H7",
                "SJ C8 H8",
                "HJ C9 H9",
                "DJ CQ HQ",
                "CA CK D7",
                "C10 SQ D8",
                "SA SK D9",
                "S10 S9 DQ",
                "HA S8 HK",
                "DA S7 D10",
            ],
        );
        let human: Vec<u8> = totals.iter().map(|t| t[0]).collect();
        assert_eq!(human, [2, 4, 6, 14, 29, 42, 57, 70, 85, 106]);
        assert!(totals.iter().all(|t| t[1] == 0 && t[2] == 0));
        // Every card but the unknown Skat: 120 − 14.
        assert_eq!(totals.last().unwrap().iter().sum::<u8>(), 106);
    }

    #[test]
    fn trick_augen_go_to_the_trick_winner() {
        // As above, but the left defender holds HA instead of S7.
        let totals = running(
            "CJ SJ HJ DJ CA SA S7 DA C10 S10",
            "CK CQ C9 C8 C7 SK SQ S9 S8 HA",
            "HK HQ H9 H8 H7 D10 DQ D9 D8 D7",
            &[
                "CJ C7 H7",
                "SJ C8 H8",
                "HJ C9 H9",
                "DJ CQ HQ",
                "CA CK D7",
                "C10 SQ D8",
                "SA SK D9",
                "S10 S9 DQ",
                // The left defender wins S8 over S7, then leads HA.
                "S7 S8 HK",
                "HA D10 DA",
            ],
        );
        assert_eq!(totals[7], [70, 0, 0]);
        assert_eq!(totals[8], [70, 4, 0]);
        assert_eq!(totals[9], [70, 36, 0]);
        assert_eq!(totals[9].iter().sum::<u8>(), 120 - 14);
    }
}
