// SPDX-License-Identifier: EUPL-1.2

//! Core-replayed non-Ouvert defender "decline" fallback: the declarer is
//! ahead and Easy does not take the trick (the cheapest overtaking card is
//! neither Trumpf nor a follow-class master, or nothing overtakes), so it keeps
//! the lowest legal card by `(is_trump, Augen, power)`, first exact tie. Expected
//! cards are derived by hand from the Grand/Herz card orders.

#[path = "support/guided.rs"]
mod support;
use skat_analysis::baseline::rule;
use skat_coach::guided::*;
use skat_core::{Action, Deal, GameType, Seat, Suit};
use support::*;

const HERZ: GameType = GameType::Farbe(Suit::Herz);

fn plays(cards: &[&str]) -> Vec<Action> {
    cards.iter().map(|c| Action::Play(card(c))).collect()
}

#[test]
fn ouvert_defender_declines_with_a_consistent_revealed_hand() {
    for game in [GameType::Grand, HERZ] {
        let g = contract_position(deal(), Seat::Vorhand, hand_game(game, true), &["C9"]);
        let v = g.view(Seat::Mittelhand);
        let l = lesson(&v).expect("Ouvert decline lesson");
        assert_eq!(l.rule.branch, Branch::Fallback);
        assert_eq!(l.rule.card, card("C7"));
        assert_eq!(l.rule.candidates, vec![card("C7"), card("C8")]);
        let mut missing = v.clone();
        missing.declarer_cards = None;
        assert!(lesson(&missing).is_none());
        let mut overlap = v.clone();
        overlap.declarer_cards = Some(v.hand);
        assert!(lesson(&overlap).is_none());
        let mut restricted = v.clone();
        restricted.legal_actions.pop();
        assert!(lesson(&restricted).is_none());
    }
}

#[test]
fn second_seat_defender_keeps_a_loser_under_the_declarer_in_grand_and_farbspiel() {
    // VH declarer leads C9; MH's only overtaking card CT is no master (CA, CK,
    // CQ unseen) and no Trumpf, so Easy keeps the lowest loser C7 (C8 ties on
    // Augen but ranks higher).
    for game in [GameType::Grand, HERZ] {
        let g = contract_position(deal(), Seat::Vorhand, hand_game(game, false), &["C9"]);
        let v = g.view(Seat::Mittelhand);
        assert_eq!(v.legal_actions, plays(&["C7", "C8", "CT"]));
        let l = lesson(&v).expect("decline lesson");
        assert_eq!(l.rule.branch, Branch::Fallback);
        assert!(!l.rule.partner_ahead && !l.rule.last);
        assert_eq!(l.rule.winner, Some(Seat::Vorhand));
        assert_eq!(l.rule.winners, vec![card("CT")]);
        assert_eq!(l.rule.candidates, vec![card("C7"), card("C8")]);
        assert_eq!(
            (l.rule.card, l.remaining),
            (card("C7"), Some(Seat::Hinterhand))
        );
        assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
        assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
        assert!(l.check(Step::Narrow, Answer::Card(card("C7"))));
        assert!(!l.check(Step::Narrow, Answer::Card(card("C8"))));
        assert!(!l.check(Step::Narrow, Answer::Card(card("CT"))));

        // Unseen cards, including the Skat, are not part of the dialogue.
        let other = contract_position(
            swap(deal(), card("H8"), card("DK")),
            Seat::Vorhand,
            hand_game(game, false),
            &["C9"],
        );
        assert_eq!(v, other.view(Seat::Mittelhand));
        assert_eq!(Some(l.clone()), lesson(&other.view(Seat::Mittelhand)));

        // Restricted legal list, revealed declarer hand or a Skat are rejected.
        let mut restricted = v.clone();
        restricted.legal_actions.pop();
        assert!(lesson(&restricted).is_none());
        let mut exposed = v.clone();
        exposed.declarer_cards = Some(g.view(Seat::Vorhand).hand);
        assert!(lesson(&exposed).is_none());
    }
}

#[test]
fn last_seat_defender_without_an_overtaking_card_keeps_the_lowest_loser() {
    // MH declarer trumps VH's H7 with SJ; HH's H8 and HA cannot overtake.
    let d = swap(deal(), card("D7"), card("H8"));
    let g = contract_position(
        d,
        Seat::Mittelhand,
        hand_game(GameType::Grand, false),
        &["H7", "SJ"],
    );
    let v = g.view(Seat::Hinterhand);
    assert_eq!(v.legal_actions, plays(&["H8", "HA"]));
    let l = lesson(&v).expect("last-seat decline lesson");
    assert_eq!(l.rule.branch, Branch::Fallback);
    assert!(l.rule.winners.is_empty() && l.rule.last && l.remaining.is_none());
    assert_eq!(l.rule.winner, Some(Seat::Mittelhand));
    assert_eq!(l.rule.card, card("H8"));
    assert!(l.check(Step::Narrow, Answer::Card(card("H8"))));
    assert!(!l.check(Step::Narrow, Answer::Card(card("HA"))));
}

#[test]
fn single_card_and_declarer_second_seat_without_loser_stay_outside() {
    // Declarer in the middle seat: the same Fallback is now admitted only with
    // a declarer decline lesson (see below); it names the opponent as winner.
    let g = contract_position(
        deal(),
        Seat::Mittelhand,
        hand_game(GameType::Grand, false),
        &["CK"],
    );
    let v = g.view(Seat::Mittelhand);
    assert_eq!(rule(&v).unwrap().branch, Branch::Fallback);
    assert_eq!(lesson(&v).unwrap().rule.winner, Some(Seat::Vorhand));

    // Single legal card (C7 only): nothing to choose.
    let d = swap(swap(deal(), card("C8"), card("H7")), card("CT"), card("H8"));
    let g = contract_position(d, Seat::Vorhand, hand_game(GameType::Grand, false), &["C9"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["C7"]));
    assert!(lesson(&v).is_none());
}

#[test]
fn declarer_second_seat_keeps_a_loser_under_an_opponent_lead() {
    // MH declares; VH leads C9. MH's only overtaking card CT is no master and
    // no Trumpf, so Easy keeps the lowest loser C7 (C8 ranks higher).
    for game in [GameType::Grand, HERZ] {
        let g = contract_position(deal(), Seat::Mittelhand, hand_game(game, false), &["C9"]);
        let v = g.view(Seat::Mittelhand);
        let Some(l) = lesson(&v) else {
            panic!("no declarer decline lesson for {game:?}")
        };
        assert_eq!(l.rule.branch, Branch::Fallback);
        assert!(!l.rule.partner_ahead && !l.rule.last);
        assert_eq!(l.rule.winner, Some(Seat::Vorhand));
        assert_eq!(l.rule.card, card("C7"));
        assert!(l.check(Step::Narrow, Answer::Card(card("C7"))));
        assert!(!l.check(Step::Narrow, Answer::Card(card("C8"))));
        let mut restricted = v.clone();
        restricted.legal_actions.pop();
        assert!(lesson(&restricted).is_none());
        let mut revealed = v.clone();
        revealed.declarer_cards = Some(v.hand);
        assert!(lesson(&revealed).is_none());
    }
}

#[test]
fn declarer_lead_without_master_or_trumpf_is_the_lowest_card() {
    // VH declares Grand Hand with no Bube and no Ace: nothing is a master and
    // no Trumpf can be drawn, so Easy leads the lowest card, C7 (first of the exact ties).
    let deal = Deal::new(
        [
            cards("C7 C8 C9 S7 S8 H7 H8 D7 D8 D9"),
            cards("CA CT CK CQ CJ SA ST SK SQ SJ"),
            cards("S9 HA HT HK HQ HJ H9 DA DT DK"),
        ],
        cards("DQ DJ"),
    )
    .unwrap();
    let g = contract_position(deal, Seat::Vorhand, hand_game(GameType::Grand, false), &[]);
    let v = g.view(Seat::Vorhand);
    let l = lesson(&v).expect("lead fallback lesson");
    assert_eq!(l.rule.branch, Branch::Fallback);
    assert_eq!(l.rule.card, card("C7"));
    assert!(l.check(
        Step::Orient,
        Answer::Lead {
            seat: Seat::Vorhand,
            declarer: true
        }
    ));
    for a in &v.legal_actions {
        let Action::Play(c) = a else { panic!() };
        assert_eq!(l.check(Step::Narrow, Answer::Card(*c)), *c == card("C7"));
    }
    // Hidden cards may be exchanged without changing the view or the lesson.
    let other = swap(deal, card("CA"), card("HA"));
    let w = contract_position(other, Seat::Vorhand, hand_game(GameType::Grand, false), &[])
        .view(Seat::Vorhand);
    assert_eq!((&w, lesson(&w)), (&v, Some(l)));
    // Restricted actions stay unsupported; Ouvert is covered separately.
    let mut restricted = v.clone();
    restricted.legal_actions.pop();
    assert!(lesson(&restricted).is_none());
}

#[test]
fn ouvert_lead_without_master_or_trumpf_is_the_lowest_card() {
    // Grand Ouvert: the declarer VH (own hand revealed) and the defender VH
    // against a revealed MH both lead C7; Easy's master test ignores the reveal.
    let deal = Deal::new(
        [
            cards("C7 C8 C9 S7 S8 H7 H8 D7 D8 D9"),
            cards("CA CT CK CQ CJ SA ST SK SQ SJ"),
            cards("S9 HA HT HK HQ HJ H9 DA DT DK"),
        ],
        cards("DQ DJ"),
    )
    .unwrap();
    for declarer in [Seat::Vorhand, Seat::Mittelhand] {
        let g = contract_position(deal, declarer, hand_game(GameType::Grand, true), &[]);
        let v = g.view(Seat::Vorhand);
        let l = lesson(&v).expect("Ouvert lead fallback lesson");
        assert_eq!(l.rule.branch, Branch::Fallback);
        assert_eq!(l.rule.card, card("C7"));
        assert!(l.check(
            Step::Orient,
            Answer::Lead {
                seat: Seat::Vorhand,
                declarer: declarer == Seat::Vorhand
            }
        ));
        // Hidden cards may be exchanged without changing the view or the lesson.
        let other = swap(deal, card("S9"), card("DQ"));
        let w = contract_position(other, declarer, hand_game(GameType::Grand, true), &[])
            .view(Seat::Vorhand);
        assert_eq!((&w, lesson(&w)), (&v, Some(l)));
        let mut restricted = v.clone();
        restricted.legal_actions.pop();
        assert!(lesson(&restricted).is_none());
        if declarer != Seat::Vorhand {
            let mut missing = v.clone();
            missing.declarer_cards = None;
            assert!(lesson(&missing).is_none());
            let mut overlap = v.clone();
            overlap.declarer_cards = Some(v.hand);
            assert!(lesson(&overlap).is_none());
        }
    }
}
