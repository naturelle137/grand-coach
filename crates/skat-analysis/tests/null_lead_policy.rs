// SPDX-License-Identifier: EUPL-1.2

//! Cloud-owned #176 literal regression, accepted1695 original1694 SHA256
//! 8a608bf13b958750b859e45a5f13f02d29708d2db1c1622e0021ef10b0bccef7.
//! Separate source-informed addendum e5f4cfaaf17ca0991cc73fc0a904f7defb8c86fc50afe2bc29d18562135dfcb7:
//! six power values 1..8 -> 0..7, mixed None and phase-only Some.
//! Policy, complete views/Knowledge, choices and 286 paths unchanged.
//! Original is not rewritten. No local-code ownership or acceptance claimed.
use skat_analysis::{
    Knowledge,
    baseline::{self, Branch, CardRule, TieBreak},
};
use skat_core::*;
pub(crate) fn card(n: u8) -> Card {
    Card::from_index(n).unwrap()
}
pub(crate) struct Case {
    pub id: &'static str,
    pub deal: Deal,
    pub history: Vec<Action>,
    pub view: PlayerView,
    pub knowledge: Knowledge,
    pub expected: CardRule,
    pub paths: Vec<([Card; 3], Seat)>,
}
impl Case {
    pub(crate) fn game(&self) -> Game {
        let mut g = Game::from_deal(self.deal, RuleConfig { kontra_re: false });
        for &a in &self.history {
            g.apply(a).unwrap();
        }
        g
    }
}
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "A-hand-tie",
            deal: Deal::new(
                [
                    CardSet::from_bits(2214726045),
                    CardSet::from_bits(156542018),
                    CardSet::from_bits(313086496),
                ],
                CardSet::from_bits(1610612736),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(2214726045),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(true),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: None,
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(0)),
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(8)),
                    Action::Play(card(11)),
                    Action::Play(card(17)),
                    Action::Play(card(26)),
                    Action::Play(card(31)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(2214726045),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(0),
                skat: None,
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(0),
                branch: Branch::Null,
                winners: vec![
                    card(0),
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(8),
                    card(11),
                    card(17),
                    card(26),
                    card(31),
                ],
                losers: vec![],
                candidates: vec![
                    card(0),
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(8),
                    card(11),
                    card(17),
                    card(26),
                    card(31),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 0),
            },
            paths: vec![
                ([card(0), card(1), card(5)], Seat::Hinterhand),
                ([card(0), card(6), card(5)], Seat::Mittelhand),
                ([card(2), card(1), card(5)], Seat::Hinterhand),
                ([card(2), card(6), card(5)], Seat::Mittelhand),
                ([card(3), card(1), card(5)], Seat::Hinterhand),
                ([card(3), card(6), card(5)], Seat::Mittelhand),
                ([card(4), card(1), card(5)], Seat::Hinterhand),
                ([card(4), card(6), card(5)], Seat::Mittelhand),
                ([card(7), card(1), card(5)], Seat::Vorhand),
                ([card(7), card(6), card(5)], Seat::Vorhand),
                ([card(8), card(10), card(9)], Seat::Mittelhand),
                ([card(8), card(10), card(12)], Seat::Hinterhand),
                ([card(8), card(10), card(14)], Seat::Hinterhand),
                ([card(8), card(13), card(9)], Seat::Mittelhand),
                ([card(8), card(13), card(12)], Seat::Mittelhand),
                ([card(8), card(13), card(14)], Seat::Hinterhand),
                ([card(8), card(15), card(9)], Seat::Mittelhand),
                ([card(8), card(15), card(12)], Seat::Mittelhand),
                ([card(8), card(15), card(14)], Seat::Mittelhand),
                ([card(11), card(10), card(9)], Seat::Vorhand),
                ([card(11), card(10), card(12)], Seat::Hinterhand),
                ([card(11), card(10), card(14)], Seat::Hinterhand),
                ([card(11), card(13), card(9)], Seat::Mittelhand),
                ([card(11), card(13), card(12)], Seat::Mittelhand),
                ([card(11), card(13), card(14)], Seat::Hinterhand),
                ([card(11), card(15), card(9)], Seat::Mittelhand),
                ([card(11), card(15), card(12)], Seat::Mittelhand),
                ([card(11), card(15), card(14)], Seat::Mittelhand),
                ([card(17), card(18), card(16)], Seat::Mittelhand),
                ([card(17), card(18), card(19)], Seat::Hinterhand),
                ([card(17), card(18), card(21)], Seat::Hinterhand),
                ([card(17), card(18), card(23)], Seat::Hinterhand),
                ([card(17), card(20), card(16)], Seat::Mittelhand),
                ([card(17), card(20), card(19)], Seat::Mittelhand),
                ([card(17), card(20), card(21)], Seat::Hinterhand),
                ([card(17), card(20), card(23)], Seat::Hinterhand),
                ([card(17), card(22), card(16)], Seat::Mittelhand),
                ([card(17), card(22), card(19)], Seat::Mittelhand),
                ([card(17), card(22), card(21)], Seat::Mittelhand),
                ([card(17), card(22), card(23)], Seat::Hinterhand),
                ([card(26), card(24), card(25)], Seat::Vorhand),
                ([card(26), card(24), card(28)], Seat::Hinterhand),
                ([card(26), card(27), card(25)], Seat::Mittelhand),
                ([card(26), card(27), card(28)], Seat::Hinterhand),
                ([card(31), card(24), card(25)], Seat::Vorhand),
                ([card(31), card(24), card(28)], Seat::Vorhand),
                ([card(31), card(27), card(25)], Seat::Vorhand),
                ([card(31), card(27), card(28)], Seat::Vorhand),
            ],
        },
        Case {
            id: "A-hidden",
            deal: Deal::new(
                [
                    CardSet::from_bits(2214726045),
                    CardSet::from_bits(424977472),
                    CardSet::from_bits(44651042),
                ],
                CardSet::from_bits(1610612736),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(2214726045),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(true),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: None,
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(0)),
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(8)),
                    Action::Play(card(11)),
                    Action::Play(card(17)),
                    Action::Play(card(26)),
                    Action::Play(card(31)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(2214726045),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(0),
                skat: None,
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(0),
                branch: Branch::Null,
                winners: vec![
                    card(0),
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(8),
                    card(11),
                    card(17),
                    card(26),
                    card(31),
                ],
                losers: vec![],
                candidates: vec![
                    card(0),
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(8),
                    card(11),
                    card(17),
                    card(26),
                    card(31),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 0),
            },
            paths: vec![
                ([card(0), card(6), card(1)], Seat::Mittelhand),
                ([card(0), card(6), card(5)], Seat::Mittelhand),
                ([card(2), card(6), card(1)], Seat::Mittelhand),
                ([card(2), card(6), card(5)], Seat::Mittelhand),
                ([card(3), card(6), card(1)], Seat::Mittelhand),
                ([card(3), card(6), card(5)], Seat::Mittelhand),
                ([card(4), card(6), card(1)], Seat::Mittelhand),
                ([card(4), card(6), card(5)], Seat::Mittelhand),
                ([card(7), card(6), card(1)], Seat::Vorhand),
                ([card(7), card(6), card(5)], Seat::Vorhand),
                ([card(8), card(10), card(9)], Seat::Mittelhand),
                ([card(8), card(10), card(12)], Seat::Hinterhand),
                ([card(8), card(10), card(14)], Seat::Hinterhand),
                ([card(8), card(13), card(9)], Seat::Mittelhand),
                ([card(8), card(13), card(12)], Seat::Mittelhand),
                ([card(8), card(13), card(14)], Seat::Hinterhand),
                ([card(8), card(15), card(9)], Seat::Mittelhand),
                ([card(8), card(15), card(12)], Seat::Mittelhand),
                ([card(8), card(15), card(14)], Seat::Mittelhand),
                ([card(11), card(10), card(9)], Seat::Vorhand),
                ([card(11), card(10), card(12)], Seat::Hinterhand),
                ([card(11), card(10), card(14)], Seat::Hinterhand),
                ([card(11), card(13), card(9)], Seat::Mittelhand),
                ([card(11), card(13), card(12)], Seat::Mittelhand),
                ([card(11), card(13), card(14)], Seat::Hinterhand),
                ([card(11), card(15), card(9)], Seat::Mittelhand),
                ([card(11), card(15), card(12)], Seat::Mittelhand),
                ([card(11), card(15), card(14)], Seat::Mittelhand),
                ([card(17), card(18), card(16)], Seat::Mittelhand),
                ([card(17), card(18), card(19)], Seat::Hinterhand),
                ([card(17), card(18), card(21)], Seat::Hinterhand),
                ([card(17), card(18), card(23)], Seat::Hinterhand),
                ([card(17), card(20), card(16)], Seat::Mittelhand),
                ([card(17), card(20), card(19)], Seat::Mittelhand),
                ([card(17), card(20), card(21)], Seat::Hinterhand),
                ([card(17), card(20), card(23)], Seat::Hinterhand),
                ([card(17), card(22), card(16)], Seat::Mittelhand),
                ([card(17), card(22), card(19)], Seat::Mittelhand),
                ([card(17), card(22), card(21)], Seat::Mittelhand),
                ([card(17), card(22), card(23)], Seat::Hinterhand),
                ([card(26), card(24), card(25)], Seat::Vorhand),
                ([card(26), card(27), card(25)], Seat::Mittelhand),
                ([card(26), card(28), card(25)], Seat::Mittelhand),
                ([card(31), card(24), card(25)], Seat::Vorhand),
                ([card(31), card(27), card(25)], Seat::Vorhand),
                ([card(31), card(28), card(25)], Seat::Vorhand),
            ],
        },
        Case {
            id: "B-pickup-nine",
            deal: Deal::new(
                [
                    CardSet::from_bits(142875804),
                    CardSet::from_bits(307405089),
                    CardSet::from_bits(623460930),
                ],
                CardSet::from_bits(3221225472),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::TakeSkat,
                Action::Discard(CardSet::from_bits(3221225472)),
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(142875804),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(false),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: false,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: Some(CardSet::from_bits(3221225472)),
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(10)),
                    Action::Play(card(11)),
                    Action::Play(card(12)),
                    Action::Play(card(18)),
                    Action::Play(card(23)),
                    Action::Play(card(27)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(142875804),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(3221225472),
                skat: Some(CardSet::from_bits(3221225472)),
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(2),
                branch: Branch::Null,
                winners: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(10),
                    card(11),
                    card(12),
                    card(18),
                    card(23),
                    card(27),
                ],
                losers: vec![],
                candidates: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(10),
                    card(11),
                    card(12),
                    card(18),
                    card(23),
                    card(27),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 2),
            },
            paths: vec![
                ([card(2), card(0), card(1)], Seat::Vorhand),
                ([card(2), card(0), card(6)], Seat::Hinterhand),
                ([card(2), card(5), card(1)], Seat::Mittelhand),
                ([card(2), card(5), card(6)], Seat::Hinterhand),
                ([card(3), card(0), card(1)], Seat::Vorhand),
                ([card(3), card(0), card(6)], Seat::Hinterhand),
                ([card(3), card(5), card(1)], Seat::Mittelhand),
                ([card(3), card(5), card(6)], Seat::Hinterhand),
                ([card(4), card(0), card(1)], Seat::Vorhand),
                ([card(4), card(0), card(6)], Seat::Hinterhand),
                ([card(4), card(5), card(1)], Seat::Mittelhand),
                ([card(4), card(5), card(6)], Seat::Hinterhand),
                ([card(7), card(0), card(1)], Seat::Vorhand),
                ([card(7), card(0), card(6)], Seat::Vorhand),
                ([card(7), card(5), card(1)], Seat::Vorhand),
                ([card(7), card(5), card(6)], Seat::Vorhand),
                ([card(10), card(8), card(9)], Seat::Vorhand),
                ([card(10), card(8), card(14)], Seat::Hinterhand),
                ([card(10), card(13), card(9)], Seat::Mittelhand),
                ([card(10), card(13), card(14)], Seat::Hinterhand),
                ([card(10), card(15), card(9)], Seat::Mittelhand),
                ([card(10), card(15), card(14)], Seat::Mittelhand),
                ([card(11), card(8), card(9)], Seat::Vorhand),
                ([card(11), card(8), card(14)], Seat::Hinterhand),
                ([card(11), card(13), card(9)], Seat::Mittelhand),
                ([card(11), card(13), card(14)], Seat::Hinterhand),
                ([card(11), card(15), card(9)], Seat::Mittelhand),
                ([card(11), card(15), card(14)], Seat::Mittelhand),
                ([card(12), card(8), card(9)], Seat::Vorhand),
                ([card(12), card(8), card(14)], Seat::Hinterhand),
                ([card(12), card(13), card(9)], Seat::Mittelhand),
                ([card(12), card(13), card(14)], Seat::Hinterhand),
                ([card(12), card(15), card(9)], Seat::Mittelhand),
                ([card(12), card(15), card(14)], Seat::Mittelhand),
                ([card(18), card(17), card(16)], Seat::Vorhand),
                ([card(18), card(17), card(19)], Seat::Hinterhand),
                ([card(18), card(17), card(21)], Seat::Hinterhand),
                ([card(18), card(20), card(16)], Seat::Mittelhand),
                ([card(18), card(20), card(19)], Seat::Mittelhand),
                ([card(18), card(20), card(21)], Seat::Hinterhand),
                ([card(18), card(22), card(16)], Seat::Mittelhand),
                ([card(18), card(22), card(19)], Seat::Mittelhand),
                ([card(18), card(22), card(21)], Seat::Mittelhand),
                ([card(23), card(17), card(16)], Seat::Vorhand),
                ([card(23), card(17), card(19)], Seat::Vorhand),
                ([card(23), card(17), card(21)], Seat::Vorhand),
                ([card(23), card(20), card(16)], Seat::Vorhand),
                ([card(23), card(20), card(19)], Seat::Vorhand),
                ([card(23), card(20), card(21)], Seat::Vorhand),
                ([card(23), card(22), card(16)], Seat::Vorhand),
                ([card(23), card(22), card(19)], Seat::Vorhand),
                ([card(23), card(22), card(21)], Seat::Vorhand),
                ([card(27), card(25), card(24)], Seat::Vorhand),
                ([card(27), card(25), card(26)], Seat::Vorhand),
                ([card(27), card(25), card(29)], Seat::Hinterhand),
                ([card(27), card(28), card(24)], Seat::Mittelhand),
                ([card(27), card(28), card(26)], Seat::Mittelhand),
                ([card(27), card(28), card(29)], Seat::Hinterhand),
            ],
        },
        Case {
            id: "B-hidden",
            deal: Deal::new(
                [
                    CardSet::from_bits(142875804),
                    CardSet::from_bits(844276000),
                    CardSet::from_bits(86590019),
                ],
                CardSet::from_bits(3221225472),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::TakeSkat,
                Action::Discard(CardSet::from_bits(3221225472)),
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(142875804),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(false),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: false,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: Some(CardSet::from_bits(3221225472)),
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(10)),
                    Action::Play(card(11)),
                    Action::Play(card(12)),
                    Action::Play(card(18)),
                    Action::Play(card(23)),
                    Action::Play(card(27)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(142875804),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(3221225472),
                skat: Some(CardSet::from_bits(3221225472)),
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(2),
                branch: Branch::Null,
                winners: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(10),
                    card(11),
                    card(12),
                    card(18),
                    card(23),
                    card(27),
                ],
                losers: vec![],
                candidates: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(10),
                    card(11),
                    card(12),
                    card(18),
                    card(23),
                    card(27),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 2),
            },
            paths: vec![
                ([card(2), card(5), card(0)], Seat::Mittelhand),
                ([card(2), card(5), card(1)], Seat::Mittelhand),
                ([card(2), card(5), card(6)], Seat::Hinterhand),
                ([card(3), card(5), card(0)], Seat::Mittelhand),
                ([card(3), card(5), card(1)], Seat::Mittelhand),
                ([card(3), card(5), card(6)], Seat::Hinterhand),
                ([card(4), card(5), card(0)], Seat::Mittelhand),
                ([card(4), card(5), card(1)], Seat::Mittelhand),
                ([card(4), card(5), card(6)], Seat::Hinterhand),
                ([card(7), card(5), card(0)], Seat::Vorhand),
                ([card(7), card(5), card(1)], Seat::Vorhand),
                ([card(7), card(5), card(6)], Seat::Vorhand),
                ([card(10), card(8), card(9)], Seat::Vorhand),
                ([card(10), card(8), card(14)], Seat::Hinterhand),
                ([card(10), card(13), card(9)], Seat::Mittelhand),
                ([card(10), card(13), card(14)], Seat::Hinterhand),
                ([card(10), card(15), card(9)], Seat::Mittelhand),
                ([card(10), card(15), card(14)], Seat::Mittelhand),
                ([card(11), card(8), card(9)], Seat::Vorhand),
                ([card(11), card(8), card(14)], Seat::Hinterhand),
                ([card(11), card(13), card(9)], Seat::Mittelhand),
                ([card(11), card(13), card(14)], Seat::Hinterhand),
                ([card(11), card(15), card(9)], Seat::Mittelhand),
                ([card(11), card(15), card(14)], Seat::Mittelhand),
                ([card(12), card(8), card(9)], Seat::Vorhand),
                ([card(12), card(8), card(14)], Seat::Hinterhand),
                ([card(12), card(13), card(9)], Seat::Mittelhand),
                ([card(12), card(13), card(14)], Seat::Hinterhand),
                ([card(12), card(15), card(9)], Seat::Mittelhand),
                ([card(12), card(15), card(14)], Seat::Mittelhand),
                ([card(18), card(17), card(16)], Seat::Vorhand),
                ([card(18), card(17), card(19)], Seat::Hinterhand),
                ([card(18), card(17), card(21)], Seat::Hinterhand),
                ([card(18), card(20), card(16)], Seat::Mittelhand),
                ([card(18), card(20), card(19)], Seat::Mittelhand),
                ([card(18), card(20), card(21)], Seat::Hinterhand),
                ([card(18), card(22), card(16)], Seat::Mittelhand),
                ([card(18), card(22), card(19)], Seat::Mittelhand),
                ([card(18), card(22), card(21)], Seat::Mittelhand),
                ([card(23), card(17), card(16)], Seat::Vorhand),
                ([card(23), card(17), card(19)], Seat::Vorhand),
                ([card(23), card(17), card(21)], Seat::Vorhand),
                ([card(23), card(20), card(16)], Seat::Vorhand),
                ([card(23), card(20), card(19)], Seat::Vorhand),
                ([card(23), card(20), card(21)], Seat::Vorhand),
                ([card(23), card(22), card(16)], Seat::Vorhand),
                ([card(23), card(22), card(19)], Seat::Vorhand),
                ([card(23), card(22), card(21)], Seat::Vorhand),
                ([card(27), card(25), card(24)], Seat::Vorhand),
                ([card(27), card(25), card(26)], Seat::Vorhand),
                ([card(27), card(28), card(24)], Seat::Mittelhand),
                ([card(27), card(28), card(26)], Seat::Mittelhand),
                ([card(27), card(29), card(24)], Seat::Mittelhand),
                ([card(27), card(29), card(26)], Seat::Mittelhand),
            ],
        },
        Case {
            id: "C-hand-bube",
            deal: Deal::new(
                [
                    CardSet::from_bits(63736),
                    CardSet::from_bits(357892613),
                    CardSet::from_bits(715785474),
                ],
                CardSet::from_bits(3221225472),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(63736),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(true),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: None,
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(5)),
                    Action::Play(card(6)),
                    Action::Play(card(7)),
                    Action::Play(card(11)),
                    Action::Play(card(12)),
                    Action::Play(card(13)),
                    Action::Play(card(14)),
                    Action::Play(card(15)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(63736),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(0),
                skat: None,
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(4),
                branch: Branch::Null,
                winners: vec![
                    card(3),
                    card(4),
                    card(5),
                    card(6),
                    card(7),
                    card(11),
                    card(12),
                    card(13),
                    card(14),
                    card(15),
                ],
                losers: vec![],
                candidates: vec![
                    card(3),
                    card(4),
                    card(5),
                    card(6),
                    card(7),
                    card(11),
                    card(12),
                    card(13),
                    card(14),
                    card(15),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 2, 4),
            },
            paths: vec![
                ([card(3), card(0), card(1)], Seat::Vorhand),
                ([card(3), card(2), card(1)], Seat::Vorhand),
                ([card(4), card(0), card(1)], Seat::Vorhand),
                ([card(4), card(2), card(1)], Seat::Vorhand),
                ([card(5), card(0), card(1)], Seat::Vorhand),
                ([card(5), card(2), card(1)], Seat::Vorhand),
                ([card(6), card(0), card(1)], Seat::Vorhand),
                ([card(6), card(2), card(1)], Seat::Vorhand),
                ([card(7), card(0), card(1)], Seat::Vorhand),
                ([card(7), card(2), card(1)], Seat::Vorhand),
                ([card(11), card(9), card(8)], Seat::Vorhand),
                ([card(11), card(9), card(10)], Seat::Vorhand),
                ([card(12), card(9), card(8)], Seat::Vorhand),
                ([card(12), card(9), card(10)], Seat::Vorhand),
                ([card(13), card(9), card(8)], Seat::Vorhand),
                ([card(13), card(9), card(10)], Seat::Vorhand),
                ([card(14), card(9), card(8)], Seat::Vorhand),
                ([card(14), card(9), card(10)], Seat::Vorhand),
                ([card(15), card(9), card(8)], Seat::Vorhand),
                ([card(15), card(9), card(10)], Seat::Vorhand),
            ],
        },
        Case {
            id: "D-pickup-nine",
            deal: Deal::new(
                [
                    CardSet::from_bits(404232348),
                    CardSet::from_bits(88425761),
                    CardSet::from_bits(581083714),
                ],
                CardSet::from_bits(3221225472),
            )
            .unwrap(),
            history: vec![
                Action::Pass,
                Action::Pass,
                Action::Bid(18),
                Action::TakeSkat,
                Action::Discard(CardSet::from_bits(3221225472)),
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
            ],
            view: PlayerView {
                seat: Seat::Vorhand,
                config: RuleConfig { kontra_re: false },
                phase: Phase::Spiel,
                current_seat: Some(Seat::Vorhand),
                hand: CardSet::from_bits(404232348),
                hand_sizes: [10, 10, 10],
                bidding: vec![
                    BidRecord {
                        seat: Seat::Mittelhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Hinterhand,
                        call: Call::Pass,
                    },
                    BidRecord {
                        seat: Seat::Vorhand,
                        call: Call::Bid(18),
                    },
                ],
                bid: 18,
                declarer: Some(Seat::Vorhand),
                hand_game: Some(false),
                contract: Some(Contract {
                    game: GameType::Null,
                    hand: false,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                own_skat: Some(CardSet::from_bits(3221225472)),
                declarer_cards: None,
                tricks: vec![],
                current_trick: Trick {
                    leader: None,
                    cards: [None, None, None],
                },
                kontra: KontraLevel::None,
                legal_actions: vec![
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(11)),
                    Action::Play(card(12)),
                    Action::Play(card(19)),
                    Action::Play(card(20)),
                    Action::Play(card(27)),
                    Action::Play(card(28)),
                ],
                result: None,
            },
            knowledge: Knowledge {
                seat: Seat::Vorhand,
                game: Some(GameType::Null),
                declarer: Some(Seat::Vorhand),
                own: CardSet::from_bits(404232348),
                played: CardSet::from_bits(0),
                known_elsewhere: CardSet::from_bits(3221225472),
                skat: Some(CardSet::from_bits(3221225472)),
                voids: [
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                    CardSet::from_bits(0),
                ],
                hand_sizes: [10, 10, 10],
                augen_won: [0, 0, 0],
            },
            expected: CardRule {
                card: card(2),
                branch: Branch::Null,
                winners: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(11),
                    card(12),
                    card(19),
                    card(20),
                    card(27),
                    card(28),
                ],
                losers: vec![],
                candidates: vec![
                    card(2),
                    card(3),
                    card(4),
                    card(7),
                    card(11),
                    card(12),
                    card(19),
                    card(20),
                    card(27),
                    card(28),
                ],
                winner: None,
                partner_ahead: false,
                last: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 2),
            },
            paths: vec![
                ([card(2), card(0), card(1)], Seat::Vorhand),
                ([card(2), card(0), card(6)], Seat::Hinterhand),
                ([card(2), card(5), card(1)], Seat::Mittelhand),
                ([card(2), card(5), card(6)], Seat::Hinterhand),
                ([card(3), card(0), card(1)], Seat::Vorhand),
                ([card(3), card(0), card(6)], Seat::Hinterhand),
                ([card(3), card(5), card(1)], Seat::Mittelhand),
                ([card(3), card(5), card(6)], Seat::Hinterhand),
                ([card(4), card(0), card(1)], Seat::Vorhand),
                ([card(4), card(0), card(6)], Seat::Hinterhand),
                ([card(4), card(5), card(1)], Seat::Mittelhand),
                ([card(4), card(5), card(6)], Seat::Hinterhand),
                ([card(7), card(0), card(1)], Seat::Vorhand),
                ([card(7), card(0), card(6)], Seat::Vorhand),
                ([card(7), card(5), card(1)], Seat::Vorhand),
                ([card(7), card(5), card(6)], Seat::Vorhand),
                ([card(11), card(8), card(9)], Seat::Vorhand),
                ([card(11), card(8), card(13)], Seat::Hinterhand),
                ([card(11), card(8), card(15)], Seat::Hinterhand),
                ([card(11), card(10), card(9)], Seat::Vorhand),
                ([card(11), card(10), card(13)], Seat::Hinterhand),
                ([card(11), card(10), card(15)], Seat::Hinterhand),
                ([card(11), card(14), card(9)], Seat::Mittelhand),
                ([card(11), card(14), card(13)], Seat::Mittelhand),
                ([card(11), card(14), card(15)], Seat::Hinterhand),
                ([card(12), card(8), card(9)], Seat::Vorhand),
                ([card(12), card(8), card(13)], Seat::Hinterhand),
                ([card(12), card(8), card(15)], Seat::Hinterhand),
                ([card(12), card(10), card(9)], Seat::Vorhand),
                ([card(12), card(10), card(13)], Seat::Hinterhand),
                ([card(12), card(10), card(15)], Seat::Hinterhand),
                ([card(12), card(14), card(9)], Seat::Mittelhand),
                ([card(12), card(14), card(13)], Seat::Mittelhand),
                ([card(12), card(14), card(15)], Seat::Hinterhand),
                ([card(19), card(16), card(17)], Seat::Vorhand),
                ([card(19), card(16), card(21)], Seat::Hinterhand),
                ([card(19), card(16), card(23)], Seat::Hinterhand),
                ([card(19), card(18), card(17)], Seat::Vorhand),
                ([card(19), card(18), card(21)], Seat::Hinterhand),
                ([card(19), card(18), card(23)], Seat::Hinterhand),
                ([card(19), card(22), card(17)], Seat::Mittelhand),
                ([card(19), card(22), card(21)], Seat::Mittelhand),
                ([card(19), card(22), card(23)], Seat::Hinterhand),
                ([card(20), card(16), card(17)], Seat::Vorhand),
                ([card(20), card(16), card(21)], Seat::Hinterhand),
                ([card(20), card(16), card(23)], Seat::Hinterhand),
                ([card(20), card(18), card(17)], Seat::Vorhand),
                ([card(20), card(18), card(21)], Seat::Hinterhand),
                ([card(20), card(18), card(23)], Seat::Hinterhand),
                ([card(20), card(22), card(17)], Seat::Mittelhand),
                ([card(20), card(22), card(21)], Seat::Mittelhand),
                ([card(20), card(22), card(23)], Seat::Hinterhand),
                ([card(27), card(24), card(25)], Seat::Vorhand),
                ([card(27), card(24), card(29)], Seat::Hinterhand),
                ([card(27), card(26), card(25)], Seat::Vorhand),
                ([card(27), card(26), card(29)], Seat::Hinterhand),
                ([card(28), card(24), card(25)], Seat::Vorhand),
                ([card(28), card(24), card(29)], Seat::Hinterhand),
                ([card(28), card(26), card(25)], Seat::Vorhand),
                ([card(28), card(26), card(29)], Seat::Hinterhand),
            ],
        },
    ]
}
pub(crate) fn boundaries() -> Vec<(&'static str, PlayerView, bool)> {
    let base = cases().remove(0).view;
    vec![
        (
            "single-action",
            {
                let mut v = base.clone();
                v.legal_actions = vec![Action::Play(card(0))];
                v
            },
            true,
        ),
        (
            "restricted",
            {
                let mut v = base.clone();
                v.legal_actions = vec![Action::Play(card(0)), Action::Play(card(2))];
                v
            },
            true,
        ),
        (
            "reordered",
            {
                let mut v = base.clone();
                v.legal_actions = vec![
                    Action::Play(card(31)),
                    Action::Play(card(26)),
                    Action::Play(card(17)),
                    Action::Play(card(11)),
                    Action::Play(card(8)),
                    Action::Play(card(7)),
                    Action::Play(card(4)),
                    Action::Play(card(3)),
                    Action::Play(card(2)),
                    Action::Play(card(0)),
                ];
                v
            },
            true,
        ),
        (
            "mixed",
            {
                let mut v = base.clone();
                v.legal_actions = vec![
                    Action::Play(card(0)),
                    Action::Play(card(2)),
                    Action::Play(card(3)),
                    Action::Play(card(4)),
                    Action::Play(card(7)),
                    Action::Play(card(8)),
                    Action::Play(card(11)),
                    Action::Play(card(17)),
                    Action::Play(card(26)),
                    Action::Play(card(31)),
                    Action::Kontra,
                ];
                v
            },
            false,
        ),
        (
            "empty-actions",
            {
                let mut v = base.clone();
                v.legal_actions = vec![];
                v
            },
            false,
        ),
        (
            "wrongturn",
            {
                let mut v = base.clone();
                v.current_seat = Some(Seat::Mittelhand);
                v
            },
            true,
        ),
        (
            "wrongleader",
            {
                let mut v = base.clone();
                v.current_trick = Trick {
                    leader: Some(Seat::Hinterhand),
                    cards: [None, None, None],
                };
                v
            },
            true,
        ),
        (
            "malformed",
            {
                let mut v = base.clone();
                v.current_trick = Trick {
                    leader: None,
                    cards: [None, Some(card(1)), None],
                };
                v
            },
            true,
        ),
        (
            "nonempty",
            {
                let mut v = base.clone();
                v.current_trick = Trick {
                    leader: Some(Seat::Hinterhand),
                    cards: [Some(card(1)), None, None],
                };
                v
            },
            true,
        ),
        (
            "hand-with-skat",
            {
                let mut v = base.clone();
                v.own_skat = Some(CardSet::from_bits(1610612736));
                v
            },
            true,
        ),
        (
            "pickup-without-skat",
            {
                let mut v = base.clone();
                v.hand_game = Some(false);
                v.contract = Some(Contract {
                    game: GameType::Null,
                    hand: false,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                });
                v.own_skat = None;
                v
            },
            true,
        ),
        (
            "defender",
            {
                let mut v = base.clone();
                v.declarer = Some(Seat::Mittelhand);
                v
            },
            true,
        ),
        (
            "Grand",
            {
                let mut v = base.clone();
                v.contract = Some(Contract {
                    game: GameType::Grand,
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                });
                v
            },
            true,
        ),
        (
            "Farbe",
            {
                let mut v = base.clone();
                v.contract = Some(Contract {
                    game: GameType::Farbe(Suit::Herz),
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                });
                v
            },
            true,
        ),
        (
            "Ouvert",
            {
                let mut v = base.clone();
                v.contract = Some(Contract {
                    game: GameType::Null,
                    hand: true,
                    schneider: false,
                    schwarz: false,
                    ouvert: true,
                });
                v
            },
            true,
        ),
        (
            "wrongphase",
            {
                let mut v = base.clone();
                v.phase = Phase::Ansage;
                v
            },
            true,
        ),
        (
            "MH-otherlead",
            {
                let mut v = base.clone();
                v.seat = Seat::Mittelhand;
                v.declarer = Some(Seat::Mittelhand);
                v.current_seat = Some(Seat::Vorhand);
                v.legal_actions = vec![];
                v
            },
            false,
        ),
        (
            "HH-otherlead",
            {
                let mut v = base.clone();
                v.seat = Seat::Hinterhand;
                v.declarer = Some(Seat::Hinterhand);
                v.current_seat = Some(Seat::Vorhand);
                v.legal_actions = vec![];
                v
            },
            false,
        ),
    ]
}

#[test]
fn null_lead_full_literals_and_all_286_legal_paths() {
    let mut counts = [0; 3];
    for c in cases() {
        let g = c.game();
        let v = g.view(Seat::Vorhand);
        assert_eq!(v, c.view, "{} full view", c.id);
        assert_eq!(
            Knowledge::from_view(&v),
            c.knowledge,
            "{} full Knowledge",
            c.id
        );
        assert_eq!(
            baseline::rule(&v),
            Some(c.expected),
            "{} all eleven fields",
            c.id
        );
        let mut actual = Vec::new();
        for a in g.legal_actions() {
            let mut x = g.clone();
            x.apply(a).unwrap();
            for b in x.legal_actions() {
                let mut y = x.clone();
                y.apply(b).unwrap();
                for d in y.legal_actions() {
                    let mut z = y.clone();
                    z.apply(d).unwrap();
                    let cards =
                        [a, b, d].map(|a| if let Action::Play(c) = a { c } else { panic!() });
                    let winner = z.tricks()[0].winner;
                    counts[winner.index()] += 1;
                    assert_eq!(z.phase() == Phase::Beendet, winner == Seat::Vorhand);
                    if winner == Seat::Vorhand {
                        assert!(!z.result().unwrap().won);
                        assert!(z.legal_actions().is_empty());
                        assert_eq!(z.current_seat(), None);
                    } else {
                        assert_eq!(z.current_seat(), Some(winner));
                        assert!(z.result().is_none());
                    }
                    actual.push((cards, winner));
                }
            }
        }
        assert_eq!(actual, c.paths, "{} full next-trick set/order", c.id);
    }
    assert_eq!(counts, [94, 103, 89]);
}
#[test]
fn null_lead_two_complete_hidden_view_pairs() {
    let cs = cases();
    for (a, b) in [(0, 1), (2, 3)] {
        assert_eq!(cs[a].view, cs[b].view);
        assert_eq!(cs[a].knowledge, cs[b].knowledge);
        assert_eq!(cs[a].expected, cs[b].expected);
    }
}
#[test]
fn null_lead_named_actual_shared_api_boundaries() {
    for (id, v, present) in boundaries() {
        assert_eq!(
            baseline::rule(&v).is_some(),
            present,
            "{id}: shared API is broader than new admission"
        );
    }
}
