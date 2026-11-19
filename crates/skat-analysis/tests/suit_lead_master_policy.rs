// SPDX-License-Identifier: EUPL-1.2

//! Fresh handwritten #182 evidence: non-Ouvert Farbspiel `LeadMaster` on an own
//! empty-trick lead. Card index = suit*8 + rank with suits Kreuz, Pik, Herz,
//! Karo and ranks 7, 8, 9, 10, Bube, Dame, Koenig, Ass (Ass = 7, Zehn = 3).
//! Expectations were derived by hand from the ISkO ranks, not from the code.

use skat_analysis::Knowledge;
use skat_analysis::baseline::{self, Branch, TieBreak};
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Game, GameType, Phase, RuleConfig, Seat, Suit,
};

pub(crate) fn card(n: u8) -> Card {
    Card::from_index(n).unwrap()
}
pub(crate) fn set(ns: &[u8]) -> CardSet {
    ns.iter().copied().map(card).collect()
}

/// Vorhand holds `vh`; Mittelhand/Hinterhand get the preferred cards first, then
/// the remaining cards in index order; the last two cards form the Skat.
pub(crate) fn deal(vh: &[u8], mh: &[u8], hh: &[u8]) -> Deal {
    let mut rest: Vec<u8> = (0..32)
        .filter(|n| !vh.contains(n) && !mh.contains(n) && !hh.contains(n))
        .collect();
    let mut take = |pref: &[u8], out: &mut Vec<u8>| {
        out.extend_from_slice(pref);
        while out.len() < 10 {
            out.push(rest.remove(0));
        }
    };
    let (mut a, mut b) = (vec![], vec![]);
    take(mh, &mut a);
    take(hh, &mut b);
    Deal::new([set(vh), set(&a), set(&b)], set(&rest)).unwrap()
}

pub(crate) fn play(deal: Deal, suit: Suit, bids: [Action; 3], extra: &[u8]) -> Game {
    let mut g = Game::from_deal(deal, RuleConfig { kontra_re: false });
    for a in bids
        .into_iter()
        .chain([
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Farbe(suit))),
        ])
        .chain(extra.iter().map(|&n| Action::Play(card(n))))
    {
        g.apply(a).unwrap_or_else(|e| panic!("{a:?}: {e:?}"));
    }
    g
}
/// Mittelhand declares Hand; Vorhand is a defender.
pub(crate) const MH_DECLARES: [Action; 3] = [Action::Bid(18), Action::Pass, Action::Pass];

pub(crate) struct Case {
    pub(crate) id: &'static str,
    pub(crate) deal: Deal,
    pub(crate) game: Game,
    pub(crate) card: u8,
    pub(crate) candidates: &'static [u8],
}

pub(crate) fn cases() -> Vec<Case> {
    let d0 = deal(&[3, 7, 4, 11, 14, 31, 26, 16, 23, 12], &[], &[]);
    let d1 = deal(&[15, 11, 23, 19, 30, 24, 0, 7, 20, 26], &[], &[]);
    let d2 = deal(&[3, 23, 22, 27, 15, 9, 6, 8, 4, 28], &[], &[]);
    let d3 = deal(&[7, 3, 15, 11, 16, 24, 31, 4, 12, 22], &[], &[]);
    let d4 = deal(
        &[3, 7, 4, 11, 14, 31, 26, 16, 23, 12],
        &[24, 27, 29, 30],
        &[25],
    );
    vec![
        // Herz: own CA/C10 both masters (own higher does not block C10); CA and
        // KaroA tie at 11 Augen, LAST in core order wins. Bube/Herz cards are Trumpf.
        Case {
            id: "herz-tie-last",
            deal: d0,
            game: play(d0, Suit::Herz, MH_DECLARES, &[]),
            card: 31,
            candidates: &[3, 7, 31],
        },
        // Kreuz: Kreuz-Ass is Trumpf and never a plain candidate; PikA/HerzA tie.
        Case {
            id: "kreuz-announced-ace-excluded",
            deal: d1,
            game: play(d1, Suit::Kreuz, MH_DECLARES, &[]),
            card: 23,
            candidates: &[11, 15, 19, 23],
        },
        // Pik: single candidate; own Pik-Ass/Buben are Trumpf, KreuzK/Karo10 have unseen higher cards.
        Case {
            id: "pik-single",
            deal: d2,
            game: play(d2, Suit::Pik, MH_DECLARES, &[]),
            card: 23,
            candidates: &[23],
        },
        // Karo: KreuzA/KreuzZehn/PikA/Pik10 masters; KreuzA and PikA tie, PikA last.
        Case {
            id: "karo-tie-last",
            deal: d3,
            game: play(d3, Suit::Karo, MH_DECLARES, &[]),
            card: 15,
            candidates: &[3, 7, 11, 15],
        },
        // Late own lead after Vorhand won trick 1 with KaroA: KaroA is played, so
        // KreuzA beats C10 on Augen and the Karo 9 remains blocked by Karo 10.
        Case {
            id: "herz-late-after-win",
            deal: d4,
            game: play(d4, Suit::Herz, MH_DECLARES, &[31, 24, 25]),
            card: 7,
            candidates: &[3, 7],
        },
    ]
}

#[test]
fn farbspiel_lead_master_rules_match_handwritten_expectations() {
    for c in cases() {
        let seat = c.game.current_seat().unwrap();
        assert_eq!(seat, Seat::Vorhand, "{}", c.id);
        let v = c.game.view(seat);
        let r = baseline::rule(&v).unwrap();
        assert_eq!(r.branch, Branch::LeadMaster, "{}", c.id);
        assert_eq!(r.card, card(c.card), "{}", c.id);
        assert_eq!(
            r.candidates,
            c.candidates.iter().map(|&n| card(n)).collect::<Vec<_>>(),
            "{}",
            c.id
        );
        assert_eq!(r.tie_break, TieBreak::HighestAugenLast);
        assert_eq!(r.winner, None);
        let k = Knowledge::from_view(&v);
        let masters: Vec<_> = v
            .hand
            .iter()
            .filter(|&x| !c.game_type().is_trump(x) && k.is_master(x))
            .collect();
        assert_eq!(masters, r.candidates, "{}", c.id);
    }
}

impl Case {
    pub(crate) fn game_type(&self) -> GameType {
        self.game.contract().unwrap().game
    }
}

#[test]
fn farbspiel_lead_master_void_follower_trumps_so_a_master_is_no_guarantee() {
    // Hinterhand holds no Karo and trumps KaroA with Herz 7: master of its class, not of the trick.
    let mut g = play(
        deal(
            &[3, 7, 4, 11, 14, 31, 26, 16, 23, 12],
            &[24, 25, 27, 29, 30],
            &[17, 18, 19, 21, 22, 1, 2, 5, 6, 8],
        ),
        Suit::Herz,
        MH_DECLARES,
        &[],
    );
    for n in [31, 24, 17] {
        g.apply(Action::Play(card(n))).unwrap();
    }
    assert_eq!(g.tricks().last().unwrap().winner, Seat::Hinterhand);
}

/// The "herz-tie-last" deal plus its two hidden-deal substitutions (swap the two hidden hands;
/// swap one Hinterhand card with a Skat card). Vorhand's view is identical in all three.
pub(crate) fn hidden_deals() -> [Deal; 3] {
    let d = deal(&[3, 7, 4, 11, 14, 31, 26, 16, 23, 12], &[], &[]);
    let (h0, h1, h2, skat) = (
        d.hand(Seat::Vorhand),
        d.hand(Seat::Mittelhand),
        d.hand(Seat::Hinterhand),
        d.skat(),
    );
    let swapped_hands = Deal::new([h0, h2, h1], skat).unwrap();
    // Hinterhand's Herz 9 (a non-Karo card) and the Skat's Karo Dame change places: after the
    // Karo lead both deals still share legal replies.
    let a = h2.iter().next().unwrap();
    let s = skat.iter().next().unwrap();
    let (mut n2, mut nsk) = (h2, skat);
    n2.remove(a);
    n2.insert(s);
    nsk.remove(s);
    nsk.insert(a);
    [d, swapped_hands, Deal::new([h0, h1, n2], nsk).unwrap()]
}

#[test]
fn farbspiel_lead_master_hidden_deal_substitutions_keep_the_view_and_rule() {
    let [d, ..] = hidden_deals();
    let base = play(d, Suit::Herz, MH_DECLARES, &[]);
    let v0 = base.view(Seat::Vorhand);
    for alt in &hidden_deals()[1..] {
        let g = play(*alt, Suit::Herz, MH_DECLARES, &[]);
        let v = g.view(Seat::Vorhand);
        assert_eq!(v, v0);
        assert_eq!(baseline::rule(&v), baseline::rule(&v0));
    }
    assert_eq!(base.phase(), Phase::Spiel);
}

// ---- Independent legal-witness oracle (#182) -------------------------------------------------
// Written from the ISkO rank/Augen tables only. It shares no code with `skat-analysis` or the
// core trick evaluator; it uses the core only to enumerate legal actions (the legality witness).

const SEATS: [Seat; 3] = [Seat::Vorhand, Seat::Mittelhand, Seat::Hinterhand];

fn is_trump(trump: u8, c: u8) -> bool {
    c % 8 == 4 || c / 8 == trump
}
/// Ordering power inside one follow class (higher beats lower).
fn power(c: u8) -> u8 {
    if c % 8 == 4 {
        return 20 + (3 - c / 8); // Kreuz-Bube highest
    }
    match c % 8 {
        7 => 7, // Ass
        3 => 6, // Zehn
        6 => 5, // Koenig
        5 => 4, // Dame
        r => r + 1,
    }
}
fn oracle_augen(c: u8) -> u8 {
    match c % 8 {
        7 => 11,
        3 => 10,
        6 => 4,
        5 => 3,
        4 => 2,
        _ => 0,
    }
}
fn class(trump: u8, c: u8) -> u8 {
    if is_trump(trump, c) { 9 } else { c / 8 }
}
/// Index in play order (0 = leader) of the trick winner.
pub(crate) fn oracle_winner(trump: u8, t: [u8; 3]) -> usize {
    let mut best = 0;
    for i in 1..3 {
        let (c, b) = (t[i], t[best]);
        let wins = if class(trump, c) == class(trump, b) {
            power(c) > power(b)
        } else {
            is_trump(trump, c)
        };
        if wins {
            best = i;
        }
    }
    best
}
/// Plain-suit master candidates and Easy's pick (max Augen, last in core order on a tie).
pub(crate) fn oracle_lead(trump: u8, hand: &[u8], played: &[u8]) -> (Vec<u8>, u8) {
    let mut cands: Vec<u8> = hand
        .iter()
        .copied()
        .filter(|&c| !is_trump(trump, c))
        .filter(|&c| {
            (0..32u8)
                .filter(|&y| class(trump, y) == class(trump, c) && power(y) > power(c))
                .all(|y| hand.contains(&y) || played.contains(&y))
        })
        .collect();
    cands.sort();
    let max = cands.iter().map(|&c| oracle_augen(c)).max().unwrap();
    let pick = *cands
        .iter()
        .rev()
        .find(|&&c| oracle_augen(c) == max)
        .unwrap();
    (cands, pick)
}
fn trump_index(game: &Game) -> u8 {
    match game.contract().unwrap().game {
        GameType::Farbe(s) => s.index(),
        g => panic!("{g:?}"),
    }
}
/// Every legal (lead, reply, reply) completion of the next trick with the oracle winner, which
/// must equal the winner the core recorded.
pub(crate) fn completions(game: &Game) -> Vec<([u8; 3], Seat)> {
    let trump = trump_index(game);
    let first = game.current_seat().unwrap();
    let at = SEATS.iter().position(|&s| s == first).unwrap();
    let mut out = vec![];
    let plays = |g: &Game| -> Vec<u8> {
        g.legal_actions()
            .iter()
            .map(|a| match a {
                Action::Play(c) => c.index(),
                a => panic!("{a:?}"),
            })
            .collect()
    };
    for a in plays(game) {
        let mut g1 = game.clone();
        g1.apply(Action::Play(card(a))).unwrap();
        for b in plays(&g1) {
            let mut g2 = g1.clone();
            g2.apply(Action::Play(card(b))).unwrap();
            for c in plays(&g2) {
                let mut g3 = g2.clone();
                g3.apply(Action::Play(card(c))).unwrap();
                let t = g3.tricks().last().unwrap();
                let winner = SEATS[(at + oracle_winner(trump, [a, b, c])) % 3];
                assert_eq!(t.winner, winner, "{a} {b} {c}");
                out.push(([a, b, c], winner));
            }
        }
    }
    out
}
/// Frozen FNV-1a digest of an ordered completion list.
pub(crate) fn digest(list: &[([u8; 3], Seat)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (t, w) in list {
        for b in t
            .iter()
            .copied()
            .chain([SEATS.iter().position(|s| s == w).unwrap() as u8])
        {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}
pub(crate) fn played_cards(g: &Game) -> Vec<u8> {
    g.tricks()
        .iter()
        .flat_map(|t| t.cards)
        .chain(g.current_trick().cards.into_iter().flatten())
        .map(|c| c.index())
        .collect()
}
pub(crate) fn hand_of(g: &Game, seat: Seat) -> Vec<u8> {
    g.view(seat).hand.iter().map(|c| c.index()).collect()
}

/// Frozen (case id, completion count, digest). The oracle winner logic above is independent;
/// the counts and digests were calibrated from it (source-informed) and now guard regressions.
pub(crate) const FROZEN: &[(&str, usize, u64)] = &[
    ("herz-tie-last", 540, 0x3bd83b06b5387ce9),
    ("kreuz-announced-ace-excluded", 193, 0x62114389929d4743),
    ("pik-single", 265, 0x931e78dd1ac5fa17),
    ("karo-tie-last", 500, 0x83afc0bf66600855),
    ("herz-late-after-win", 305, 0xe0602e7f863a77ea),
];

#[test]
fn independent_oracle_matches_baseline_candidates_pick_and_every_next_trick_completion() {
    for c in cases() {
        let seat = c.game.current_seat().unwrap();
        let trump = trump_index(&c.game);
        let mut dealt = c.deal.hand(seat);
        for p in played_cards(&c.game) {
            dealt.remove(card(p)); // the late case already played one of its dealt cards
        }
        assert_eq!(dealt, c.game.view(seat).hand, "{}", c.id);
        let (cands, pick) = oracle_lead(trump, &hand_of(&c.game, seat), &played_cards(&c.game));
        assert_eq!(cands, c.candidates, "{}", c.id);
        assert_eq!(pick, c.card, "{}", c.id);
        let list = completions(&c.game);
        assert!(list.iter().any(|(t, _)| t[0] == pick), "{}", c.id);
        let (_, n, d) = *FROZEN.iter().find(|f| f.0 == c.id).unwrap();
        assert_eq!((list.len(), digest(&list)), (n, d), "{}", c.id);
    }
}

#[test]
fn oracle_winner_hand_checked_tricks() {
    // Herz trumps: Karo Bube beats Herz Ass; Herz 7 beats Kreuz Ass; Pik Ass beats Pik 10.
    assert_eq!(oracle_winner(2, [28, 23, 16]), 0); // Karo Bube (28) vs Herz Ass (23), Herz 7
    assert_eq!(oracle_winner(2, [7, 16, 3]), 1); // Kreuz Ass led, Herz 7 trumps
    assert_eq!(oracle_winner(2, [15, 11, 8]), 0); // Pik Ass over Pik 10 and Pik 7
    assert_eq!(oracle_winner(0, [4, 12, 28]), 0); // Kreuz Bube beats Pik/Karo Bube
}
