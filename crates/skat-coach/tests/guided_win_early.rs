// SPDX-License-Identifier: EUPL-1.2

//! Independent handwritten A–E oracle replayed through core, then compared
//! separately with shared policy and coach output. Both roles are fixtures.
#[path = "support/guided.rs"]
mod fixture;
#[path = "support/win_early.rs"]
mod oracle;
use fixture::*;
use skat_analysis::{Knowledge, baseline};
use skat_coach::{Source, guided::*};
use skat_core::{Action, Follow, GameType, Seat, Suit};

#[test]
fn handwritten_a_to_e_both_roles_all_58_legal_completions_and_16_named_winners() {
    assert_eq!(oracle::deals()[4], deal());
    for declarer in [Seat::Mittelhand, Seat::Vorhand] {
        for (n, deal) in oracle::deals().into_iter().enumerate() {
            let g = oracle::position(deal, declarer);
            let v = g.view(Seat::Mittelhand);
            let legal: Vec<_> = cards(oracle::LEGAL[n]).iter().map(Action::Play).collect();
            assert_eq!(g.legal_actions(), legal);
            // Domain predicate independent of shared policy.
            let winners: skat_core::CardSet = legal
                .iter()
                .filter_map(|a| {
                    let Action::Play(c) = a else { unreachable!() };
                    GameType::Grand
                        .beats(*c, card("CK"), card("CK"))
                        .then_some(*c)
                })
                .collect();
            assert_eq!(winners, cards(oracle::WINNERS[n]));
            let r = baseline::rule(&v).unwrap();
            assert_eq!(r.card, card(oracle::EASY[n]));
            assert_eq!(
                r.branch,
                if n == 4 {
                    Branch::Fallback
                } else {
                    Branch::WinEarly
                }
            );
            let l = lesson(&v);
            if n == 4 {
                // Declining defender fallback only behind the declarer.
                assert_eq!(l.is_some(), declarer == Seat::Vorhand);
            } else {
                let l = l.unwrap();
                assert_eq!(
                    l.rule
                        .candidates
                        .iter()
                        .copied()
                        .collect::<skat_core::CardSet>(),
                    winners
                );
                assert!(!l.rule.last && !l.rule.partner_ahead);
                assert_eq!(l.remaining, Some(Seat::Hinterhand));
                assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
                assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
                assert_eq!(
                    l.early_condition(),
                    Some(if n < 2 {
                        EarlyCondition::FollowClassMaster
                    } else {
                        EarlyCondition::Trumpf
                    })
                );
                assert_eq!(Knowledge::from_view(&v).is_master(r.card), n < 2);
                for c in v.hand.iter() {
                    assert_eq!(l.check(Step::Narrow, Answer::Card(c)), winners.contains(c));
                }
            }
            for mine in oracle::LEGAL[n].split_whitespace() {
                check_completion(&g, mine, oracle::REPLY[n], oracle::winner(n, mine));
            }
            // Frozen named witnesses additional to the all-alternative table.
            for (mine, reply, expected) in match n {
                0 => vec![
                    ("CT", "CQ", Seat::Mittelhand),
                    ("C8", "CQ", Seat::Vorhand),
                    ("CA", "CQ", Seat::Mittelhand),
                ],
                1 => vec![("CT", "DJ", Seat::Hinterhand)],
                2 => vec![("SJ", "CA", Seat::Mittelhand), ("D7", "C7", Seat::Vorhand)],
                3 => vec![("SJ", "CJ", Seat::Hinterhand)],
                _ => vec![("CT", "CA", Seat::Hinterhand)],
            } {
                check_completion(&g, mine, reply, expected);
            }
        }
    }
}

fn check_completion(g: &skat_core::Game, mine: &str, reply: &str, winner: Seat) {
    let mut g = g.clone();
    g.apply(Action::Play(card(mine))).unwrap();
    g.apply(Action::Play(card(reply))).unwrap();
    assert_eq!(g.tricks()[0].winner, winner, "{mine}/{reply}");
    let o = observe(
        GameType::Grand,
        Source::Trick {
            index: 0,
            seat: Seat::Hinterhand,
            led: card("CK"),
            card: card(reply),
        },
        Some(g.tricks()[0].winner),
    )
    .unwrap();
    assert_eq!(o.follow, Follow::Farbe(Suit::Kreuz));
    assert_eq!(o.proves_void, GameType::Grand.is_trump(card(reply)));
    assert_eq!(o.winner, Some(winner));
}

#[test]
fn hidden_swaps_and_unknown_skat_do_not_change_early_lesson() {
    for declarer in [Seat::Mittelhand, Seat::Vorhand] {
        let d = oracle::deals();
        for (a, b) in [
            (d[0], d[1]),
            (d[2], d[3]),
            (d[0], swap(d[0], card("DK"), card("SA"))),
            (d[2], swap(d[2], card("DK"), card("SA"))),
        ] {
            let a = oracle::position(a, declarer).view(Seat::Mittelhand);
            let b = oracle::position(b, declarer).view(Seat::Mittelhand);
            assert_eq!(a, b);
            assert_eq!(lesson(&a), lesson(&b));
        }
    }
}

#[test]
fn early_exclusions_retain_existing_branches_and_suit_extension() {
    let d = oracle::deals();
    let no = swap(d[4], card("CT"), card("C9"));
    let one = swap(swap(d[0], card("C8"), card("H7")), card("CT"), card("H8"));
    for (deal, decl, game, plays, seat, branch, supported) in [
        (
            d[4],
            Seat::Hinterhand,
            hand_game(GameType::Grand, false),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::UnderPartner,
            true,
        ),
        (
            no,
            Seat::Mittelhand,
            hand_game(GameType::Grand, false),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::Fallback,
            false,
        ),
        (
            one,
            Seat::Mittelhand,
            hand_game(GameType::Grand, false),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::WinEarly,
            false,
        ),
        (
            d[4],
            Seat::Vorhand,
            hand_game(GameType::Grand, false),
            vec!["CK", "C7"],
            Seat::Hinterhand,
            Branch::WinLast,
            true,
        ),
        (
            d[4],
            Seat::Mittelhand,
            hand_game(GameType::Null, false),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::Null,
            true,
        ),
        (
            d[0],
            Seat::Mittelhand,
            hand_game(GameType::Grand, true),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::WinEarly,
            true,
        ),
        (
            d[0],
            Seat::Mittelhand,
            hand_game(GameType::Farbe(Suit::Herz), false),
            vec!["CK"],
            Seat::Mittelhand,
            Branch::WinEarly,
            true,
        ),
    ] {
        let v = contract_position(deal, decl, game, &plays).view(seat);
        assert_eq!(baseline::rule(&v).unwrap().branch, branch);
        assert_eq!(lesson(&v).is_some(), supported);
    }
}

#[test]
fn ouvert_early_reuses_rule_and_rejects_invalid_views() {
    let deal = oracle::deals()[0];
    let make = |deal| {
        contract_position(
            deal,
            Seat::Mittelhand,
            hand_game(GameType::Grand, true),
            &["CK"],
        )
    };
    let g = make(deal);
    let v = g.view(Seat::Mittelhand);
    let l = lesson(&v).unwrap();
    assert_eq!(l.rule, baseline::rule(&v).unwrap());
    assert_eq!(l.rule.branch, Branch::WinEarly);
    for mine in oracle::LEGAL[0].split_whitespace() {
        check_completion(&g, mine, oracle::REPLY[0], oracle::winner(0, mine));
    }
    let other = make(swap(deal, card("H7"), card("DK")));
    assert_eq!(v, other.view(Seat::Mittelhand));
    assert_eq!(Some(l), lesson(&other.view(Seat::Mittelhand)));
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.declarer_cards = Some(skat_core::CardSet::default()),
        |v| v.contract.as_mut().unwrap().schwarz = false,
        |v| v.legal_actions.truncate(1),
        |v| v.current_seat = Some(Seat::Vorhand),
    ] {
        let mut invalid = v.clone();
        edit(&mut invalid);
        assert!(lesson(&invalid).is_none());
    }
}
