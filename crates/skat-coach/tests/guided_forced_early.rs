// SPDX-License-Identifier: EUPL-1.2

//! Core-replayed second-seat Grand/Farbspiel decision in which every legal
//! card overtakes the opponent's lead but none is Trumpf or a follow-class
//! master. Easy then keeps the lowest legal card by `(is_trump, Augen, power)`.

#[path = "support/guided.rs"]
mod support;
use skat_analysis::baseline::rule;
use skat_coach::guided::*;
use skat_core::{Action, Deal, GameType, Seat, Suit};
use support::*;

const HERZ: GameType = GameType::Farbe(Suit::Herz);

/// Vorhand leads C7; Mittelhand holds only C8 and C9 in Kreuz.
fn forced_deal() -> Deal {
    Deal::new(
        [
            cards("C7 CK S7 S8 S9 ST SK H7 H8 H9"),
            cards("C8 C9 SA SQ D7 D8 D9 DT DK DQ"),
            cards("CT CJ CQ CA SJ HJ HQ HK HA DJ"),
        ],
        cards("DA HT"),
    )
    .unwrap()
}

#[test]
fn forced_second_seat_overtake_in_both_roles_and_games() {
    for game in [GameType::Grand, HERZ] {
        // Defender behind the declarer, then declarer behind a defender lead.
        for declarer in [Seat::Vorhand, Seat::Mittelhand] {
            let g = contract_position(forced_deal(), declarer, hand_game(game, false), &["C7"]);
            let v = g.view(Seat::Mittelhand);
            assert_eq!(
                v.legal_actions,
                [Action::Play(card("C8")), Action::Play(card("C9"))]
            );
            let l = lesson(&v).expect("forced overtake lesson");
            assert_eq!(l.rule, rule(&v).unwrap());
            assert_eq!(l.rule.branch, Branch::Fallback);
            assert!(!l.rule.partner_ahead && !l.rule.last);
            assert!(l.rule.losers.is_empty());
            assert_eq!(l.rule.winners, [card("C8"), card("C9")]);
            assert_eq!(l.rule.card, card("C8"));
            assert_eq!(l.remaining, Some(Seat::Hinterhand));
            assert!(l.check(Step::Narrow, Answer::Card(card("C8"))));
            assert!(!l.check(Step::Narrow, Answer::Card(card("C9"))));
            // Unseen cards (the other defender's, the Skat) do not matter.
            let swapped = contract_position(
                swap(forced_deal(), card("CT"), card("HT")),
                declarer,
                hand_game(game, false),
                &["C7"],
            );
            assert_eq!(v, swapped.view(Seat::Mittelhand));
            assert_eq!(Some(l.clone()), lesson(&swapped.view(Seat::Mittelhand)));
            // Every legal card is playable; Hinterhand's CT then takes the trick.
            for a in v.legal_actions.clone() {
                let mut next = g.clone();
                next.apply(a).unwrap();
                next.apply(Action::Play(card("CT"))).unwrap();
                assert_eq!(next.tricks()[0].winner, Seat::Hinterhand);
            }
            let mut restricted = v.clone();
            restricted.legal_actions.pop();
            assert!(lesson(&restricted).is_none());
        }
    }
}

#[test]
fn forced_overtake_in_ouvert_needs_the_consistent_reveal() {
    for game in [GameType::Grand, HERZ] {
        for declarer in [Seat::Vorhand, Seat::Mittelhand] {
            let g = contract_position(forced_deal(), declarer, hand_game(game, true), &["C7"]);
            let v = g.view(Seat::Mittelhand);
            let l = lesson(&v).expect("Ouvert forced overtake lesson");
            assert_eq!(l.rule.branch, Branch::Fallback);
            assert!(l.rule.losers.is_empty());
            assert_eq!(l.rule.card, card("C8"));
            let mut missing = v.clone();
            missing.declarer_cards = None;
            assert!(lesson(&missing).is_none());
            let mut overlap = v.clone();
            overlap.declarer_cards = Some(if declarer == Seat::Mittelhand {
                v.hand - cards("C8")
            } else {
                v.hand
            });
            assert!(lesson(&overlap).is_none());
        }
    }
}
