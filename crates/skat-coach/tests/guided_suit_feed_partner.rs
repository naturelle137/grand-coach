// SPDX-License-Identifier: EUPL-1.2
//! Strict new-family admission and whole permitted-view lesson privacy.
#[path = "../../skat-analysis/tests/common/suit_feed_partner.rs"]
mod fixture;
use skat_coach::guided::*;
use skat_core::*;

#[test]
fn suit_feed_partner_admission_every_candidate_and_actual_individual_observation() {
    for c in fixture::cases() {
        let l = lesson(&c.game().view(c.view.seat)).unwrap();
        assert_eq!(l.rule, c.rule);
        assert_eq!(l.knowledge, c.knowledge);
        assert_eq!(l.remaining, None);
        assert_eq!(l.early_condition(), None);
        assert!(l.check(Step::Orient, Answer::Seat(c.rule.winner)));
        assert!(l.check(Step::Check, Answer::Seat(None)));
        for (card, winner) in c.completions {
            assert!(l.check(Step::Narrow, Answer::Card(card)));
            let o = observe(
                l.game,
                skat_coach::Source::Trick {
                    index: c.view.tricks.len() as u8,
                    seat: c.view.seat,
                    led: c.view.current_trick.led().unwrap(),
                    card,
                },
                Some(winner),
            )
            .unwrap();
            assert_eq!(o.winner, Some(winner));
            assert_eq!(
                o.proves_void,
                l.game.follow(card) != l.game.follow(c.view.current_trick.led().unwrap())
            );
        }
    }
}
#[test]
fn suit_feed_partner_all_full_api_boundaries_and_grand_preserved() {
    for (id, v, admit) in fixture::mutations() {
        let l = lesson(&v); // Missing declarer must return, NOT panic/catch_unwind.
        assert_eq!(
            l.as_ref()
                .is_some_and(|l| matches!(l.game, GameType::Farbe(_))
                    && l.rule.branch == Branch::FeedPartner),
            admit,
            "{id}"
        );
        if id == "Grand" {
            assert_eq!(l.unwrap().rule.branch, Branch::FeedPartner);
        }
    }
}
#[test]
fn suit_feed_partner_both_hidden_pairs_entire_nonabsent_lessons_equal() {
    let cs = fixture::cases();
    for (a, b) in [(0, 6), (3, 7)] {
        assert_eq!(cs[a].view, cs[b].view);
        let x = lesson(&cs[a].view).unwrap();
        let y = lesson(&cs[b].view).unwrap();
        assert_eq!(x, y);
    }
}
