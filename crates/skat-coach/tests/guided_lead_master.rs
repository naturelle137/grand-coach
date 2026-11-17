// SPDX-License-Identifier: EUPL-1.2

//! Whole #171 lessons, factual predicate, scope and non-absent privacy.
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/lead_master_policy.rs"]
mod fixture;
use fixture::{card, cases};
use skat_coach::{
    Source,
    guided::{Answer, Step, lesson},
};
use skat_core::{Action, Seat};

#[test]
fn lead_master_whole_lesson_role_master_attempt_and_memory() {
    for c in cases() {
        let v = c.game().view(c.seat);
        let l = lesson(&v);
        assert_eq!(l.is_some(), c.admitted(), "{}", c.id);
        if !c.admitted() {
            continue;
        }
        let l = l.unwrap();
        assert_eq!(l.rule, c.expected);
        assert_eq!(
            l.source,
            Source::Lead {
                index: v.tricks.len() as u8,
                seat: c.seat
            }
        );
        assert_eq!(l.rule.winner, None);
        assert_eq!(l.remaining, None); // Two replies, not the old single remaining seat.
        assert!(l.check(
            Step::Orient,
            Answer::Lead {
                seat: c.seat,
                declarer: v.declarer == Some(c.seat)
            }
        ));
        assert!(!l.check(Step::Orient, Answer::Seat(None)));
        for a in &v.legal_actions {
            let Action::Play(card) = a else { panic!() };
            for step in [Step::Check, Step::Narrow] {
                assert_eq!(
                    l.check(step, Answer::Card(*card)),
                    if step == Step::Check {
                        c.expected.candidates.contains(card)
                    } else {
                        *card == c.expected.card
                    }
                );
            }
        }
        assert_eq!(l.memory().skat, v.own_skat);
    }
}

#[test]
fn lead_master_two_entire_non_absent_hidden_substitutions() {
    let cs = cases();
    for b in ["02", "01-hidden-swap"] {
        let view = |id| {
            let c = cs.iter().find(|c| c.id == id).unwrap();
            c.game().view(c.seat)
        };
        let x = view("01");
        let y = view(b);
        assert_eq!(x, y);
        let l = lesson(&x).expect("new lead lesson, never None equality");
        assert_eq!(Some(l), lesson(&y));
    }
}

#[test]
fn lead_master_rejects_malformed_restricted_and_other_contract_boundaries() {
    let c = cases().remove(0);
    let v = c.game().view(c.seat);
    type Edit = fn(&mut skat_core::PlayerView);
    let edits: &[Edit] = &[
        |v| v.legal_actions.push(Action::Kontra),
        |v| {
            v.legal_actions.remove(0);
        },
        |v| v.legal_actions.reverse(),
        |v| v.legal_actions.push(Action::Play(card(1))),
        |v| v.current_trick.cards[1] = Some(card(1)),
        |v| v.current_trick.cards[2] = Some(card(1)),
        |v| v.current_trick.leader = Some(v.seat),
        |v| v.current_seat = Some(Seat::Hinterhand),
        |v| v.current_seat = None,
        |v| v.declarer = None,
        |v| v.hand_game = None,
        |v| v.own_skat = Some(fixture::set(&[17, 29])),
        |v| v.contract = None,
        |v| v.contract.as_mut().unwrap().ouvert = true,
        |v| v.contract.as_mut().unwrap().game = skat_core::GameType::Null,
        |v| v.contract.as_mut().unwrap().game = skat_core::GameType::Farbe(skat_core::Suit::Herz),
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut x = v.clone();
        edit(&mut x);
        assert!(lesson(&x).is_none(), "edit{i}");
    }
    let mut g = c.game();
    g.apply(Action::Play(card(3))).unwrap();
    assert!(lesson(&g.view(g.current_seat().unwrap())).is_none());
}
