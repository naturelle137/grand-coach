// SPDX-License-Identifier: EUPL-1.2

//! Farbspiel current-second feeding: the #163 full deal reused as a Herz game.
#[allow(dead_code)]
#[path = "support/feed_partner.rs"]
mod fixture;
use fixture::*;
use skat_coach::guided::{Answer, Branch, Step, lesson};
use skat_core::{Action, GameType, Seat, Suit};

fn case() -> Case {
    let mut c = cases()
        .into_iter()
        .find(|c| c.id == "B-early-master-can-be-trumped")
        .unwrap();
    c.game = GameType::Farbe(Suit::Herz);
    c
}

#[test]
fn suit_early_feed_admits_partner_master_and_declarer_may_trump() {
    let g = case().position();
    let v = g.view(Seat::Mittelhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l, lesson(&v).unwrap());
    assert_eq!(l.game, GameType::Farbe(Suit::Herz));
    assert_eq!(l.rule.branch, Branch::FeedPartner);
    assert!(l.rule.partner_ahead && l.rule.table_master && !l.rule.last);
    assert_eq!(l.rule.winner, Some(Seat::Vorhand));
    assert_eq!(l.remaining, Some(Seat::Hinterhand));
    assert_eq!(l.rule.candidates, vec![card(3), card(5)]);
    assert_eq!(l.rule.card, card(3)); // CT's ten Augen beat CQ's three.
    assert_eq!(l.rule.tie_key, (1, 10, 0));
    assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
    assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
    for n in [3, 5] {
        assert!(l.check(Step::Narrow, Answer::Card(card(n))));
        // The declarer's Herz trumps the side master; Kreuz-Bube too.
        for (reply, winner) in [(16, 2), (4, 2)] {
            let mut end = g.clone();
            end.apply(Action::Play(card(n))).unwrap();
            end.apply(Action::Play(card(reply))).unwrap();
            assert_eq!(end.tricks()[0].winner.index(), winner);
        }
    }
}

#[test]
fn suit_early_feed_ignores_hidden_holders_and_requires_exact_input() {
    let a = case();
    let mut b = case();
    b.hands[0][3] = 16;
    b.hands[2][2] = 13;
    b.hands[2][3] = 20;
    b.skat[0] = 17;
    let (va, vb) = (
        a.position().view(Seat::Mittelhand),
        b.position().view(Seat::Mittelhand),
    );
    assert_eq!(va, vb);
    assert_eq!(lesson(&va), lesson(&vb));
    for n in 0..8 {
        let mut x = va.clone();
        match n {
            0 => x.contract.as_mut().unwrap().ouvert = true,
            1 => x.current_seat = Some(Seat::Vorhand),
            2 => x.current_trick.leader = Some(Seat::Hinterhand),
            3 => x.legal_actions.reverse(),
            4 => x.legal_actions.truncate(1),
            5 => x.legal_actions.push(Action::Kontra),
            6 => x.hand_game = Some(false),
            7 => x.own_skat = Some([card(20), card(30)].into_iter().collect()),
            _ => unreachable!(),
        }
        assert!(lesson(&x).is_none(), "mutation {n}");
    }
}

#[test]
fn ouvert_early_feed_admits_consistent_reveal_and_observes_trumping() {
    let mut c = case();
    c.ouvert = true;
    let game = c.position();
    let view = game.view(Seat::Mittelhand);
    let guided = lesson(&view).unwrap();
    assert_eq!(guided.rule.branch, Branch::FeedPartner);
    assert_eq!(guided.rule.candidates, vec![card(3), card(5)]);
    assert_eq!(guided.rule.card, card(3));
    for own in [3, 5] {
        for reply in [16, 4] {
            let mut after = game.clone();
            after.apply(Action::Play(card(own))).unwrap();
            after.apply(Action::Play(card(reply))).unwrap();
            assert_eq!(after.tricks()[0].winner, Seat::Hinterhand);
        }
    }
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.declarer_cards = Some(v.hand),
        |v| v.contract.as_mut().unwrap().schwarz = false,
        |v| v.legal_actions.reverse(),
        |v| v.current_seat = Some(Seat::Vorhand),
    ] {
        let mut bad = view.clone();
        edit(&mut bad);
        assert!(lesson(&bad).is_none());
    }
}
