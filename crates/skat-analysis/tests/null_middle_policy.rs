// SPDX-License-Identifier: EUPL-1.2

//! Cloud-authored #169 policy regression from QA1591's sealed handwritten oracle.
//! Original SHA256 c332c4e5a5255141244e811b91e84a13dec4732e8b721c952489d2583a4ec4bd.
//! Includes oracle-extra and the explicit changed-lead correction from the
//! addendum in BOTH completion representations: C8/C7/C9 is won by HH.
//! Local attempts1592/1595 were not accepted. No serde or external job paths.

use skat_analysis::baseline::{self, Branch, CardRule, TieBreak};
use skat_core::{Action, Announcement, Card, CardSet, Deal, Game, GameType, RuleConfig, Seat};

pub(crate) fn card(n: u8) -> Card {
    Card::from_index(n).unwrap()
}
pub(crate) fn set(ns: &[u8]) -> CardSet {
    ns.iter().copied().map(card).collect()
}

pub(crate) struct Case {
    pub(crate) id: &'static str,
    pub(crate) deal: Deal,
    pub(crate) config: RuleConfig,
    pub(crate) history: Vec<Action>,
    pub(crate) seat: Seat,
    pub(crate) admitted: bool,
    pub(crate) legal: Vec<Card>,
    pub(crate) expected: Option<CardRule>,
    pub(crate) completions: Vec<(u8, u8, Seat)>,
}
impl Case {
    pub(crate) fn game(&self) -> Game {
        let mut g = Game::from_deal(self.deal, self.config);
        for &a in &self.history {
            g.apply(a)
                .unwrap_or_else(|e| panic!("{}: {a:?}: {e:?}", self.id));
        }
        g
    }
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mixed-hand",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "mixed-pickup",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::TakeSkat,
                Action::Discard(set(&[30, 31])),
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "changed-leading-rank",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(1)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(0),
                branch: Branch::Null,
                candidates: vec![card(0)],
                winners: vec![card(3), card(4), card(6)],
                losers: vec![card(0)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (0, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Hinterhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Mittelhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Mittelhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "all-loser-follow",
            deal: Deal::new(
                [
                    set(&[1, 7, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 5, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(7)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(6),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4), card(6)],
                winners: vec![],
                losers: vec![card(0), card(3), card(4), card(6)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: true,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (6, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 5, Seat::Vorhand),
                (3, 2, Seat::Vorhand),
                (3, 5, Seat::Vorhand),
                (4, 2, Seat::Vorhand),
                (4, 5, Seat::Vorhand),
                (6, 2, Seat::Vorhand),
                (6, 5, Seat::Vorhand),
            ],
        },
        Case {
            id: "void-last-exact-tie",
            deal: Deal::new(
                [
                    set(&[0, 1, 2, 3, 4, 5, 10, 11, 17, 18]),
                    set(&[8, 9, 12, 15, 16, 20, 23, 24, 28, 31]),
                    set(&[6, 7, 13, 14, 19, 21, 22, 25, 26, 27]),
                ],
                set(&[29, 30]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![
                card(8),
                card(9),
                card(12),
                card(15),
                card(16),
                card(20),
                card(23),
                card(24),
                card(28),
                card(31),
            ],
            expected: Some(CardRule {
                card: card(31),
                branch: Branch::Null,
                candidates: vec![
                    card(8),
                    card(9),
                    card(12),
                    card(15),
                    card(16),
                    card(20),
                    card(23),
                    card(24),
                    card(28),
                    card(31),
                ],
                winners: vec![],
                losers: vec![
                    card(8),
                    card(9),
                    card(12),
                    card(15),
                    card(16),
                    card(20),
                    card(23),
                    card(24),
                    card(28),
                    card(31),
                ],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (7, 0, 0),
            }),
            completions: vec![
                (8, 6, Seat::Hinterhand),
                (8, 7, Seat::Hinterhand),
                (9, 6, Seat::Hinterhand),
                (9, 7, Seat::Hinterhand),
                (12, 6, Seat::Hinterhand),
                (12, 7, Seat::Hinterhand),
                (15, 6, Seat::Hinterhand),
                (15, 7, Seat::Hinterhand),
                (16, 6, Seat::Hinterhand),
                (16, 7, Seat::Hinterhand),
                (20, 6, Seat::Hinterhand),
                (20, 7, Seat::Hinterhand),
                (23, 6, Seat::Hinterhand),
                (23, 7, Seat::Hinterhand),
                (24, 6, Seat::Hinterhand),
                (24, 7, Seat::Hinterhand),
                (28, 6, Seat::Hinterhand),
                (28, 7, Seat::Hinterhand),
                (31, 6, Seat::Hinterhand),
                (31, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "no-loser",
            deal: Deal::new(
                [
                    set(&[0, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[3, 4, 6, 7, 8, 9, 16, 17, 24, 25]),
                    set(&[1, 2, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(0)),
            ],
            seat: Seat::Mittelhand,
            admitted: false,
            legal: vec![card(3), card(4), card(6), card(7)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(3), card(4), card(6), card(7)],
                winners: vec![card(3), card(4), card(6), card(7)],
                losers: vec![],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 2, 4),
            }),
            completions: vec![
                (3, 1, Seat::Mittelhand),
                (3, 2, Seat::Mittelhand),
                (4, 1, Seat::Mittelhand),
                (4, 2, Seat::Mittelhand),
                (6, 1, Seat::Mittelhand),
                (6, 2, Seat::Mittelhand),
                (7, 1, Seat::Mittelhand),
                (7, 2, Seat::Mittelhand),
            ],
        },
        Case {
            id: "single-legal",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 8, 9, 14, 16, 17, 22, 24, 25, 29]),
                    set(&[2, 3, 4, 6, 7, 15, 20, 21, 23, 28]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: false,
            legal: vec![card(0)],
            expected: Some(CardRule {
                card: card(0),
                branch: Branch::Null,
                candidates: vec![card(0)],
                winners: vec![],
                losers: vec![card(0)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (0, 0, 0),
            }),
            completions: vec![(0, 2, Seat::Vorhand), (0, 7, Seat::Hinterhand)],
        },
        Case {
            id: "ouvert",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: true,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: false,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "lead",
            deal: Deal::new(
                [
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
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
            seat: Seat::Vorhand,
            admitted: false,
            legal: vec![
                card(0),
                card(3),
                card(4),
                card(6),
                card(8),
                card(9),
                card(16),
                card(17),
                card(24),
                card(25),
            ],
            expected: Some(CardRule {
                card: card(0),
                branch: Branch::Null,
                candidates: vec![
                    card(0),
                    card(3),
                    card(4),
                    card(6),
                    card(8),
                    card(9),
                    card(16),
                    card(17),
                    card(24),
                    card(25),
                ],
                winners: vec![
                    card(0),
                    card(3),
                    card(4),
                    card(6),
                    card(8),
                    card(9),
                    card(16),
                    card(17),
                    card(24),
                    card(25),
                ],
                losers: vec![],
                winner: None,
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::LowestCostFirst,
                tie_key: (0, 0, 0),
            }),
            completions: vec![],
        },
        Case {
            id: "last",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Bid(20),
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
                Action::Play(card(0)),
            ],
            seat: Seat::Hinterhand,
            admitted: false,
            legal: vec![card(2), card(7)],
            expected: Some(CardRule {
                card: card(2),
                branch: Branch::Null,
                candidates: vec![card(2)],
                winners: vec![card(7)],
                losers: vec![card(2)],
                winner: Some(Seat::Vorhand),
                last: true,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (2, 0, 0),
            }),
            completions: vec![],
        },
        Case {
            id: "defender-mixed-call",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 14, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: true },
            history: vec![
                Action::Bid(18),
                Action::Hold,
                Action::Pass,
                Action::Bid(20),
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: false,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: None,
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "privacy-opponent-opponent",
            deal: Deal::new(
                [
                    set(&[1, 5, 11, 12, 13, 14, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 10, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "privacy-opponent-unknown-Hand-Skat",
            deal: Deal::new(
                [
                    set(&[1, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 15, 20, 21, 22, 23, 28, 29, 30]),
                ],
                set(&[14, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::Hand,
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "privacy-pickup-opponents",
            deal: Deal::new(
                [
                    set(&[1, 5, 11, 12, 13, 14, 18, 19, 26, 27]),
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 10, 15, 20, 21, 22, 23, 28, 29]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
            history: vec![
                Action::Bid(18),
                Action::Pass,
                Action::Pass,
                Action::TakeSkat,
                Action::Discard(set(&[30, 31])),
                Action::Announce(Announcement {
                    game: GameType::Null,
                    schneider: false,
                    schwarz: false,
                    ouvert: false,
                }),
                Action::Play(card(5)),
            ],
            seat: Seat::Mittelhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Vorhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Vorhand),
                (0, 7, Seat::Hinterhand),
                (3, 2, Seat::Vorhand),
                (3, 7, Seat::Hinterhand),
                (4, 2, Seat::Vorhand),
                (4, 7, Seat::Hinterhand),
                (6, 2, Seat::Mittelhand),
                (6, 7, Seat::Hinterhand),
            ],
        },
        Case {
            id: "current-middle-after-leader-change",
            deal: Deal::new(
                [
                    set(&[0, 3, 4, 6, 8, 9, 16, 17, 24, 25]),
                    set(&[2, 7, 13, 14, 20, 21, 22, 23, 28, 29]),
                    set(&[1, 5, 10, 11, 12, 15, 18, 19, 26, 27]),
                ],
                set(&[30, 31]),
            )
            .unwrap(),
            config: RuleConfig { kontra_re: false },
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
                Action::Play(card(8)),
                Action::Play(card(14)),
                Action::Play(card(15)),
                Action::Play(card(5)),
            ],
            seat: Seat::Vorhand,
            admitted: true,
            legal: vec![card(0), card(3), card(4), card(6)],
            expected: Some(CardRule {
                card: card(4),
                branch: Branch::Null,
                candidates: vec![card(0), card(3), card(4)],
                winners: vec![card(6)],
                losers: vec![card(0), card(3), card(4)],
                winner: Some(Seat::Hinterhand),
                last: false,
                partner_ahead: false,
                table_master: false,
                tie_break: TieBreak::HighestPowerLast,
                tie_key: (4, 0, 0),
            }),
            completions: vec![
                (0, 2, Seat::Hinterhand),
                (0, 7, Seat::Mittelhand),
                (3, 2, Seat::Hinterhand),
                (3, 7, Seat::Mittelhand),
                (4, 2, Seat::Hinterhand),
                (4, 7, Seat::Mittelhand),
                (6, 2, Seat::Vorhand),
                (6, 7, Seat::Mittelhand),
            ],
        },
    ]
}

#[test]
fn sealed_null_middle_policy_and_every_legal_completion() {
    let mut admitted = 0;
    let mut completions = 0;
    for c in cases() {
        let g = c.game();
        let v = g.view(c.seat);
        assert_eq!(
            v.legal_actions
                .iter()
                .filter_map(|a| if let Action::Play(c) = a {
                    Some(*c)
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            c.legal,
            "{}",
            c.id
        );
        assert_eq!(baseline::rule(&v), c.expected, "{}", c.id);
        if let Some(r) = &c.expected {
            assert_eq!(baseline::play(&v), Action::Play(r.card), "{}", c.id);
        }
        if c.admitted {
            admitted += 1;
        }
        for &(candidate, third, winner) in &c.completions {
            let mut g = c.game();
            let index = g.view(c.seat).tricks.len();
            g.apply(Action::Play(card(candidate))).unwrap();
            g.apply(Action::Play(card(third))).unwrap();
            let v = g.view(c.seat);
            let t = v.tricks[index];
            assert_eq!(t.winner, winner, "{} / {candidate} / {third}", c.id);
            assert_eq!(t.cards[1..], [card(candidate), card(third)]);
            if c.admitted {
                completions += 1;
                if c.expected
                    .as_ref()
                    .unwrap()
                    .losers
                    .contains(&card(candidate))
                {
                    assert_ne!(winner, c.seat);
                }
            }
        }
    }
    assert_eq!((admitted, completions), (9, 84));
}

#[test]
fn legal_order_last_equal_power_tie_is_frozen_not_sorted_by_the_coach() {
    let c = cases()
        .into_iter()
        .find(|c| c.id == "void-last-exact-tie")
        .unwrap();
    let mut v = c.game().view(c.seat);
    assert_eq!(baseline::rule(&v).unwrap().card, card(31));
    v.legal_actions.reverse();
    assert_eq!(baseline::rule(&v).unwrap().card, card(15));
}
