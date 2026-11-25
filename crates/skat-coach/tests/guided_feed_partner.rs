// SPDX-License-Identifier: EUPL-1.2

//! Fixture-only oracle replay. Expectations were handwritten before policy execution.
#[path = "support/feed_partner.rs"]
mod fixture;
use fixture::*;
use skat_analysis::{TrickState, baseline};
use skat_coach::guided::{Answer, Branch, Step, lesson, observe};
use skat_core::{Action, Seat};

#[test]
fn frozen_complete_deals_legal_order_every_completion_and_exact_feed_key() {
    for c in cases() {
        let g = c.position();
        let v = g.view(Seat::from_index(c.actor));
        assert_eq!(g.current_seat().unwrap().index(), c.actor, "{}", c.id);
        assert_eq!(v.declarer.unwrap().index(), c.declarer);
        let legal: Vec<_> = v
            .legal_actions
            .iter()
            .filter_map(|a| match a {
                Action::Play(c) => Some(c.index()),
                _ => None,
            })
            .collect();
        assert_eq!(legal, c.legal, "{} legal", c.id);
        assert_eq!(v.legal_actions.contains(&Action::Kontra), c.kontra);
        assert_eq!(
            TrickState::of(c.game, &v.current_trick)
                .best_seat
                .unwrap()
                .index(),
            c.winner
        );
        let mut winners = vec![];
        if !c.completed.is_empty() {
            for &n in c.legal {
                let mut next = g.clone();
                next.apply(Action::Play(card(n))).unwrap();
                let winner = next.tricks().last().unwrap().winner;
                winners.push(winner.index());
                let source = skat_coach::Source::Trick {
                    index: 0,
                    seat: v.seat,
                    led: v.current_trick.cards[0].unwrap(),
                    card: card(n),
                };
                let observed = observe(c.game, source, Some(winner)).unwrap();
                assert_eq!(observed.winner, Some(winner));
            }
            assert_eq!(winners, c.completed, "{} completions", c.id);
        }
        for &(candidate, replies, expected) in c.tree {
            let mut next = g.clone();
            next.apply(Action::Play(card(candidate))).unwrap();
            assert_eq!(
                next.legal_actions(),
                replies
                    .iter()
                    .map(|&n| Action::Play(card(n)))
                    .collect::<Vec<_>>()
            );
            for (&reply, &winner) in replies.iter().zip(expected) {
                let mut last = next.clone();
                last.apply(Action::Play(card(reply))).unwrap();
                assert_eq!(last.tricks().last().unwrap().winner.index(), winner);
            }
        }
        // Rule comparison is separate from the core replay above.
        let r = baseline::rule(&v);
        if let Some(choice) = c.choice {
            assert_eq!(r.as_ref().unwrap().card, card(choice), "{} choice", c.id);
        }
        for &(n, nontrump, augen) in c.keys {
            assert_eq!(
                (u8::from(!c.game.is_trump(card(n))), card(n).augen()),
                (nontrump, augen)
            );
        }
        let l = lesson(&v);
        assert_eq!(l, lesson(&v), "same permitted view is deterministic");
        let feed = l
            .as_ref()
            .is_some_and(|l| l.rule.branch == Branch::FeedPartner);
        // Keep #163's historical fixture metadata unchanged. These two
        // Farbspiel exclusions now belong to #185's independently gated
        // family; this updates current admission, not that historical oracle.
        let suit_now_supported = matches!(c.id, "A-suit" | "A-suit-corrected");
        let early_now_supported = c.id == "B-early-master-can-be-trumped";
        let ouvert_now_supported = c.id == "A-ouvert";
        // The Null fallback for a last-seat defender behind the partner (#131).
        let null_now_supported = c.id == "A-null";
        assert_eq!(
            feed,
            c.supported || suit_now_supported || early_now_supported || ouvert_now_supported,
            "{} applicability",
            c.id
        );
        if suit_now_supported {
            assert_eq!(l.as_ref().unwrap().rule.card, card(7));
        }
        if c.id == "B-declarer-ahead" {
            assert_eq!(l.as_ref().unwrap().rule.branch, Branch::WinLast);
        } else if !c.supported
            && !suit_now_supported
            && !early_now_supported
            && !ouvert_now_supported
            && !null_now_supported
        {
            assert!(l.is_none(), "{} exclusion", c.id);
        }
        if feed {
            let l = l.unwrap();
            assert_eq!(l.remaining, early_now_supported.then_some(Seat::Hinterhand));
            assert!(l.rule.partner_ahead);
            assert_eq!(l.rule.last, !early_now_supported);
            assert_eq!(
                l.rule.candidates,
                c.legal.iter().map(|&n| card(n)).collect::<Vec<_>>()
            );
            assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::from_index(c.winner)))));
            assert!(l.check(Step::Check, Answer::Seat(l.remaining)));
            for &n in c.legal {
                assert!(l.check(Step::Narrow, Answer::Card(card(n))));
            }
            assert_eq!(l.rule.tie_key.2, 0, "no power criterion");
            assert_eq!(l.rule.tie_break, baseline::TieBreak::FeedLast);
            if let Some(&(_, nontrump, augen)) =
                c.keys.iter().find(|&&(n, _, _)| Some(n) == c.choice)
            {
                assert_eq!(l.rule.tie_key, (i32::from(nontrump), i32::from(augen), 0));
            }
        }
    }
}

#[test]
fn hidden_permutation_preserves_the_entire_permitted_lesson() {
    let cs = cases();
    let a = cs
        .iter()
        .find(|c| c.id == "B-void-nontrump-priority")
        .unwrap()
        .position();
    let b = cs
        .iter()
        .find(|c| c.id == "B-hidden-permutation")
        .unwrap()
        .position();
    let (a, b) = (a.view(Seat::Hinterhand), b.view(Seat::Hinterhand));
    assert_eq!(a, b);
    assert_eq!(lesson(&a), lesson(&b));
}

#[test]
fn ouvert_feed_requires_consistent_reveal_and_acting_view() {
    let c = cases().into_iter().find(|c| c.id == "A-ouvert").unwrap();
    let v = c.position().view(Seat::Hinterhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l.rule.branch, Branch::FeedPartner);
    // Kreuz 8, 9, Ass: the Ass has most Augen and overtakes partner's 10.
    assert_eq!(l.rule.card, card(7));
    assert_eq!(l.rule.candidates, vec![card(1), card(2), card(7)]);
    // Swapping the partner's unplayed card with the unknown Skat preserves
    // the whole permitted view, including the revealed declarer hand.
    let mut swapped = cases().into_iter().find(|c| c.id == "A-ouvert").unwrap();
    std::mem::swap(&mut swapped.hands[1][1], &mut swapped.skat[0]);
    let same = swapped.position().view(Seat::Hinterhand);
    assert_eq!(v, same);
    assert_eq!(lesson(&v), lesson(&same));
    assert_eq!(l.knowledge.known_elsewhere, v.declarer_cards.unwrap());
    let mut bad = v.clone();
    bad.declarer_cards = None;
    assert!(lesson(&bad).is_none());
    bad = v.clone();
    bad.declarer_cards = Some(v.hand);
    assert!(lesson(&bad).is_none());
    bad = v.clone();
    bad.legal_actions.pop();
    assert!(lesson(&bad).is_none());
    bad = v.clone();
    bad.current_seat = Some(Seat::Mittelhand);
    assert!(lesson(&bad).is_none());
    bad = v.clone();
    bad.contract.as_mut().unwrap().schwarz = false;
    assert!(lesson(&bad).is_none());
}

#[test]
fn suit_ouvert_last_defender_feed_checks_reveal_and_every_legal_card() {
    let mut c = cases().into_iter().find(|c| c.id == "A-suit").unwrap();
    c.ouvert = true;
    let g = c.position();
    let v = g.view(Seat::Hinterhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l.rule.branch, Branch::FeedPartner);
    assert_eq!(l.rule.card, card(7));
    assert_eq!(
        l.rule.candidates,
        vec![card(1), card(2), card(7), card(12), card(20)]
    );
    for (&n, &winner) in c.legal.iter().zip(c.completed) {
        assert!(l.check(Step::Narrow, Answer::Card(card(n))));
        let mut next = g.clone();
        next.apply(Action::Play(card(n))).unwrap();
        assert_eq!(next.tricks().last().unwrap().winner.index(), winner);
    }
    std::mem::swap(&mut c.hands[1][1], &mut c.skat[0]);
    let same = c.position().view(Seat::Hinterhand);
    assert_eq!(v, same);
    assert_eq!(lesson(&v), lesson(&same));
    let mut declarer_ahead = cases().into_iter().find(|c| c.id == "A-suit").unwrap();
    declarer_ahead.ouvert = true;
    declarer_ahead.hands[0][0] = 7;
    declarer_ahead.hands[2][2] = 0;
    declarer_ahead.plays = &[7, 3];
    assert_eq!(
        lesson(&declarer_ahead.position().view(Seat::Hinterhand))
            .unwrap()
            .rule
            .branch,
        Branch::WinLast
    );
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.declarer_cards = Some(v.hand),
        |v| v.legal_actions.pop().map(|_| ()).unwrap(),
        |v| v.current_seat = Some(Seat::Mittelhand),
        |v| v.contract.as_mut().unwrap().schwarz = false,
    ] {
        let mut bad = v.clone();
        edit(&mut bad);
        assert!(lesson(&bad).is_none());
    }
}

#[test]
fn grand_ouvert_last_defender_behind_declarer_checks_reveal_and_every_legal_card() {
    // The declarer leads and Mittelhand follows low: the declarer is ahead.
    let mut c = cases().into_iter().find(|c| c.id == "A-suit").unwrap();
    c.game = skat_core::GameType::Grand;
    c.ouvert = true;
    // Kreuz Acht leads, Kreuz Sieben follows; Zehn, Neun and Ass can overtake.
    c.hands[0][0] = 1;
    c.hands[1][0] = 0;
    c.hands[2][0] = 3;
    c.plays = &[1, 0];
    let g = c.position();
    let v = g.view(Seat::Hinterhand);
    assert_eq!(
        TrickState::of(c.game, &v.current_trick).best_seat,
        v.declarer
    );
    let l = lesson(&v).expect("Grand Ouvert defender last behind the declarer");
    assert_eq!(l.rule.branch, Branch::WinLast);
    let legal: Vec<_> = v.legal_actions.clone();
    assert!(legal.len() >= 2);
    for a in &legal {
        let Action::Play(card) = *a else { panic!() };
        assert!(l.check(Step::Narrow, Answer::Card(card)));
        let mut next = g.clone();
        next.apply(*a).unwrap();
        assert_eq!(next.tricks().len(), 1);
    }
    // Unseen Skat and partner cards do not change the permitted lesson.
    std::mem::swap(&mut c.hands[1][1], &mut c.skat[0]);
    let same = c.position().view(Seat::Hinterhand);
    assert_eq!(lesson(&same), Some(l.clone()));
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.declarer_cards = Some(v.hand),
        |v| v.legal_actions.pop().map(|_| ()).unwrap(),
        |v| v.current_seat = Some(Seat::Mittelhand),
        |v| v.contract.as_mut().unwrap().schwarz = false,
    ] {
        let mut bad = v.clone();
        edit(&mut bad);
        assert_ne!(lesson(&bad), Some(l.clone()));
    }
}
