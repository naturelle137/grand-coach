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
    pub(crate) game: Game,
    pub(crate) card: u8,
    pub(crate) candidates: &'static [u8],
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        // Herz: own CA/C10 both masters (own higher does not block C10); CA and
        // KaroA tie at 11 Augen, LAST in core order wins. Bube/Herz cards are Trumpf.
        Case {
            id: "herz-tie-last",
            game: play(
                deal(&[3, 7, 4, 11, 14, 31, 26, 16, 23, 12], &[], &[]),
                Suit::Herz,
                MH_DECLARES,
                &[],
            ),
            card: 31,
            candidates: &[3, 7, 31],
        },
        // Kreuz: Kreuz-Ass is Trumpf and never a plain candidate; PikA/HerzA tie.
        Case {
            id: "kreuz-announced-ace-excluded",
            game: play(
                deal(&[15, 11, 23, 19, 30, 24, 0, 7, 20, 26], &[], &[]),
                Suit::Kreuz,
                MH_DECLARES,
                &[],
            ),
            card: 23,
            candidates: &[11, 15, 19, 23],
        },
        // Pik: single candidate; own Pik-Ass/Buben are Trumpf, KreuzK/Karo10 have unseen higher cards.
        Case {
            id: "pik-single",
            game: play(
                deal(&[3, 23, 22, 27, 15, 9, 6, 8, 4, 28], &[], &[]),
                Suit::Pik,
                MH_DECLARES,
                &[],
            ),
            card: 23,
            candidates: &[23],
        },
        // Karo: KreuzA/KreuzZehn/PikA/Pik10 masters; KreuzA and PikA tie, PikA last.
        Case {
            id: "karo-tie-last",
            game: play(
                deal(&[7, 3, 15, 11, 16, 24, 31, 4, 12, 22], &[], &[]),
                Suit::Karo,
                MH_DECLARES,
                &[],
            ),
            card: 15,
            candidates: &[3, 7, 11, 15],
        },
        // Late own lead after Vorhand won trick 1 with KaroA: KaroA is played, so
        // KreuzA beats C10 on Augen and the Karo 9 remains blocked by Karo 10.
        Case {
            id: "herz-late-after-win",
            game: play(
                deal(
                    &[3, 7, 4, 11, 14, 31, 26, 16, 23, 12],
                    &[24, 27, 29, 30],
                    &[25],
                ),
                Suit::Herz,
                MH_DECLARES,
                &[31, 24, 25],
            ),
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

#[test]
fn farbspiel_lead_master_hidden_deal_substitutions_keep_the_view_and_rule() {
    let vh = [3, 7, 4, 11, 14, 31, 26, 16, 23, 12];
    let d = deal(&vh, &[], &[]);
    let base = play(d, Suit::Herz, MH_DECLARES, &[]);
    // Pair 1: swap the two hidden hands; pair 2: swap one hidden card with the Skat.
    let (h0, h1, h2, skat) = (
        d.hand(Seat::Vorhand),
        d.hand(Seat::Mittelhand),
        d.hand(Seat::Hinterhand),
        d.skat(),
    );
    let swapped_hands = Deal::new([h0, h2, h1], skat).unwrap();
    let a = h1.iter().next().unwrap();
    let s = skat.iter().next().unwrap();
    let (mut n1, mut nsk) = (h1, skat);
    n1.remove(a);
    n1.insert(s);
    nsk.remove(s);
    nsk.insert(a);
    let swapped_skat = Deal::new([h0, n1, h2], nsk).unwrap();
    let v0 = base.view(Seat::Vorhand);
    for alt in [swapped_hands, swapped_skat] {
        let g = play(alt, Suit::Herz, MH_DECLARES, &[]);
        let v = g.view(Seat::Vorhand);
        assert_eq!(v, v0);
        assert_eq!(baseline::rule(&v), baseline::rule(&v0));
    }
    assert_eq!(base.phase(), Phase::Spiel);
}
