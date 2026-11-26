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
        assert_eq!(
            l.is_some(),
            c.admitted || c.id == "last" || c.id == "ouvert" || c.id == "no-loser",
            "{}",
            c.id
        );
        if !c.admitted && c.id != "ouvert" && c.id != "no-loser" {
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
                if l.rule.losers.is_empty() {
                    a == l.rule.card
                } else {
                    l.rule.losers.contains(&a)
                }
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
fn no_loser_hidden_skat_swap_preserves_entire_lesson() {
    let mut c = cases().into_iter().find(|c| c.id == "no-loser").unwrap();
    let original = c.game().view(c.seat);
    c.deal = skat_core::Deal::new(
        [
            fixture::set(&[0, 5, 10, 11, 12, 13, 18, 19, 26, 27]),
            fixture::set(&[3, 4, 6, 7, 8, 9, 16, 17, 24, 25]),
            fixture::set(&[2, 14, 15, 20, 21, 22, 23, 28, 29, 30]),
        ],
        fixture::set(&[1, 31]),
    )
    .unwrap();
    let changed = c.game().view(c.seat);
    assert_eq!(original, changed);
    let original_lesson = lesson(&original).expect("no-loser lesson");
    assert_eq!(Some(original_lesson), lesson(&changed));
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

#[test]
fn ouvert_middle_requires_the_revealed_own_hand_and_exact_actions() {
    let mut c = cases().into_iter().find(|c| c.id == "mixed-hand").unwrap();
    for action in &mut c.history {
        if let Action::Announce(announcement) = action {
            announcement.ouvert = true;
        }
    }
    let v = c.game().view(c.seat);
    assert_eq!(v.declarer_cards, Some(v.hand));
    let l = lesson(&v).unwrap();
    assert_eq!(Some(l.rule.clone()), c.expected);
    for card in c.legal {
        assert_eq!(
            l.check(Step::Narrow, Answer::Card(card)),
            l.rule.losers.contains(&card)
        );
    }
    for edit in 0..4 {
        let mut bad = v.clone();
        match edit {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(fixture::set(&[0])),
            2 => {
                bad.legal_actions.pop();
            }
            _ => bad.current_trick.cards[0] = None,
        }
        assert!(lesson(&bad).is_none());
    }
}
