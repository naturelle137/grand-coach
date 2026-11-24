// SPDX-License-Identifier: EUPL-1.2

//! Grand current-second feeding: independent #163 core witnesses, newly admitted.
#[allow(dead_code)]
#[path = "support/feed_partner.rs"]
mod fixture;
use fixture::*;
use skat_coach::guided::{Answer, Branch, Step, lesson, observe};
use skat_core::{Action, GameType, Seat};

fn case() -> Case {
    cases()
        .into_iter()
        .find(|c| c.id == "B-early-master-can-be-trumped")
        .unwrap()
}

#[test]
fn grand_ouvert_early_feed_requires_reveal_and_observes_trump() {
    let mut c = case();
    c.ouvert = true;
    let game = c.position();
    let view = game.view(Seat::Mittelhand);
    let guided = lesson(&view).unwrap();
    assert_eq!(guided.rule.branch, Branch::FeedPartner);
    assert_eq!(guided.rule.candidates, vec![card(3), card(5)]);
    assert_eq!(guided.rule.card, card(3));
    for own in [3, 5] {
        for reply in [4, 12, 16] {
            let mut after = game.clone();
            after.apply(Action::Play(card(own))).unwrap();
            after.apply(Action::Play(card(reply))).unwrap();
            assert_eq!(
                after.tricks()[0].winner,
                if reply == 16 {
                    Seat::Vorhand
                } else {
                    Seat::Hinterhand
                }
            );
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

#[test]
fn early_feed_every_reply_has_a_handwritten_individual_winner() {
    let c = case();
    let g = c.position();
    let v = g.view(Seat::Mittelhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l, lesson(&v).unwrap());
    assert_eq!(l.game, GameType::Grand);
    assert_eq!(l.rule.branch, Branch::FeedPartner);
    assert!(l.rule.partner_ahead && l.rule.table_master && !l.rule.last);
    assert_eq!(l.rule.winner, Some(Seat::Vorhand));
    assert_eq!(l.remaining, Some(Seat::Hinterhand));
    assert_eq!(l.rule.card, card(3)); // CT's ten Augen beat CQ's three.
    assert_eq!(l.rule.candidates, vec![card(3), card(5)]);
    assert_eq!(l.rule.losers, l.rule.candidates);
    assert!(l.rule.winners.is_empty());
    assert_eq!(l.rule.tie_key, (1, 10, 0));
    assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
    assert!(!l.check(Step::Check, Answer::Seat(None)));
    assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
    for &(chosen, replies, winners) in c.tree {
        assert!(l.check(Step::Narrow, Answer::Card(card(chosen))));
        let mut after = g.clone();
        after.apply(Action::Play(card(chosen))).unwrap();
        assert_eq!(
            after.legal_actions(),
            replies
                .iter()
                .map(|&n| Action::Play(card(n)))
                .collect::<Vec<_>>()
        );
        for (&reply, &winner) in replies.iter().zip(winners) {
            let mut end = after.clone();
            end.apply(Action::Play(card(reply))).unwrap();
            assert_eq!(end.tricks()[0].winner.index(), winner);
            let o = observe(
                l.game,
                skat_coach::Source::Trick {
                    index: 0,
                    seat: Seat::Hinterhand,
                    led: card(7),
                    card: card(reply),
                },
                Some(end.tricks()[0].winner),
            )
            .unwrap();
            assert!(o.proves_void); // All ten replies are outside plain Kreuz.
            assert_eq!(o.winner, Some(Seat::from_index(winner)));
            assert_eq!(winner, if [4, 12].contains(&reply) { 2 } else { 0 });
        }
    }
}

#[test]
fn early_feed_entire_lesson_and_memory_ignore_hidden_holders() {
    let a = case();
    let mut b = case();
    // Swap unseen partner/declarer cards; then unseen declarer/Skat cards.
    b.hands[0][3] = 16;
    b.hands[2][2] = 13;
    b.hands[2][3] = 20;
    b.skat[0] = 17;
    let (a, b) = (
        a.position().view(Seat::Mittelhand),
        b.position().view(Seat::Mittelhand),
    );
    assert_eq!(a, b);
    let l = lesson(&a).unwrap();
    assert_eq!(Some(l.clone()), lesson(&b));
    assert_eq!(l.memory(), lesson(&b).unwrap().memory());
}

#[test]
fn early_feed_requires_exact_acting_defender_and_grand_master() {
    let v = case().position().view(Seat::Mittelhand);
    assert!(lesson(&v).is_some());
    for n in 0..9 {
        let mut x = v.clone();
        match n {
            0 => x.contract.as_mut().unwrap().ouvert = true,
            1 => x.contract.as_mut().unwrap().game = GameType::Null,
            2 => x.current_seat = Some(Seat::Vorhand),
            3 => x.current_trick.leader = Some(Seat::Hinterhand),
            4 => x.legal_actions.reverse(),
            5 => x.legal_actions.truncate(1),
            6 => x.legal_actions.push(Action::Kontra),
            7 => x.hand_game = Some(false),
            8 => x.own_skat = Some([card(20), card(30)].into_iter().collect()),
            _ => unreachable!(),
        }
        if n == 1 {
            assert_eq!(lesson(&x).unwrap().rule.branch, Branch::Null);
        } else {
            assert!(lesson(&x).is_none(), "mutation {n}");
        }
    }
    let c = cases()
        .into_iter()
        .find(|c| c.id == "A-early-partner-ahead")
        .unwrap();
    assert!(lesson(&c.position().view(Seat::Mittelhand)).is_none());
}

#[test]
fn early_feed_master_can_be_bube_and_own_higher_card_can_overtake_partner() {
    let mut c = case();
    // CJ leads; MH must follow with HJ or DJ (both two Augen). FeedLast
    // selects the last exact tie, DJ, despite HJ's higher power.
    c.hands[0][5] = 4;
    c.hands[2][0] = 7;
    c.hands[1][0] = 20;
    c.skat[0] = 3;
    c.plays = &[4];
    let g = c.position();
    let l = lesson(&g.view(Seat::Mittelhand)).unwrap();
    assert_eq!(l.rule.candidates, vec![card(20), card(28)]);
    assert_eq!(l.rule.card, card(28));
    assert_eq!(l.rule.tie_key, (0, 2, 0));
    for n in [20, 28] {
        let mut end = g.clone();
        end.apply(Action::Play(card(n))).unwrap();
        assert_eq!(end.legal_actions(), vec![Action::Play(card(12))]);
        end.apply(Action::Play(card(12))).unwrap();
        assert_eq!(end.tricks()[0].winner, Seat::Vorhand);
    }
    // CK leads; CA is in the learner's own hand, so it does not block the
    // conservative table-master predicate. Both legal cards remain candidates.
    let mut c = case();
    c.hands[0][5] = 5;
    c.hands[1][1] = 7;
    c.plays = &[6];
    let l = lesson(&c.position().view(Seat::Mittelhand)).unwrap();
    assert!(l.rule.table_master);
    assert_eq!(l.rule.candidates, vec![card(3), card(7)]);
    assert_eq!(l.rule.card, card(7));
    assert_eq!(l.rule.winners, vec![card(3), card(7)]);
}
