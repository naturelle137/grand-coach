// SPDX-License-Identifier: EUPL-1.2

//! #169 whole-lesson scope and privacy, using the accepted legal oracle.
#[path = "../../skat-analysis/tests/null_middle_policy.rs"]
mod fixture;
use fixture::{card, cases};
use skat_coach::guided::{Answer, Step, lesson};
use skat_core::{Action, Seat};

#[test]
fn middle_scope_whole_lesson_and_every_candidate() {
    for c in cases() {
        let v = c.game().view(c.seat);
        let l = lesson(&v);
        // Historical #169 scope flags remain frozen. #176 now admits this
        // genuine opening lead using the already frozen complete shared rule.
        if c.id == "lead" {
            let l = l.expect("#176 opening lead");
            assert_eq!(Some(l.rule.clone()), c.expected);
            assert_eq!(
                l.source,
                skat_coach::Source::Lead {
                    index: 0,
                    seat: Seat::Vorhand
                }
            );
            assert_eq!(l.lead_declarer, Some(true));
            assert_eq!(l.remaining, None);
            assert!(l.check(
                Step::Orient,
                Answer::Lead {
                    seat: Seat::Vorhand,
                    declarer: true
                }
            ));
            for card in c.legal {
                assert!(l.check(Step::Check, Answer::Card(card)));
                assert_eq!(
                    l.check(Step::Narrow, Answer::Card(card)),
                    card == l.rule.card
                );
            }
            continue;
        }
        // #165's supported last-seat lesson remains available, outside #169.
        assert_eq!(l.is_some(), c.admitted || c.id == "last", "{}", c.id);
        if !c.admitted {
            continue;
        }
        let l = l.unwrap();
        assert_eq!(Some(l.rule.clone()), c.expected);
        assert_eq!(l.remaining, Some(Seat::from_index(c.seat.index() + 1)));
        assert!(!l.rule.last);
        assert!(l.check(Step::Orient, Answer::Seat(l.rule.winner)));
        assert!(l.check(Step::Check, Answer::Seat(l.remaining)));
        assert!(!l.check(Step::Check, Answer::Seat(None)));
        for a in c.legal {
            assert_eq!(
                l.check(Step::Narrow, Answer::Card(a)),
                l.rule.losers.contains(&a)
            );
        }
        assert_eq!(l.memory().skat, v.own_skat);
        assert!(l.memory().trumpf.is_none());
    }
}

#[test]
fn middle_non_absent_entire_lesson_privacy_including_known_pickup_skat() {
    let cs = cases();
    for (a, b) in [
        ("mixed-hand", "privacy-opponent-opponent"),
        ("mixed-hand", "privacy-opponent-unknown-Hand-Skat"),
        ("mixed-pickup", "privacy-pickup-opponents"),
    ] {
        let view = |id| {
            let c = cs.iter().find(|c| c.id == id).unwrap();
            c.game().view(c.seat)
        };
        let x = view(a);
        let y = view(b);
        assert_eq!(x, y);
        let l = lesson(&x).expect("non-absent new middle-seat lesson");
        assert_eq!(Some(l), lesson(&y));
    }
}

#[test]
fn middle_restricted_and_noncontiguous_views_reject_instead_of_teaching() {
    let c = cases().into_iter().find(|c| c.id == "mixed-hand").unwrap();
    let v = c.game().view(c.seat);
    type Edit = fn(&mut skat_core::PlayerView);
    let edits: &[Edit] = &[
        |v| v.legal_actions.push(Action::Kontra),
        |v| v.legal_actions = vec![Action::Play(card(0))],
        |v| v.legal_actions.reverse(),
        |v| v.current_trick.cards[0] = None,
        |v| v.current_trick.cards[2] = Some(card(2)),
        |v| v.current_trick.leader = None,
        |v| v.current_seat = Some(Seat::Hinterhand),
        |v| v.seat = Seat::Hinterhand,
        |v| v.declarer = None,
        |v| v.hand_game = None,
        |v| v.own_skat = Some(fixture::set(&[30, 31])),
        |v| v.contract.as_mut().unwrap().schwarz = true,
    ];
    for edit in edits {
        let mut x = v.clone();
        edit(&mut x);
        assert!(lesson(&x).is_none());
    }
}
