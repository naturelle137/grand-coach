// SPDX-License-Identifier: EUPL-1.2

//! Grand declarer `DrawTrumpf` own-lead witnesses (#175).
//!
//! Every deal, history and expectation below is handwritten from the ISkO
//! Grand rules (only the four Buben are Trumpf, order C > S > H > D; follow
//! suit otherwise, order A 10 K Q 9 8 7) and from the issue's admission
//! text, not copied from another family's fixtures. Card indices are
//! `suit * 8 + rank` with Kreuz 0, Pik 1, Herz 2, Karo 3 and the rank order
//! 7 8 9 10 J Q K A = 0..7, so the Buben are CJ 4, SJ 12, HJ 20, DJ 28.
//! The trick winner is computed by `winner` below from those rules only.
//!
//! Limits: the cases are a bounded set of legal deals; they say nothing about
//! unseen holders, replies or game results, and the exhaustive continuation
//! counts cover only the listed next tricks.

use skat_analysis::Knowledge;
use skat_analysis::baseline::{self, Branch, CardRule, TieBreak};
use skat_core::{Action, Announcement, Card, CardSet, Deal, Game, GameType, RuleConfig, Seat};

fn card(n: u8) -> Card {
    Card::from_index(n).unwrap()
}
fn set(ns: &[u8]) -> CardSet {
    ns.iter().copied().map(card).collect()
}

/// Trump power in Grand for the independent witness: Bube of suit s -> 4 + (3 - s).
fn rank_key(n: u8) -> i32 {
    let (suit, rank) = (i32::from(n / 8), i32::from(n % 8));
    if rank == 4 {
        100 + (3 - suit) // CJ > SJ > HJ > DJ
    } else {
        // 7 8 9 10 J Q K A -> order 7,8,9,Q,K,10,A with J excluded.
        [0, 1, 2, 5, 0, 3, 4, 6][rank as usize]
    }
}
/// Independent trick winner: highest Bube, otherwise highest card of the led suit.
fn winner(leader: Seat, cards: [u8; 3]) -> Seat {
    let is_bube = |n: u8| n % 8 == 4;
    let best = if cards.iter().any(|&n| is_bube(n)) {
        (0..3)
            .filter(|&i| is_bube(cards[i]))
            .max_by_key(|&i| rank_key(cards[i]))
            .unwrap()
    } else {
        let led = cards[0] / 8;
        (0..3)
            .filter(|&i| cards[i] / 8 == led)
            .max_by_key(|&i| rank_key(cards[i]))
            .unwrap()
    };
    Seat::from_index(leader.index() + best)
}

pub(crate) struct Case {
    pub(crate) id: &'static str,
    pub(crate) declarer: Seat,
    /// Declarer hand (10), Mittelhand/Vorhand-order helper hands and Skat.
    pub(crate) deal: [Vec<u8>; 3],
    pub(crate) skat: [u8; 2],
    /// `Some(discard)` is a pickup game, `None` Hand.
    pub(crate) discard: Option<[u8; 2]>,
    pub(crate) game: GameType,
    pub(crate) ouvert: bool,
    pub(crate) plays: Vec<u8>,
    pub(crate) viewer: Seat,
    /// `Some` = expected exact CardRule fields (without winners/losers).
    pub(crate) branch: Branch,
    pub(crate) card: u8,
    pub(crate) candidates: Vec<u8>,
    pub(crate) trumps_out: Vec<u8>,
    /// Whether the guided DrawTrumpf lesson is admitted for the viewer.
    pub(crate) lesson: bool,
}

pub(crate) fn deal_of(vh: &[u8], mh: &[u8], hh: &[u8], skat: [u8; 2]) -> Deal {
    Deal::new([set(vh), set(mh), set(hh)], set(&skat)).unwrap()
}

impl Case {
    pub(crate) fn deal(&self) -> Deal {
        deal_of(&self.deal[0], &self.deal[1], &self.deal[2], self.skat)
    }
    pub(crate) fn game(&self) -> Game {
        let mut g = Game::from_deal(self.deal(), RuleConfig { kontra_re: false });
        // Bidding: the declarer takes the bid at 18 and the others pass.
        while g.declarer().is_none() && g.phase() == skat_core::Phase::Reizen {
            let s = g.current_seat().unwrap();
            let legal = g.legal_actions();
            let a = if s == self.declarer {
                [Action::Bid(18), Action::Hold]
                    .into_iter()
                    .find(|a| legal.contains(a))
                    .unwrap()
            } else {
                Action::Pass
            };
            g.apply(a)
                .unwrap_or_else(|e| panic!("{} {a:?}: {e:?}", self.id));
        }
        let mut apply = |a: Action| {
            g.apply(a)
                .unwrap_or_else(|e| panic!("{} {a:?}: {e:?}", self.id));
        };
        match self.discard {
            None => apply(Action::Hand),
            Some(d) => {
                apply(Action::TakeSkat);
                apply(Action::Discard(set(&d)));
            }
        }
        apply(Action::Announce(Announcement {
            ouvert: self.ouvert,
            schneider: self.ouvert,
            schwarz: self.ouvert,
            ..Announcement::plain(self.game)
        }));
        for &p in &self.plays {
            apply(Action::Play(card(p)));
        }
        assert_eq!(g.declarer(), Some(self.declarer), "{}", self.id);
        g
    }
}

pub(crate) fn cases() -> Vec<Case> {
    let grand = GameType::Grand;
    let all_legal = |v: &[u8]| v.to_vec();
    let _ = all_legal;
    vec![
        // 01 Vorhand Hand declarer, CJ+SJ own, HJ/DJ unaccounted (defenders or Skat).
        Case {
            id: "01-vh-hand",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 28, 1, 6, 8, 10, 11, 14, 16, 18],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 4,
            candidates: vec![4, 12],
            trumps_out: vec![20, 28],
            lesson: true,
        },
        // 02 Mittelhand Hand declarer with the two lowest Buben.
        Case {
            id: "02-mh-hand-low-buben",
            declarer: Seat::Mittelhand,
            deal: [
                vec![4, 12, 1, 8, 10, 11, 14, 16, 18, 6],
                vec![20, 28, 7, 3, 9, 17, 25, 26, 2, 0],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![1, 7, 15],
            viewer: Seat::Mittelhand,
            branch: Branch::DrawTrumpf,
            card: 20,
            candidates: vec![20, 28],
            trumps_out: vec![4, 12],
            lesson: true,
        },
        // 03 Hinterhand pickup declarer with only DJ; a single Trumpf candidate.
        Case {
            id: "03-hh-pickup-single-trumpf",
            declarer: Seat::Hinterhand,
            deal: [
                vec![4, 12, 1, 8, 10, 11, 14, 16, 18, 19],
                vec![20, 0, 15, 21, 22, 23, 24, 27, 29, 30],
                vec![28, 7, 3, 9, 17, 25, 26, 2, 31, 6],
            ],
            skat: [5, 13],
            discard: Some([25, 26]),
            game: grand,
            ouvert: false,
            plays: vec![1, 0, 7],
            viewer: Seat::Hinterhand,
            branch: Branch::DrawTrumpf,
            card: 28,
            candidates: vec![28],
            trumps_out: vec![4, 12, 20],
            lesson: true,
        },
        // 04 Vorhand pickup: HJ comes from the Skat and is kept; own Skat is known.
        Case {
            id: "04-vh-pickup-kept-skat-bube",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 7, 3, 0, 9, 17, 25, 6, 2, 26],
                vec![12, 28, 1, 8, 10, 11, 14, 16, 18, 19],
                vec![15, 21, 22, 23, 24, 27, 29, 30, 31, 5],
            ],
            skat: [20, 13],
            discard: Some([6, 2]),
            game: grand,
            ouvert: false,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 4,
            candidates: vec![4, 20],
            trumps_out: vec![12, 28],
            lesson: true,
        },
        // 05 Known-discard-only trigger: own CJ SJ HJ, DJ discarded and thus known,
        // yet still counted by the conservative `trumps_out`.
        Case {
            id: "05-mh-known-discard-only",
            declarer: Seat::Mittelhand,
            deal: [
                vec![1, 8, 10, 11, 14, 15, 16, 18, 19, 21],
                vec![0, 2, 3, 4, 6, 7, 9, 12, 17, 20],
                vec![5, 22, 23, 24, 25, 26, 27, 29, 30, 31],
            ],
            skat: [13, 28],
            discard: Some([13, 28]),
            game: grand,
            ouvert: false,
            plays: vec![1, 7, 5],
            viewer: Seat::Mittelhand,
            branch: Branch::DrawTrumpf,
            card: 4,
            candidates: vec![4, 12, 20],
            trumps_out: vec![28],
            lesson: true,
        },
        // 06 All four Buben own: nothing is out, so LeadMaster (CA 11 Augen) instead.
        Case {
            id: "06-vh-all-four-own",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 20, 28, 7, 3, 0, 9, 17, 25],
                vec![1, 6, 8, 10, 11, 14, 16, 18, 19, 15],
                vec![21, 22, 23, 24, 27, 29, 30, 31, 26, 2],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::LeadMaster,
            card: 7,
            candidates: vec![3, 7],
            trumps_out: vec![],
            lesson: false,
        },
        // 07 Later lead: CJ won trick 1 against HJ, only DJ stays out.
        Case {
            id: "07-vh-later-lead-one-bube-out",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 28, 1, 6, 8, 10, 11, 14, 16, 18],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![4, 20, 15],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 12,
            candidates: vec![12],
            trumps_out: vec![28],
            lesson: true,
        },
        // 08 Outside Buben both played in trick 1: nothing out, LeadMaster instead.
        Case {
            id: "08-vh-later-lead-outside-buben-played",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 1, 6, 8, 10, 11, 14, 16, 18, 19],
                vec![28, 15, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![4, 20, 28],
            viewer: Seat::Vorhand,
            branch: Branch::LeadMaster,
            card: 7,
            candidates: vec![3, 7],
            trumps_out: vec![],
            lesson: false,
        },
        // 09 Later declarer lead after a defender led trick 1 and the declarer won it
        // from Hinterhand; DJ is the declarer's own, CJ/SJ/HJ stay out.
        Case {
            id: "09-hh-later-lead-after-follow",
            declarer: Seat::Hinterhand,
            deal: [
                vec![1, 8, 10, 11, 14, 16, 18, 19, 5, 13],
                vec![15, 21, 22, 23, 24, 27, 29, 30, 31, 12],
                vec![28, 7, 3, 0, 9, 17, 25, 26, 2, 6],
            ],
            skat: [20, 4],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![1, 15, 7],
            viewer: Seat::Hinterhand,
            branch: Branch::DrawTrumpf,
            card: 28,
            candidates: vec![28],
            trumps_out: vec![4, 12, 20],
            lesson: true,
        },
        // 10 No own Trumpf: LeadMaster (CA), never DrawTrumpf.
        Case {
            id: "10-hh-no-own-trumpf",
            declarer: Seat::Hinterhand,
            deal: [
                vec![4, 12, 1, 8, 11, 14, 16, 18, 19, 15],
                vec![20, 28, 0, 21, 22, 23, 24, 27, 29, 30],
                vec![7, 3, 9, 17, 25, 26, 2, 31, 10, 6],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![1, 0, 7],
            viewer: Seat::Hinterhand,
            branch: Branch::LeadMaster,
            card: 31,
            candidates: vec![3, 6, 31],
            trumps_out: vec![4, 12, 20, 28],
            lesson: false,
        },
        // 11 Competing side master: DrawTrumpf keeps priority over CA/C10.
        Case {
            id: "11-vh-side-master-competes",
            declarer: Seat::Vorhand,
            deal: [
                vec![12, 7, 3, 0, 9, 17, 25, 26, 2, 1],
                vec![4, 20, 8, 10, 11, 14, 16, 18, 19, 6],
                vec![28, 15, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 12,
            candidates: vec![12],
            trumps_out: vec![4, 20, 28],
            lesson: true,
        },
        // 12 Defender view of case 01: never DrawTrumpf, not a guided lesson here.
        Case {
            id: "12-defender-leads-later",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 28, 1, 6, 8, 10, 11, 14, 16, 18],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: false,
            plays: vec![0, 6, 15],
            viewer: Seat::Mittelhand,
            branch: Branch::LeadMaster,
            card: 0,
            candidates: vec![],
            trumps_out: vec![],
            lesson: false,
        },
        // 13 Ouvert Grand Hand: excluded from the lesson.
        Case {
            id: "13-ouvert-excluded",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 28, 1, 6, 8, 10, 11, 14, 16, 18],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: grand,
            ouvert: true,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 4,
            candidates: vec![4, 12],
            trumps_out: vec![20, 28],
            lesson: false,
        },
        // 14 Farbspiel (Kreuz): shared rule may lead Trumpf, the Grand lesson does not.
        Case {
            id: "14-farbspiel-excluded",
            declarer: Seat::Vorhand,
            deal: [
                vec![4, 12, 7, 3, 0, 9, 17, 25, 26, 2],
                vec![20, 28, 1, 6, 8, 10, 11, 14, 16, 18],
                vec![15, 19, 21, 22, 23, 24, 27, 29, 30, 31],
            ],
            skat: [5, 13],
            discard: None,
            game: GameType::Farbe(skat_core::Suit::Kreuz),
            ouvert: false,
            plays: vec![],
            viewer: Seat::Vorhand,
            branch: Branch::DrawTrumpf,
            card: 7,
            candidates: vec![],
            trumps_out: vec![],
            lesson: false,
        },
    ]
}

fn legal_cards(v: &skat_core::PlayerView) -> Vec<Card> {
    v.legal_actions
        .iter()
        .filter_map(|a| match a {
            Action::Play(c) => Some(*c),
            _ => None,
        })
        .collect()
}

#[test]
fn draw_trumpf_admission_branch_candidates_order_and_trumps_out() {
    let mut count = 0;
    for c in cases() {
        let g = c.game();
        let v = g.view(c.viewer);
        let rule = baseline::rule(&v).unwrap_or_else(|| panic!("{}", c.id));
        // Cases 12 and 14 only pin the exclusion; their selected card is not an oracle.
        if matches!(c.id, "12-defender-leads-later" | "14-farbspiel-excluded") {
            if c.id.starts_with("12") {
                assert_ne!(rule.branch, Branch::DrawTrumpf);
            }
            count += 1;
            continue;
        }
        let legal = legal_cards(&v);
        assert_eq!(rule.branch, c.branch, "{}", c.id);
        assert_eq!(rule.card, card(c.card), "{}", c.id);
        if c.branch == Branch::DrawTrumpf {
            let k = Knowledge::from_view(&v);
            assert_eq!(k.trumps_out(), set(&c.trumps_out), "{}", c.id);
            // All eleven CardRule fields on an exactly empty table.
            let power = 3 - i32::from(c.card / 8);
            assert_eq!(
                rule,
                CardRule {
                    card: card(c.card),
                    branch: Branch::DrawTrumpf,
                    candidates: c.candidates.iter().map(|&n| card(n)).collect(),
                    winners: legal.clone(),
                    losers: vec![],
                    winner: None,
                    partner_ahead: false,
                    last: false,
                    table_master: false,
                    tie_break: TieBreak::HighestPowerLast,
                    tie_key: (power, 0, 0),
                },
                "{}",
                c.id
            );
        } else {
            assert_eq!(
                rule.candidates,
                c.candidates.iter().map(|&n| card(n)).collect::<Vec<_>>(),
                "{}",
                c.id
            );
            assert_eq!(Knowledge::from_view(&v).trumps_out(), set(&c.trumps_out));
        }
        assert!(legal.len() >= 2, "{}", c.id);
        count += 1;
    }
    assert_eq!(count, 14);
}

#[test]
fn draw_trumpf_legal_list_is_exactly_the_ten_hand_cards() {
    // Shared-rule side: a single legal card or a non-Play action never
    // yields a Play rule list of two or more exact cards (coach admission
    // is asserted in skat-coach/tests/guided_draw_trumpf.rs).
    let c = &cases()[0];
    let v = c.game().view(c.viewer);
    assert_eq!(legal_cards(&v).len(), 10);
    assert_eq!(baseline::rule(&v).unwrap().branch, Branch::DrawTrumpf);
}

/// Every admitted next trick, exhaustively, with the independent winner.
#[test]
fn draw_trumpf_exhaustive_next_trick_continuations_and_winners() {
    fn walk(g: &Game, n: &mut usize, bubes: &mut usize) {
        for a in g.legal_actions() {
            let mut h = g.clone();
            let before = h.tricks().len();
            h.apply(a).unwrap();
            if h.tricks().len() > before {
                let t = h.tricks().last().unwrap();
                let cards = t.cards.map(|c| c.index());
                assert_eq!(t.winner, winner(t.leader, cards), "{cards:?}");
                *n += 1;
                *bubes += usize::from(cards.iter().any(|c| c % 8 == 4));
            } else {
                walk(&h, n, bubes);
            }
        }
    }
    let mut total = 0;
    for c in cases().iter().filter(|c| c.lesson) {
        let g = c.game();
        let (mut n, mut bubes) = (0, 0);
        walk(&g, &mut n, &mut bubes);
        assert!(n > 0 && bubes > 0, "{}", c.id);
        total += n;
    }
    // The represented set only: counts of the bounded cases above.
    assert!(total > 500, "{total}");
    eprintln!("draw_trumpf next-trick continuations: {total}");
}

/// A bounded late history: play the remaining tricks with the first legal
/// card and compare each completed trick with the independent winner.
#[test]
fn draw_trumpf_terminal_continuations_use_the_independent_winner() {
    for c in cases().iter().filter(|c| c.lesson) {
        let mut g = c.game();
        while g.phase() != skat_core::Phase::Beendet {
            let before = g.tricks().len();
            let a = *g.legal_actions().first().unwrap();
            g.apply(a).unwrap();
            if g.tricks().len() > before {
                let t = g.tricks().last().unwrap();
                assert_eq!(t.winner, winner(t.leader, t.cards.map(|c| c.index())));
            }
        }
        assert_eq!(g.tricks().len(), 10, "{}", c.id);
    }
}

/// Same permitted view, different concealed defender cards: rule, trumps_out
/// and the guided lesson do not change.
#[test]
fn draw_trumpf_hidden_substitutions_keep_view_rule_and_lesson() {
    let swaps: [(&str, u8, u8); 3] = [
        ("01-vh-hand", 1, 15),
        ("03-hh-pickup-single-trumpf", 8, 15),
        ("11-vh-side-master-competes", 8, 29),
    ];
    for (id, a, b) in swaps {
        let c = cases().into_iter().find(|c| c.id == id).unwrap();
        let (v0, rule0) = {
            let v = c.game().view(c.viewer);
            (v.clone(), baseline::rule(&v))
        };
        let swap = |h: &[u8]| -> Vec<u8> {
            h.iter()
                .map(|&n| {
                    if n == a {
                        b
                    } else if n == b {
                        a
                    } else {
                        n
                    }
                })
                .collect()
        };
        let mut other = cases().into_iter().find(|c| c.id == id).unwrap();
        for s in [Seat::Vorhand, Seat::Mittelhand, Seat::Hinterhand] {
            if s != c.viewer && s != c.declarer {
                // Exchange only between the two concealed holders.
                other.deal[s.index()] = swap(&other.deal[s.index()]);
            }
        }
        // The swap must change the concealed deal and nothing the viewer sees.
        assert_ne!(other.deal(), c.deal(), "{id}");
        let v1 = other.game().view(c.viewer);
        assert_eq!(v1, v0, "{id}");
        assert_eq!(baseline::rule(&v1), rule0, "{id}");
    }
}
