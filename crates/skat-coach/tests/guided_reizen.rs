// SPDX-License-Identifier: EUPL-1.2

//! Core-accepted Reizen fixtures for the guided threshold dialogue (#148).
//! Expected values are written out from the own hands, independent of the rule.

#[path = "support/guided.rs"]
#[allow(dead_code)]
mod support;
use skat_coach::guided::*;
use skat_core::{Action, Deal, Game, GameType, RuleConfig, Seat, Suit};
use support::*;

use Action::{Bid, Hold, Pass};

/// Applies core-checked bidding calls; never a fabricated view.
fn at(deal: Deal, calls: &[Action]) -> (Game, BidLesson) {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    for &a in calls {
        g.apply(a).unwrap();
    }
    let view = g.view(g.current_seat().unwrap());
    let l = bid_lesson(&view).expect("Reizen decision");
    // Every legal action stays available and accepted by the core.
    for a in &view.legal_actions {
        g.clone().apply(*a).unwrap();
    }
    assert!(view.legal_actions.contains(&l.rule.action));
    assert!(!l.rule.fallback);
    (g, l)
}

/// Checks orientation and the threshold answer against an explicit expectation.
fn expect(
    l: &BidLesson,
    seat: Seat,
    current: u16,
    holds: bool,
    stake: Option<u16>,
    action: Action,
) {
    assert_eq!(l.seat, seat);
    assert_eq!(l.rule.current, current);
    assert_eq!(l.rule.holds, holds);
    assert_eq!(l.rule.stake, stake);
    assert_eq!(l.rule.action, action);
    assert!(l.check(Step::Orient, Answer::Holds(holds)));
    assert!(!l.check(Step::Orient, Answer::Holds(!holds)));
    let within = action != Pass;
    assert!(l.check(Step::Threshold, Answer::Within(within)));
    assert!(!l.check(Step::Threshold, Answer::Within(!within)));
    // Answers of the other dialogue never count.
    assert!(!l.check(Step::Narrow, Answer::Within(within)));
}

/// VH: Herz with Kreuz-Bube, mit 1: 20. MH: Pik ohne 1: 22. HH: Karo ohne 3: 36.
#[test]
fn bidding_and_holding_below_at_and_above_the_suit_ceiling() {
    let d = deal();
    let (_, l) = at(d, &[]);
    assert_eq!(
        (l.rule.game, l.rule.ceiling),
        (GameType::Farbe(Suit::Pik), 22)
    );
    assert_eq!(l.rule.branch, GameBranch::Suit);
    expect(&l, Seat::Mittelhand, 0, false, Some(18), Bid(18));
    let (_, l) = at(d, &[Bid(18)]);
    assert_eq!(
        (l.rule.game, l.rule.ceiling),
        (GameType::Farbe(Suit::Herz), 20)
    );
    expect(&l, Seat::Vorhand, 18, true, Some(18), Hold);
    let (_, l) = at(d, &[Bid(20)]);
    expect(&l, Seat::Vorhand, 20, true, Some(20), Hold);
    let (_, l) = at(d, &[Bid(22)]);
    expect(&l, Seat::Vorhand, 22, true, Some(22), Pass);
    let (_, l) = at(d, &[Bid(18), Hold]);
    expect(&l, Seat::Mittelhand, 18, false, Some(20), Bid(20));
    let (_, l) = at(d, &[Bid(18), Hold, Bid(20), Hold]);
    expect(&l, Seat::Mittelhand, 20, false, Some(22), Bid(22));
    // A forced core-legal Hold above VH's own ceiling puts MH above its own.
    let (_, l) = at(d, &[Bid(22), Hold]);
    expect(&l, Seat::Mittelhand, 22, false, Some(23), Pass);
}

#[test]
fn public_jump_bid_is_the_current_value_not_a_pre_decision_step() {
    let d = deal();
    // MH jumps to 35: VH answers 35, never 18 or 22.
    let (_, l) = at(d, &[Bid(35)]);
    expect(&l, Seat::Vorhand, 35, true, Some(35), Pass);
    // HH asks after the jump: the next standard value is 36, its exact ceiling.
    let (_, l) = at(d, &[Bid(35), Pass]);
    assert_eq!(
        (l.rule.game, l.rule.ceiling),
        (GameType::Farbe(Suit::Karo), 36)
    );
    expect(&l, Seat::Hinterhand, 35, false, Some(36), Bid(36));
    let (_, l) = at(d, &[Bid(36), Pass]);
    expect(&l, Seat::Hinterhand, 36, false, Some(40), Pass);
}

#[test]
fn zero_null_grand_and_unavailable_next_value_are_named_exactly() {
    // Weak VH, Grand MH, ordinary HH.
    let d = Deal::new(
        [
            cards("C7 C8 C9 S7 S8 S9 H7 H8 HA DA"),
            cards("CJ SJ HJ DJ CA CT SA ST HT D8"),
            cards("CQ CK SQ SK H9 HQ HK D7 D9 DT"),
        ],
        cards("DQ DK"),
    )
    .unwrap();
    let (_, l) = at(d, &[Bid(18)]);
    assert_eq!((l.rule.ceiling, l.rule.branch), (0, GameBranch::Weak));
    expect(&l, Seat::Vorhand, 18, true, Some(18), Pass);
    let (_, l) = at(d, &[]);
    assert_eq!(l.rule.branch, GameBranch::Grand);
    assert_eq!(l.rule.game, GameType::Grand);
    let ceiling = l.rule.ceiling;
    assert_eq!(ceiling, 24 * 5, "mit 4: 24 x (4 + 1)");
    expect(&l, Seat::Mittelhand, 0, false, Some(18), Bid(18));
    // No standard value above 264 exists: Pass is the only legal action.
    let (g, l) = at(d, &[Bid(264), Hold]);
    assert_eq!(g.legal_actions(), vec![Pass]);
    expect(&l, Seat::Mittelhand, 264, false, None, Pass);

    // Null VH: fixed 23, holding at and above the value.
    let d = Deal::new(
        [
            cards("C7 C8 C9 S7 S8 S9 H7 H8 H9 D7"),
            cards("CJ SJ HJ DJ CA CT SA ST HT D8"),
            cards("CQ CK SQ SK HA HQ HK DA D9 DT"),
        ],
        cards("DQ DK"),
    )
    .unwrap();
    let (_, l) = at(d, &[Bid(23)]);
    assert_eq!(
        (l.rule.game, l.rule.ceiling, l.rule.branch),
        (GameType::Null, 23, GameBranch::Null)
    );
    expect(&l, Seat::Vorhand, 23, true, Some(23), Hold);
    let (_, l) = at(d, &[Bid(24)]);
    expect(&l, Seat::Vorhand, 24, true, Some(24), Pass);
    // Vorhand alone after two passes names 18 within its Null ceiling.
    let (g, l) = at(d, &[Pass, Pass]);
    assert_eq!(g.legal_actions(), vec![Bid(18), Pass]);
    expect(&l, Seat::Vorhand, 0, false, Some(18), Bid(18));
}

#[test]
fn hidden_deal_substitutions_keep_the_own_hand_rule_and_skat_supplies_no_spitzen() {
    let d = deal();
    // SJ moves from MH into the hidden Skat; C7/H7 swap between the other hands.
    let variants = [
        d,
        swap(d, card("SJ"), card("DK")),
        swap(d, card("C7"), card("H7")),
    ];
    let calls = [Bid(18), Hold, Bid(20), Hold, Bid(22), Pass];
    let (g0, l0) = at(variants[0], &calls);
    for v in variants {
        let (g, l) = at(v, &calls);
        assert_eq!(g.view(Seat::Hinterhand), g0.view(Seat::Hinterhand));
        assert_eq!(l, l0);
    }
    // Ohne 3 from the own ten cards: a Skat Bube would lower, not raise, the
    // actual value; the ceiling still counts only the own hand.
    assert_eq!(l0.rule.ceiling, 36);
    expect(&l0, Seat::Hinterhand, 22, false, Some(23), Bid(23));
}
