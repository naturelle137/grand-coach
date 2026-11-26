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
use skat_core::{Action, GameType, Seat, Suit};
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
fn declarer_role_partner_ahead_and_single_card_stay_outside() {
    // Declarer in the middle seat takes the same Fallback but is not a defender.
    let g = contract_position(
        deal(),
        Seat::Mittelhand,
        hand_game(GameType::Grand, false),
        &["CK"],
    );
    let v = g.view(Seat::Mittelhand);
    assert_eq!(rule(&v).unwrap().branch, Branch::Fallback);
    assert!(lesson(&v).is_none());

    // Single legal card (C7 only): nothing to choose.
    let d = swap(swap(deal(), card("C8"), card("H7")), card("CT"), card("H8"));
    let g = contract_position(d, Seat::Vorhand, hand_game(GameType::Grand, false), &["C9"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["C7"]));
    assert!(lesson(&v).is_none());
}
