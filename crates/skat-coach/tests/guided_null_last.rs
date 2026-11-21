// SPDX-License-Identifier: EUPL-1.2

//! Fixture-only oracle replay for the last-seat Null declarer lesson (#165).
//! Expectations were handwritten and frozen before policy execution (QA1458);
//! the core replay below checks them independently of the shared rule.
#[path = "support/null_last.rs"]
mod fixture;
use fixture::*;
use skat_analysis::{TrickState, baseline, hand::power};
use skat_coach::guided::{Answer, Branch, Step, lesson, observe};
use skat_core::{Action, Contract, GameType, PlayerView, Seat, Trick};

fn legal(v: &PlayerView) -> Vec<skat_core::Card> {
    v.legal_actions
        .iter()
        .map(|a| match a {
            Action::Play(c) => *c,
            other => panic!("noncard {other:?}"),
        })
        .collect()
}

#[test]
fn frozen_deals_every_completion_exact_null_key_and_scope() {
    let (mut completions, mut original) = (0, 0);
    for c in cases() {
        let g = c.position();
        let actor = seat(c.actor);
        assert_eq!(g.current_seat(), Some(actor), "{}", c.id);
        let v = g.view(actor);
        assert_eq!(v.declarer, Some(seat(c.declarer)), "{}", c.id);
        assert_eq!(v.own_skat.is_some(), !c.hand && c.declarer == c.actor);
        assert_eq!(legal(&v), cards(c.legal), "{} legal order", c.id);
        assert_eq!(
            TrickState::of(GameType::Null, &v.current_trick)
                .best_seat
                .map(|s| s.index()),
            c.winner.map(|w| seat(w).index()),
            "{} current winner",
            c.id
        );
        for (&n, &p) in c.legal.iter().zip(c.power) {
            assert_eq!(power(GameType::Null, card(n)), p, "{} power {n}", c.id);
        }
        // Core replay: the actual individual completed winner of every candidate.
        for (&n, &w) in c.legal.iter().zip(c.completed) {
            let mut next = g.clone();
            next.apply(Action::Play(card(n))).unwrap();
            let t = next.tricks().last().unwrap();
            assert_eq!(t.winner, seat(w), "{} completion {n}", c.id);
            assert_eq!(t.cards[2], card(n));
            let source = skat_coach::Source::Trick {
                index: v.tricks.len() as u8,
                seat: actor,
                led: v.current_trick.cards[0].unwrap(),
                card: card(n),
            };
            let o = observe(GameType::Null, source, Some(t.winner)).unwrap();
            assert_eq!(o.winner, Some(seat(w)));
            assert_eq!(
                o.proves_void,
                card(n).suit() != v.current_trick.cards[0].unwrap().suit(),
                "Null follow class is the printed suit"
            );
            completions += 1;
            original += usize::from(c.origin == Origin::Original);
        }
        // Rule comparison is separate from the core replay above.
        let r = baseline::rule(&v).unwrap();
        assert_eq!(r.branch, Branch::Null);
        if c.winner.is_some() {
            assert_eq!(r.losers, cards(c.losers), "{} losers", c.id);
        }
        if let Some(choice) = c.choice {
            assert_eq!(r.card, card(choice), "{} choice", c.id);
            assert_eq!(r.tie_break, baseline::TieBreak::HighestPowerLast);
            let i = c.legal.iter().position(|&n| n == choice).unwrap();
            assert_eq!(r.tie_key, (c.power[i], 0, 0));
            // Maximum power among the losers, LAST exact tie in legal order.
            let max = c
                .losers
                .iter()
                .map(|&n| power(GameType::Null, card(n)))
                .max();
            let last = c
                .losers
                .iter()
                .rev()
                .find(|&&n| Some(power(GameType::Null, card(n))) == max);
            assert_eq!(last, Some(&choice));
        }
        if c.id == "real-defender-partner-ahead" {
            assert_eq!(r.tie_break, baseline::TieBreak::LowestCostFirst);
        }
        let l = lesson(&v);
        assert_eq!(l, lesson(&v), "deterministic");
        // Historical #165 exclusion retained in the frozen fixture. #176 now
        // admits this genuine VH first lead through its separate family.
        if c.id == "real-lead" {
            let l = l.expect("#176 opening lead");
            assert_eq!(l.rule, r);
            assert_eq!(
                l.source,
                skat_coach::Source::Lead {
                    index: 0,
                    seat: Seat::Vorhand
                }
            );
            assert_eq!(l.lead_declarer, Some(true));
            assert_eq!(l.remaining, None);
            assert_eq!(l.rule.card, card("H7"));
            assert_eq!(l.rule.candidates, cards(c.legal));
            assert_eq!(l.rule.winners, cards(c.legal));
            assert!(l.rule.losers.is_empty());
            assert_eq!(l.rule.tie_break, baseline::TieBreak::LowestCostFirst);
            assert_eq!(l.rule.tie_key, (0, 0, 0));
            assert!(!l.rule.last);
            assert!(l.check(
                Step::Orient,
                Answer::Lead {
                    seat: Seat::Vorhand,
                    declarer: true
                }
            ));
            for &n in c.legal {
                assert!(l.check(Step::Check, Answer::Card(card(n))));
                assert_eq!(l.check(Step::Narrow, Answer::Card(card(n))), n == "H7");
            }
            continue;
        }
        assert_eq!(
            l.is_some(),
            c.supported
                || matches!(
                    c.id,
                    "real-early" | "real-defender-declarer-ahead" | "real-ouvert"
                ),
            "{} applicability",
            c.id
        );
        if let Some(l) = l {
            assert_eq!(l.rule, r, "the shared rule, unchanged");
            assert_eq!(l.rule.branch, Branch::Null);
            let remaining = if c.id == "real-early" {
                Some(Seat::Hinterhand)
            } else {
                None
            };
            assert_eq!(
                (l.remaining, l.seat, l.game),
                (remaining, actor, GameType::Null)
            );
            assert_eq!(l.rule.last, c.id != "real-early");
            assert!(!l.rule.partner_ahead);
            assert_eq!(l.rule.candidates, cards(c.losers));
            assert_eq!(l.memory().trumpf, None, "no Null Trumpf category");
            assert_eq!(l.early_condition(), None);
            let winner = seat(c.winner.unwrap());
            assert!(l.check(Step::Orient, Answer::Seat(Some(winner))));
            for wrong in Seat::ALL.into_iter().filter(|&s| s != winner) {
                assert!(!l.check(Step::Orient, Answer::Seat(Some(wrong))));
            }
            assert!(l.check(Step::Check, Answer::Seat(remaining)));
            assert!(!l.check(Step::Check, Answer::Seat(Some(winner))));
            for &n in c.legal {
                assert_eq!(
                    l.check(Step::Narrow, Answer::Card(card(n))),
                    c.losers.contains(&n),
                    "{} narrow {n}",
                    c.id
                );
            }
        }
    }
    assert_eq!(original, 30, "every frozen original completion");
    assert_eq!(
        completions,
        30 + 10 + 4 + 4,
        "original, changed queen, real defender"
    );
}

#[test]
fn hand_and_pickup_hidden_substitutions_preserve_entire_view_and_actual_lesson() {
    let cs = cases();
    let by = |id: &str| cs.iter().find(|c| c.id == id).unwrap();
    for (base, a, b) in [
        ("mixed-ten", "VH H7", "Skat DK"),
        ("mixed-ten", "VH H7", "MH S7"),
        ("pickup-mixed", "VH H7", "MH S7"),
        ("real-ouvert", "VH H7", "Skat DK"),
        ("real-ouvert", "VH H7", "MH S7"),
    ] {
        let c = by(base);
        let x = c.position().view(Seat::Hinterhand);
        let y = c.swapped(a, b).view(Seat::Hinterhand);
        assert_eq!(x, y, "{base} {a}/{b} entire permitted view");
        let lx = lesson(&x);
        assert!(lx.is_some(), "the actual new lesson, never None == None");
        assert_eq!(lx, lesson(&y), "{base} {a}/{b} entire lesson");
        let m = lx.unwrap().memory();
        if c.hand {
            assert_eq!(m.skat, None, "Hand: the Skat stays unknown");
        } else {
            assert_eq!(m.skat, Some(set("DK DA")), "own known discards kept");
            assert_eq!(x.own_skat, Some(set("DK DA")));
        }
    }
}

#[test]
fn synthetic_and_malformed_views_never_get_a_null_lesson() {
    let base = cases()
        .into_iter()
        .find(|c| c.id == "mixed-ten")
        .unwrap()
        .position()
        .view(Seat::Hinterhand);
    assert!(lesson(&base).is_some());
    // Note: the original oracle labels `defender-declarer-ahead` with base
    // all-loser-king, but its harness actually cloned mixed-ten. The honest
    // actual case is ported; real-defender-declarer-ahead replays it in core.
    type ViewEdit = (&'static str, fn(&mut PlayerView));
    let edits: [ViewEdit; 24] = [
        ("defender", |v| v.declarer = Some(Seat::Vorhand)),
        ("defender-declarer-ahead (actual mixed-ten)", |v| {
            v.declarer = Some(Seat::Vorhand)
        }),
        ("lead", |v| v.current_trick = Trick::default()),
        ("early", |v| v.current_trick.cards[1] = None),
        ("ouvert", |v| v.contract.as_mut().unwrap().ouvert = true),
        ("single-card", |v| {
            v.legal_actions = vec![Action::Play(card("C7"))]
        }),
        ("mixed-actions", |v| v.legal_actions.push(Action::Kontra)),
        ("call-only", |v| v.legal_actions = vec![Action::Kontra]),
        ("absent-contract", |v| v.contract = None),
        ("inconsistent-contract", |v| {
            v.contract = Some(Contract {
                game: GameType::Grand,
                ..v.contract.unwrap()
            })
        }),
        ("absent-seat", |v| v.current_seat = None),
        ("inconsistent-seat", |v| v.seat = Seat::Mittelhand),
        ("absent-declarer", |v| v.declarer = None),
        ("absent-hand-decision", |v| v.hand_game = None),
        ("inconsistent-hand-decision", |v| v.hand_game = Some(false)),
        ("inconsistent-null-contract", |v| {
            v.contract.as_mut().unwrap().schneider = true
        }),
        ("inconsistent-hand-skat", |v| {
            v.own_skat = Some(set("DK DA"))
        }),
        ("restricted-cards", |v| {
            v.legal_actions.pop();
        }),
        ("invalid-card", |v| {
            v.legal_actions.push(Action::Play(card("SA")))
        }),
        ("wrong-turn", |v| v.current_seat = Some(Seat::Vorhand)),
        ("absent-leader", |v| v.current_trick.leader = None),
        ("wrong-leader", |v| {
            v.current_trick.leader = Some(Seat::Mittelhand)
        }),
        ("gap-in-trick", |v| {
            v.current_trick.cards[2] = v.current_trick.cards[1];
            v.current_trick.cards[1] = None;
        }),
        ("wrong-phase", |v| v.phase = skat_core::Phase::Ansage),
    ];
    for (id, edit) in edits {
        let mut v = base.clone();
        edit(&mut v);
        let l = lesson(&v);
        if id == "inconsistent-contract" {
            // The Grand view keeps its existing, unchanged Grand WinLast lesson.
            let l = l.unwrap();
            assert_eq!((l.game, l.rule.branch), (GameType::Grand, Branch::WinLast));
        } else if matches!(
            id,
            "defender" | "defender-declarer-ahead (actual mixed-ten)"
        ) {
            assert_eq!(l.unwrap().rule.card, card("C8"));
        } else {
            assert_eq!(l, None, "{id}");
        }
    }
}

#[test]
fn ouvert_last_requires_revealed_matching_hand_and_exact_actions() {
    let c = cases().into_iter().find(|c| c.id == "real-ouvert").unwrap();
    let v = c.position().view(Seat::Hinterhand);
    assert_eq!(v.declarer_cards, Some(v.hand));
    assert_eq!(lesson(&v).unwrap().rule.card, card("C8"));
    for edit in 0..4 {
        let mut bad = v.clone();
        match edit {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(set("C7")),
            2 => {
                bad.legal_actions.pop();
            }
            _ => bad.current_trick.cards[1] = None,
        }
        assert!(lesson(&bad).is_none());
    }
    for (id, expected) in [("no-loser", None), ("void-last-tie", Some("DJ"))] {
        let mut c = cases().into_iter().find(|c| c.id == id).unwrap();
        c.ouvert = true;
        let v = c.position().view(Seat::Hinterhand);
        assert_eq!(lesson(&v).map(|l| l.rule.card), expected.map(card));
    }
}
