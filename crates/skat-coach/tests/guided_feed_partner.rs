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
        assert_eq!(
            feed,
            c.supported || suit_now_supported,
            "{} applicability",
            c.id
        );
        if suit_now_supported {
            assert_eq!(l.as_ref().unwrap().rule.card, card(7));
        }
        if c.id == "B-declarer-ahead" {
            assert_eq!(l.as_ref().unwrap().rule.branch, Branch::WinLast);
        } else if !c.supported && !suit_now_supported {
            assert!(l.is_none(), "{} exclusion", c.id);
        }
        if feed {
            let l = l.unwrap();
            assert_eq!(l.remaining, None);
            assert!(l.rule.partner_ahead && l.rule.last);
            assert_eq!(
                l.rule.candidates,
                c.legal.iter().map(|&n| card(n)).collect::<Vec<_>>()
            );
            assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::from_index(c.winner)))));
            assert!(l.check(Step::Check, Answer::Seat(None)));
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
