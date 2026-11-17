// SPDX-License-Identifier: EUPL-1.2

//! Non-Ouvert Farbspiel WinEarly admission, facts and whole-lesson privacy.

#[path = "../../skat-analysis/tests/common/suit_win_early.rs"]
mod fixture;
use skat_analysis::baseline::Branch;
use skat_coach::guided::*;
use skat_core::*;

#[test]
fn accepted_cases_nonabsent_lessons_candidates_conditions_and_all_observations() {
    for c in fixture::cases() {
        let v = c.game().view(c.view.seat);
        let l = lesson(&v);
        if c.rule.branch != Branch::WinEarly {
            assert!(l.is_none());
            continue;
        }
        let l = l.unwrap();
        assert_eq!(l.rule, c.rule);
        assert_eq!(l.knowledge, c.knowledge);
        assert_eq!(l.remaining, Some(v.seat.next()));
        assert_eq!(
            l.early_condition(),
            Some(if l.game.is_trump(l.rule.card) {
                EarlyCondition::Trumpf
            } else {
                EarlyCondition::FollowClassMaster
            })
        );
        assert!(l.check(Step::Orient, Answer::Seat(v.current_trick.leader)));
        assert!(l.check(Step::Check, Answer::Seat(Some(v.seat.next()))));
        for a in &v.legal_actions {
            let Action::Play(card) = a else {
                unreachable!()
            };
            assert_eq!(
                l.check(Step::Narrow, Answer::Card(*card)),
                c.rule.winners.contains(card)
            );
        }
        for (cards, winner, _) in c.paths {
            let o = observe(
                l.game,
                skat_coach::Source::Trick {
                    index: v.tricks.len() as u8,
                    seat: v.seat.next(),
                    led: cards[0],
                    card: cards[2],
                },
                Some(winner),
            )
            .unwrap();
            assert_eq!(o.winner, Some(winner));
            assert_eq!(
                o.proves_void,
                l.game.follow(cards[0]) != l.game.follow(cards[2])
            );
        }
    }
}

#[test]
fn named_exact_api_admission_boundaries_preserve_other_families() {
    for (id, v, admit) in fixture::mutations() {
        let l = lesson(&v);
        assert_eq!(
            l.as_ref().is_some_and(
                |l| matches!(l.game, GameType::Farbe(_)) && l.rule.branch == Branch::WinEarly
            ),
            admit,
            "{id}"
        );
        if id.contains("partner-ahead") {
            assert_eq!(l.unwrap().rule.branch, Branch::UnderPartner);
        }
    }
}

#[test]
fn both_positive_hidden_pairs_have_full_equal_nonabsent_lessons() {
    let cs = fixture::cases();
    for (a, b) in [(6, 7), (8, 9)] {
        let a = lesson(&cs[a].game().view(cs[a].view.seat)).unwrap();
        let b = lesson(&cs[b].game().view(cs[b].view.seat)).unwrap();
        assert_eq!(a, b);
    }
}
