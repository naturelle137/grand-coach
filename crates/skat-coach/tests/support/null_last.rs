// SPDX-License-Identifier: EUPL-1.2

//! Fixture-only complete deals from the independently handwritten #165 preflight
//! (QA1458). `ORIGINAL` cases come from the frozen original oracle, `ADDENDUM`
//! cases from the separately hashed post-source supplement; hashes and the
//! retained metadata discrepancy are recorded in docs/guided-dialogue.md.
//! Cards use suit letters C/S/H/D and ranks 7 8 9 T J Q K A.
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Game, GameType, Rank, RuleConfig, Seat, Suit,
};

/// Which frozen source a case comes from; the supplement never replaces the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Original,
    Addendum,
}

pub struct Case {
    pub id: &'static str,
    pub origin: Origin,
    /// Vorhand, Mittelhand, Hinterhand.
    pub hands: [&'static str; 3],
    pub skat: &'static str,
    pub bidding: &'static [&'static str],
    pub declarer: &'static str,
    /// Hand game; otherwise pickup discarding `DK DA`.
    pub hand: bool,
    pub ouvert: bool,
    pub plays: &'static [&'static str],
    pub actor: &'static str,
    pub legal: &'static [&'static str],
    /// Current public winner before the actor's card; none at the lead.
    pub winner: Option<&'static str>,
    /// Individual completed winner per legal card; empty: no assertion
    /// (early/lead, where another player still follows).
    pub completed: &'static [&'static str],
    pub losers: &'static [&'static str],
    /// Null power per legal card on the oracle's 0..7 scale (7 = 0).
    pub power: &'static [i32],
    pub choice: Option<&'static str>,
    pub supported: bool,
}

pub fn card(s: &str) -> Card {
    Card::new(
        Suit::ALL["CSHD".find(&s[0..1]).unwrap()],
        Rank::ALL["789TJQKA".find(&s[1..2]).unwrap()],
    )
}

pub fn cards(s: &[&str]) -> Vec<Card> {
    s.iter().map(|c| card(c)).collect()
}

pub fn set(s: &str) -> CardSet {
    s.split_whitespace().map(card).collect()
}

pub fn seat(s: &str) -> Seat {
    match s {
        "VH" => Seat::Vorhand,
        "MH" => Seat::Mittelhand,
        "HH" => Seat::Hinterhand,
        _ => panic!("seat {s}"),
    }
}

impl Case {
    pub fn position(&self) -> Game {
        self.position_with(self.hands, self.skat)
    }

    /// Same public history with exchanged hidden cards (`"VH H7"`, `"Skat DK"`).
    pub fn swapped(&self, a: &str, b: &str) -> Game {
        let mut hands = self.hands.map(set);
        let mut skat = set(self.skat);
        let (ca, cb) = (card(&a[a.len() - 2..]), card(&b[b.len() - 2..]));
        for (place, old, new) in [(a, ca, cb), (b, cb, ca)] {
            let target = if place.starts_with("Skat") {
                &mut skat
            } else {
                &mut hands[seat(&place[..2]).index()]
            };
            assert!(target.remove(old), "{place}");
            target.insert(new);
        }
        self.replay(Deal::new(hands, skat).unwrap())
    }

    fn position_with(&self, hands: [&str; 3], skat: &str) -> Game {
        self.replay(Deal::new(hands.map(set), set(skat)).unwrap())
    }

    fn replay(&self, deal: Deal) -> Game {
        let mut g = Game::from_deal(deal, RuleConfig { kontra_re: false });
        for b in self.bidding {
            assert_eq!(g.current_seat(), Some(seat(&b[..2])), "{} {b}", self.id);
            g.apply(match &b[3..] {
                "pass" => Action::Pass,
                "bid18" => Action::Bid(18),
                other => panic!("bid {other}"),
            })
            .unwrap();
        }
        if self.hand {
            g.apply(Action::Hand).unwrap();
        } else {
            g.apply(Action::TakeSkat).unwrap();
            g.apply(Action::Discard(set("DK DA"))).unwrap();
        }
        let mut ann = Announcement::plain(GameType::Null);
        ann.ouvert = self.ouvert;
        g.apply(Action::Announce(ann)).unwrap();
        for p in self.plays {
            assert_eq!(g.current_seat(), Some(seat(&p[..2])), "{} {p}", self.id);
            g.apply(Action::Play(card(&p[3..]))).unwrap();
        }
        g
    }
}

const BASE: [&str; 3] = [
    "CT CK CA H7 H8 H9 HT HJ HQ HK",
    "C9 S7 S8 S9 ST SJ SQ SK SA HA",
    "C7 C8 CJ CQ D7 D8 D9 DT DJ DQ",
];
const HH_DECLARES: &[&str] = &["MH pass", "HH bid18", "VH pass"];
const CLUBS: &[&str] = &["C7", "C8", "CJ", "CQ"];

pub fn cases() -> Vec<Case> {
    let null = |id, origin, hands, plays, legal, winner, completed, losers, power, choice| Case {
        id,
        origin,
        hands,
        skat: "DK DA",
        bidding: HH_DECLARES,
        declarer: "HH",
        hand: true,
        ouvert: false,
        plays,
        actor: "HH",
        legal,
        winner: Some(winner),
        completed,
        losers,
        power,
        choice,
        supported: choice.is_some(),
    };
    let void_legal: &[&str] = &["S7", "S8", "S9", "ST", "SJ", "D7", "D8", "D9", "DT", "DJ"];
    vec![
        null(
            "mixed-ten",
            Origin::Original,
            BASE,
            &["VH CT", "MH C9"],
            CLUBS,
            "VH",
            &["VH", "VH", "HH", "HH"],
            &["C7", "C8"],
            &[0, 1, 4, 5],
            Some("C8"),
        ),
        null(
            "all-loser-king",
            Origin::Original,
            BASE,
            &["VH CK", "MH C9"],
            CLUBS,
            "VH",
            &["VH"; 4],
            CLUBS,
            &[0, 1, 4, 5],
            Some("CQ"),
        ),
        null(
            "printed-bube",
            Origin::Original,
            [
                "CJ CK CA H7 H8 H9 HT HJ HQ HK",
                BASE[1],
                "C7 C8 CT CQ D7 D8 D9 DT DJ DQ",
            ],
            &["VH CJ", "MH C9"],
            &["C7", "C8", "CT", "CQ"],
            "VH",
            &["VH", "VH", "VH", "HH"],
            &["C7", "C8", "CT"],
            &[0, 1, 3, 5],
            Some("CT"),
        ),
        null(
            "void-last-tie",
            Origin::Original,
            [
                "C7 C8 C9 CT CJ CQ CK CA H7 H8",
                "SQ SK SA H9 HT HJ HQ HK HA DQ",
                "S7 S8 S9 ST SJ D7 D8 D9 DT DJ",
            ],
            &["VH CJ", "MH H9"],
            void_legal,
            "VH",
            &["VH"; 10],
            void_legal,
            &[0, 1, 2, 3, 4, 0, 1, 2, 3, 4],
            Some("DJ"),
        ),
        Case {
            hand: false,
            ..null(
                "pickup-mixed",
                Origin::Original,
                BASE,
                &["VH CT", "MH C9"],
                CLUBS,
                "VH",
                &["VH", "VH", "HH", "HH"],
                &["C7", "C8"],
                &[0, 1, 4, 5],
                Some("C8"),
            )
        },
        null(
            "no-loser",
            Origin::Original,
            [
                "C7 CK CA H7 H8 H9 HT HJ HQ HK",
                "C8 S7 S8 S9 ST SJ SQ SK SA HA",
                "CT C9 CJ CQ D7 D8 D9 DT DJ DQ",
            ],
            &["VH C7", "MH C8"],
            &["C9", "CT", "CJ", "CQ"],
            "MH",
            &["HH"; 4],
            &[],
            &[2, 3, 4, 5],
            None,
        ),
        null(
            "void-changed-queen",
            Origin::Addendum,
            [
                "C7 C8 C9 CT CJ CQ CK CA H7 H8",
                "SQ SK SA H9 HT HJ HQ HK HA DJ",
                "S7 S8 S9 ST SJ D7 D8 D9 DT DQ",
            ],
            &["VH CJ", "MH H9"],
            &["S7", "S8", "S9", "ST", "SJ", "D7", "D8", "D9", "DT", "DQ"],
            "VH",
            &["VH"; 10],
            &["S7", "S8", "S9", "ST", "SJ", "D7", "D8", "D9", "DT", "DQ"],
            &[0, 1, 2, 3, 4, 0, 1, 2, 3, 5],
            Some("DQ"),
        ),
        // Actual core replays of exclusion boundaries (supplement only).
        Case {
            bidding: &["MH pass", "HH pass", "VH bid18"],
            declarer: "VH",
            supported: false,
            ..null(
                "real-defender-declarer-ahead",
                Origin::Addendum,
                BASE,
                &["VH CT", "MH C9"],
                CLUBS,
                "VH",
                &["VH", "VH", "HH", "HH"],
                &["C7", "C8"],
                &[0, 1, 4, 5],
                Some("C8"),
            )
        },
        Case {
            bidding: &["MH bid18", "VH pass", "HH pass"],
            declarer: "MH",
            supported: false,
            ..null(
                "real-defender-partner-ahead",
                Origin::Addendum,
                BASE,
                &["VH CT", "MH C9"],
                CLUBS,
                "VH",
                &[],
                &["C7", "C8"],
                &[0, 1, 4, 5],
                None,
            )
        },
        Case {
            bidding: &["MH pass", "HH pass", "VH bid18"],
            declarer: "VH",
            actor: "VH",
            winner: None,
            completed: &[],
            losers: &[],
            supported: false,
            ..null(
                "real-lead",
                Origin::Addendum,
                BASE,
                &[],
                &["CT", "CK", "CA", "H7", "H8", "H9", "HT", "HJ", "HQ", "HK"],
                "VH",
                &[],
                &[],
                &[3, 6, 7, 0, 1, 2, 3, 4, 5, 6],
                None,
            )
        },
        Case {
            bidding: &["MH bid18", "VH pass", "HH pass"],
            declarer: "MH",
            actor: "MH",
            completed: &[],
            supported: false,
            ..null(
                "real-early",
                Origin::Addendum,
                [
                    BASE[0],
                    "C7 C9 S8 S9 ST SJ SQ SK SA HA",
                    "C8 CJ CQ S7 D7 D8 D9 DT DJ DQ",
                ],
                &["VH CT"],
                &["C7", "C9"],
                "VH",
                &[],
                &["C7", "C9"],
                &[0, 2],
                None,
            )
        },
        Case {
            ouvert: true,
            supported: false,
            ..null(
                "real-ouvert",
                Origin::Addendum,
                BASE,
                &["VH CT", "MH C9"],
                CLUBS,
                "VH",
                &["VH", "VH", "HH", "HH"],
                &["C7", "C8"],
                &[0, 1, 4, 5],
                Some("C8"),
            )
        },
    ]
}
