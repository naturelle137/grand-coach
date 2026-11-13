// SPDX-License-Identifier: EUPL-1.2

//! Core-legal guided rule fixtures and follow-class counterexamples.

#[path = "support/guided.rs"]
mod support;
use skat_coach::{Source, guided::*};
use skat_core::{Action, GameType, Seat, Suit};
use support::*;

#[test]
fn legal_declarer_and_defender_rules_have_independent_expected_candidates() {
    for declarer in [false, true] {
        let g = position(deal(), declarer);
        let view = g.view(g.current_seat().unwrap());
        let l = lesson(&view).unwrap();
        let (expected, branch, candidates) = if declarer {
            ("CQ", Branch::WinLast, vec![card("CQ"), card("CA")])
        } else {
            ("C7", Branch::UnderPartner, vec![card("C7"), card("C8")])
        };
        assert_eq!(l.rule.card, card(expected));
        assert_eq!(l.rule.branch, branch);
        assert_eq!(l.rule.candidates, candidates);
        assert_eq!(l, lesson(&view).unwrap());
        assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
        assert!(l.check(
            Step::Check,
            Answer::Seat(if declarer {
                None
            } else {
                Some(Seat::Hinterhand)
            })
        ));
        for c in &l.rule.candidates {
            assert!(l.check(Step::Narrow, Answer::Card(*c)));
        }
        for a in &view.legal_actions {
            let mut replay = g.clone();
            replay.apply(*a).unwrap(); // Every alternative, including CT, remains legal.
        }
    }
}

#[test]
fn hidden_ass_can_be_with_declarer_or_unknown_skat_without_changing_the_dialogue() {
    let a = position(deal(), false);
    let b = position(swap(deal(), card("CA"), card("DK")), false);
    let va = a.view(Seat::Mittelhand);
    let vb = b.view(Seat::Mittelhand);
    assert_eq!(va, vb);
    let l = lesson(&va).unwrap();
    assert_eq!(Some(l.clone()), lesson(&vb));
    assert!(l.memory().unseen.contains(card("CA")));
    assert_eq!(l.memory().skat, None);
}

#[test]
fn actual_follow_discard_and_kreuz_bube_have_public_follow_class_evidence() {
    // Move both plain Kreuz cards out of HH, allowing Trumpf or discard legally.
    let void_deal = swap(swap(deal(), card("CA"), card("H7")), card("CQ"), card("H8"));
    for (d, played, expected_void) in [
        (deal(), "CA", false),
        (void_deal, "D7", true),
        (
            swap(swap(deal(), card("CA"), card("H7")), card("CQ"), card("CJ")),
            "CJ",
            true,
        ),
    ] {
        let mut g = position(d, false);
        assert!(lesson(&g.view(Seat::Mittelhand)).is_some());
        g.apply(Action::Play(card("C7"))).unwrap();
        let events = g.apply(Action::Play(card(played))).unwrap();
        let winner = events.iter().find_map(|e| {
            if let skat_core::Event::TrickWon { trick } = e {
                Some(trick.winner)
            } else {
                None
            }
        });
        let o = observe(
            GameType::Farbe(Suit::Herz),
            Source::Trick {
                index: 0,
                seat: Seat::Hinterhand,
                led: card("CK"),
                card: card(played),
            },
            winner,
        )
        .unwrap();
        assert_eq!(o.proves_void, expected_void);
        assert!(o.winner.is_some());
    }
}

#[test]
fn excludes_side_master_feeding_early_wins_other_contracts_and_forced_moves() {
    // CK is now a follow-class master from MH's view, even though HH can trump it.
    let d = swap(deal(), card("CA"), card("S7"));
    let g = position(d, false);
    assert!(skat_analysis::Knowledge::from_view(&g.view(Seat::Mittelhand)).is_master(card("CK")));
    assert_eq!(
        skat_analysis::baseline::rule(&g.view(Seat::Mittelhand))
            .unwrap()
            .branch,
        Branch::FeedPartner
    );
    assert!(lesson(&g.view(Seat::Mittelhand)).is_none());
    let mut g = position(deal(), true);
    while g.current_seat().is_some() {
        let v = g.view(g.current_seat().unwrap());
        if v.legal_actions.len() == 1 {
            assert!(lesson(&v).is_none());
        }
        g.apply(v.legal_actions[0]).unwrap();
    }
    let g = skat_core::Game::new(0, skat_core::RuleConfig::default());
    assert!(lesson(&g.view(g.current_seat().unwrap())).is_none());
}

#[test]
fn partner_branch_with_no_losing_card_is_explicit_fallback_not_stay_under_advice() {
    let d = swap(swap(deal(), card("C7"), card("CA")), card("C8"), card("H7"));
    let g = position_led(d, false, "C9");
    let v = g.view(Seat::Mittelhand);
    assert_eq!(
        v.legal_actions,
        vec![Action::Play(card("CT")), Action::Play(card("CA"))]
    );
    let r = skat_analysis::baseline::rule(&v).unwrap();
    assert!(r.partner_ahead && !r.last && !r.table_master);
    assert!(r.losers.is_empty());
    assert_eq!(r.branch, Branch::Fallback);
    assert_eq!(r.card, card("CT")); // Original Easy fallback, not a losing card.
    assert!(lesson(&v).is_none());
}
