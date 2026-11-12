// SPDX-License-Identifier: EUPL-1.2

//! Independently specified legal fixtures for remaining-trumpf@1 and
//! proven-void@1 retrieval. ISkO following obligations (D-26): a failure to
//! follow proves absence of that class, never the location of another card.
mod common;
use common::*;
use skat_analysis::Knowledge;
use skat_analysis::counting::{QuestionKind, derive, retrieval_questions};
use skat_core::{Action, Announcement, Deal, Follow, Game, GameType, RuleConfig, Seat, Suit};

fn game(deal: Deal, kind: GameType, declarer: Seat) -> Game {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    let calls = if declarer == Seat::Vorhand {
        vec![Action::Bid(18), Action::Hold, Action::Pass, Action::Pass]
    } else {
        vec![Action::Bid(18), Action::Pass, Action::Pass]
    };
    for a in calls
        .into_iter()
        .chain([Action::Hand, Action::Announce(Announcement::plain(kind))])
    {
        g.apply(a).unwrap();
    }
    assert_eq!(g.declarer(), Some(declarer));
    g
}

fn count(g: &Game, seat: Seat, q: QuestionKind) -> Option<u8> {
    derive(&Knowledge::from_view(&g.view(seat)), q).map(|d| d.expected())
}

#[test]
fn herz_both_roles_public_trumpf_void_and_remaining_count() {
    let mut g = game(
        deal(
            "CJ SJ HA HT HK HQ H9 SA ST CA",
            "DJ H8 H7 SK SQ S9 CK CQ C9 DA",
            "HJ S8 S7 CT C8 C7 DT DK DQ D9",
            "D8 D7",
        ),
        GameType::Farbe(Suit::Herz),
        Seat::Vorhand,
    );
    let q = QuestionKind::ProvenVoids {
        class: Follow::Trumpf,
    };
    // HH actually has only HJ. Before failure to follow, no void is proven.
    for seat in Seat::ALL {
        assert_eq!(count(&g, seat, q), Some(0));
    }
    for card in ["CJ", "H7", "HJ", "HA", "H8"] {
        g.apply(Action::Play(c(card))).unwrap();
    }
    assert_eq!(count(&g, Seat::Vorhand, q), Some(0));
    g.apply(Action::Play(c("S7"))).unwrap();
    // HH failed on HA. VH and MH each see one other seat void in Trumpf;
    // HH does not count its own void. 5 Trumpf played, VH owns 5, MH owns 1.
    for (seat, voids, outside) in [
        (Seat::Vorhand, 1, 1),
        (Seat::Mittelhand, 1, 5),
        (Seat::Hinterhand, 0, 6),
    ] {
        assert_eq!(count(&g, seat, q), Some(voids));
        assert_eq!(count(&g, seat, QuestionKind::TrumpfOutside), Some(outside));
    }
    // Herz is Trumpf, never an additional side-suit class.
    assert_eq!(
        count(
            &g,
            Seat::Vorhand,
            QuestionKind::ProvenVoids {
                class: Follow::Farbe(Suit::Herz)
            }
        ),
        None
    );
}

#[test]
fn grand_and_null_voids_exclude_the_viewer_and_require_public_evidence() {
    let d = deal(
        "CJ SJ HJ DJ CA CT CK CQ C9 C8",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 DA DT DK DQ D9 D8",
        "C7 D7",
    );
    let q = QuestionKind::ProvenVoids {
        class: Follow::Farbe(Suit::Kreuz),
    };
    for kind in [GameType::Grand, GameType::Null] {
        let mut g = game(d, kind, Seat::Mittelhand);
        let k = Knowledge::from_view(&g.view(Seat::Vorhand));
        // Both other hands are actually Kreuz-free, but not yet proven.
        assert_eq!(derive(&k, q).unwrap().expected(), 0);
        assert!(!retrieval_questions(&k).contains(&q));
        // C8 wins; MH and HH fail Kreuz. In Null MH declarer loses no trick.
        for card in ["C8", "S7", "H7"] {
            g.apply(Action::Play(c(card))).unwrap();
        }
        for (seat, expected) in [
            (Seat::Vorhand, 2),
            (Seat::Mittelhand, 1),
            (Seat::Hinterhand, 1),
        ] {
            assert_eq!(count(&g, seat, q), Some(expected));
            let k = Knowledge::from_view(&g.view(seat));
            assert!(retrieval_questions(&k).contains(&q));
        }
        if kind == GameType::Null {
            assert_eq!(count(&g, Seat::Vorhand, QuestionKind::TrumpfOutside), None);
            assert_eq!(
                count(
                    &g,
                    Seat::Vorhand,
                    QuestionKind::ProvenVoids {
                        class: Follow::Trumpf
                    }
                ),
                None
            );
        } else {
            // All four Buben remain with VH; MH holds none, Skat is unknown.
            assert_eq!(
                count(&g, Seat::Vorhand, QuestionKind::TrumpfOutside),
                Some(0)
            );
            assert_eq!(
                count(&g, Seat::Mittelhand, QuestionKind::TrumpfOutside),
                Some(4)
            );
        }
    }
}
