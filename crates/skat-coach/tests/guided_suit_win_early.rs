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
            // Only the forced second-seat overtake fallback is a lesson.
            let forced = c.rule.branch == Branch::Fallback
                && c.rule.losers.is_empty()
                && !c.rule.partner_ahead;
            assert_eq!(l.is_some(), forced);
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

#[test]
fn ouvert_declarer_second_requires_revealed_hand_and_complete_legal_view() {
    let mut c = fixture::cases().remove(10);
    for action in &mut c.history {
        if let Action::Announce(a) = action {
            a.ouvert = true;
            a.schneider = true;
            a.schwarz = true;
        }
    }
    let g = c.game();
    let v = g.view(Seat::Hinterhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l.rule.branch, Branch::WinEarly);
    assert_eq!(l.rule, skat_analysis::baseline::rule(&v).unwrap());
    let mut pair_found = false;
    for hidden in c.deal.hand(Seat::Vorhand).iter() {
        for skat in c.deal.skat().iter() {
            let mut hands = Seat::ALL.map(|seat| c.deal.hand(seat));
            let mut buried = c.deal.skat();
            hands[0].remove(hidden);
            hands[0].insert(skat);
            buried.remove(skat);
            buried.insert(hidden);
            let deal = Deal::new(hands, buried).unwrap();
            let mut other = Game::from_deal(deal, RuleConfig::default());
            if c.history.iter().all(|&a| other.apply(a).is_ok()) {
                let other_view = other.view(Seat::Hinterhand);
                if other_view == v {
                    assert_eq!(lesson(&other_view), Some(l.clone()));
                    pair_found = true;
                    break;
                }
            }
        }
        if pair_found {
            break;
        }
    }
    assert!(pair_found);
    for change in 0..6 {
        let mut bad = v.clone();
        match change {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(CardSet::EMPTY),
            2 => bad.contract.as_mut().unwrap().schwarz = false,
            3 => bad.contract.as_mut().unwrap().schneider = false,
            4 => {
                bad.legal_actions.pop();
            }
            _ => bad.current_seat = Some(Seat::Vorhand),
        }
        assert!(!lesson(&bad).is_some_and(|l| l.rule.branch == Branch::WinEarly));
    }
}
