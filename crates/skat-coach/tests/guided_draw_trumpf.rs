// SPDX-License-Identifier: EUPL-1.2

//! Grand declarer `DrawTrumpf` own-lead lessons (#175). The legal deal and
//! history of case 13 are handwritten (Vorhand declarer, pickup, discard of
//! two non-Buben); the Easy card is the highest-power own Bube, here the
//! first of three, never a claim about defenders' holdings.
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/lead_master_policy.rs"]
mod fixture;
use fixture::{card, cases, set};
use skat_analysis::baseline::Branch;
use skat_coach::{
    Source,
    guided::{Answer, Step, lesson},
};
use skat_core::{Action, PlayerView, Seat};

fn case13() -> fixture::Case {
    cases()
        .into_iter()
        .find(|c| c.id == "13-postfreeze-actual-DrawTrumpf")
        .unwrap()
}

#[test]
fn draw_trumpf_whole_lesson_role_candidates_attempt_and_memory() {
    let c = case13();
    let v = c.game().view(c.seat);
    let l = lesson(&v).expect("DrawTrumpf lesson");
    assert_eq!(l.rule, c.expected);
    assert_eq!(l.rule.branch, Branch::DrawTrumpf);
    assert_eq!(
        l.source,
        Source::Lead {
            index: 0,
            seat: c.seat
        }
    );
    assert_eq!((l.rule.winner, l.remaining), (None, None));
    assert!(l.check(
        Step::Orient,
        Answer::Lead {
            seat: c.seat,
            declarer: true
        }
    ));
    assert!(!l.check(
        Step::Orient,
        Answer::Lead {
            seat: c.seat,
            declarer: false
        }
    ));
    // Every legal card is answerable; only own Buben are candidates and only
    // Easy's card is the attempt.
    for a in &v.legal_actions {
        let Action::Play(x) = a else { panic!() };
        assert_eq!(
            l.check(Step::Check, Answer::Card(*x)),
            [4, 12, 20].map(card).contains(x)
        );
        assert_eq!(l.check(Step::Narrow, Answer::Card(*x)), *x == card(4));
    }
    let m = l.memory();
    assert_eq!(m.skat, v.own_skat);
    assert_eq!(m.own, v.hand);
}

#[test]
fn draw_trumpf_two_hidden_substitutions_keep_the_same_permitted_view() {
    let c = case13();
    let v = c.game().view(c.seat);
    let l = lesson(&v).unwrap();
    // Exchange one non-Bube card between the defenders' hidden hands; the
    // learner's hand, pickup and discard are unchanged.
    for (a, b) in [(0, 18), (2, 19)] {
        let (a, b) = (card(a), card(b));
        let swap = |mut h: skat_core::CardSet| {
            if h.remove(a) {
                h.insert(b);
            } else if h.remove(b) {
                h.insert(a);
            }
            h
        };
        let d = skat_core::Deal::new(
            [
                c.deal.hand(Seat::Vorhand),
                swap(c.deal.hand(Seat::Mittelhand)),
                swap(c.deal.hand(Seat::Hinterhand)),
            ],
            c.deal.skat(),
        )
        .unwrap();
        assert_ne!(d, c.deal);
        let other = fixture::Case {
            deal: d,
            ..case13()
        };
        let w = other.game().view(c.seat);
        assert_eq!(w, v);
        assert_eq!(lesson(&w), Some(l.clone()));
    }
}

#[test]
fn draw_trumpf_rejects_defender_malformed_and_other_boundaries() {
    let c = case13();
    let g = c.game();
    // Defenders are not entitled to the lead.
    for s in [Seat::Mittelhand, Seat::Hinterhand] {
        assert!(lesson(&g.view(s)).is_none());
    }
    let v = g.view(c.seat);
    type Edit = fn(&mut PlayerView);
    let edits: &[Edit] = &[
        |v| v.legal_actions.push(Action::Kontra),
        |v| {
            v.legal_actions.remove(0);
        },
        |v| v.legal_actions.reverse(),
        |v| v.current_trick.cards[1] = Some(card(1)),
        |v| v.current_trick.leader = Some(v.seat),
        |v| v.current_seat = None,
        |v| v.declarer = None,
        |v| v.hand_game = None,
        |v| v.own_skat = None,
        |v| v.contract.as_mut().unwrap().ouvert = true,
        |v| v.hand &= !set(&[4, 12, 20]),
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut x = v.clone();
        edit(&mut x);
        assert!(lesson(&x).is_none(), "edit{i}");
    }
}
