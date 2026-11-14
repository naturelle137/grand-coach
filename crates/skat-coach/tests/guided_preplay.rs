// SPDX-License-Identifier: EUPL-1.2

//! Core-accepted Skataufnahme and Drücken fixtures for the guided dialogue
//! (#131 phase 4). Expected strengths and discards are worked out by hand in
//! the comments from Easy's documented counts, independent of the rule:
//! Trumpf Bube 14 (Kreuz 18), Trumpf Ass/Zehn 11, other Trumpf 8; side Ace
//! 10; side Ten 6 with its Ace or two more cards, else 1; empty side suit 3
//! (not in Grand; the Trumpf suit itself has no side cards and counts as
//! empty in a suit game). Hand from 100. Discard keeps Trumpf, Aces and guarded
//! Tens; otherwise shorter suits and more Augen go first; Null puts away the
//! highest cards.

#[path = "support/guided.rs"]
#[allow(dead_code)]
mod support;
use skat_coach::guided::*;
use skat_core::{Action, CardSet, Deal, Game, GameType, Phase, RuleConfig, Seat, Suit};
use support::*;

/// HH wins the bidding at 18 (core-checked), then takes `after` actions.
fn at(deal: Deal, after: &[Action]) -> (Game, PreLesson) {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    for a in [Action::Pass, Action::Bid(18), Action::Pass] {
        g.apply(a).unwrap();
    }
    for &a in after {
        g.apply(a).unwrap();
    }
    let view = g.view(g.current_seat().unwrap());
    assert_eq!(view.seat, Seat::Hinterhand);
    let l = pre_lesson(&view).expect("declarer pre-play decision");
    assert_eq!(l.seat, Seat::Hinterhand);
    // Every legal action stays available and accepted by the core.
    for a in &view.legal_actions {
        g.clone().apply(*a).unwrap();
    }
    assert!(view.legal_actions.contains(&l.rule.action()));
    assert!(!l.rule.fallback());
    (g, l)
}

fn pickup(l: &PreLesson) -> PickupRule {
    match l.rule {
        PreRule::Pickup(r) => r,
        _ => panic!("pickup expected"),
    }
}

fn discard(l: &PreLesson) -> DiscardRule {
    match l.rule {
        PreRule::Discard(r) => r,
        _ => panic!("discard expected"),
    }
}

/// Orientation accepts exactly the branch; answers of other dialogues never count.
fn orient(l: &PreLesson, branch: GameBranch) {
    for b in [
        GameBranch::Grand,
        GameBranch::Suit,
        GameBranch::Null,
        GameBranch::Weak,
    ] {
        assert_eq!(l.check(Step::Orient, Answer::Game(b)), b == branch);
    }
    assert!(!l.check(Step::Orient, Answer::Holds(true)));
    assert!(!l.check(Step::Narrow, Answer::Game(branch)));
}

fn expect_pickup(l: &PreLesson, game: GameType, branch: GameBranch, strength: i32) {
    let r = pickup(l);
    assert_eq!(
        (r.game, r.branch, r.strength, r.threshold),
        (game, branch, strength, 100)
    );
    let hand = strength >= 100;
    assert_eq!(r.action, if hand { Action::Hand } else { Action::TakeSkat });
    orient(l, branch);
    assert!(l.check(Step::Threshold, Answer::Within(hand)));
    assert!(!l.check(Step::Threshold, Answer::Within(!hand)));
    assert!(!l.check(Step::Threshold, Answer::Holds(hand)));
}

fn expect_discard(
    l: &PreLesson,
    game: GameType,
    branch: GameBranch,
    put: [(&str, Keep); 2],
    eligible: &str,
) {
    let r = discard(l);
    assert_eq!((r.game, r.branch), (game, branch));
    let want: CardSet = put.iter().map(|p| card(p.0)).collect();
    assert_eq!(r.action, Action::Discard(want));
    let mut reasons = r.reasons;
    reasons.sort_by_key(|p| put.iter().position(|q| card(q.0) == p.0));
    assert_eq!(reasons, put.map(|(c, k)| (card(c), k)));
    assert_eq!(r.eligible, cards(eligible));
    orient(l, branch);
    for c in CardSet::ALL {
        assert_eq!(
            l.check(Step::Threshold, Answer::Card(c)),
            r.eligible.contains(c)
        );
    }
    assert!(!l.check(Step::Threshold, Answer::Within(true)));
}

/// Herz: CJ 18, HA 11, HT 11, HK HQ H9 H8 H7 5 × 8 = 40 → Trumpf 80; side CA
/// 10, SA 10; Karo and the Herz side empty 2 × 3 → 106: Hand. One Bube, so no
/// Grand.
const SUIT_HAND: &str = "CJ HA HT HK HQ H9 H8 H7 CA SA";

#[test]
fn hand_chosen_and_one_changed_card_flips_to_pickup() {
    let (_, l) = at(deal_hh(SUIT_HAND, "DQ DK"), &[]);
    expect_pickup(&l, GameType::Farbe(Suit::Herz), GameBranch::Suit, 106);
    // Changed condition: S7 instead of SA loses the side Ace (−10): 96 < 100.
    let (_, l) = at(deal_hh("CJ HA HT HK HQ H9 H8 H7 CA S7", "DQ DK"), &[]);
    expect_pickup(&l, GameType::Farbe(Suit::Herz), GameBranch::Suit, 96);
}

#[test]
fn grand_hand_and_grand_pickup() {
    // Four Buben 18 + 3 × 14 = 60; Kreuz CA 10 + CT 6 (with Ace); Pik SA 10
    // + ST 6; Herz HA 10 → 102: Grand Hand.
    let (_, l) = at(deal_hh("CJ SJ HJ DJ CA CT CK SA ST HA", "DQ DK"), &[]);
    expect_pickup(&l, GameType::Grand, GameBranch::Grand, 102);
    // Buben 60; CA 10; HA 10 + HT 6 → 86: Grand (≥ 70), but below Hand.
    let (_, l) = at(deal_hh("CJ SJ HJ DJ HA HT HK HQ H9 CA", "S7 D7"), &[]);
    expect_pickup(&l, GameType::Grand, GameBranch::Grand, 86);
    // Drücken in Grand: Buben, Aces and the guarded HT stay; S7 and D7 are
    // both singletons (20 − 0), the five-card Herz cards rank far higher.
    let (_, l) = at(
        deal_hh("CJ SJ HJ DJ HA HT HK HQ H9 CA", "S7 D7"),
        &[Action::TakeSkat],
    );
    expect_discard(
        &l,
        GameType::Grand,
        GameBranch::Grand,
        [
            ("S7", Keep::Side { suit: 1 }),
            ("D7", Keep::Side { suit: 1 }),
        ],
        "S7 D7",
    );
}

#[test]
fn suit_discard_keeps_trumpf_ace_and_guarded_ten() {
    // Ten cards: Herz Trumpf CJ 18, HA HT 22, HK HQ H9 H8 32 = 72; CA 10, CT
    // 6 (with Ace); Karo and the Herz side empty 2 × 3 → 94: pickup.
    let d = deal_hh("CJ HA HT HK HQ H9 H8 CA CT S7", "ST D8");
    let (_, l) = at(d, &[]);
    expect_pickup(&l, GameType::Farbe(Suit::Herz), GameBranch::Suit, 94);
    // Twelve cards with ST D8: Herz stays the candidate. Kept: Trumpf, CA,
    // CT guarded by CA. Pik S7 ST (2 cards): S7 40 − 0, the loose ST 40 − 10;
    // Karo D8 alone: 20 − 0. Away: D8 (20) and ST (30).
    let (g, l) = at(d, &[Action::TakeSkat]);
    assert_eq!(g.phase(), Phase::Druecken);
    expect_discard(
        &l,
        GameType::Farbe(Suit::Herz),
        GameBranch::Suit,
        [
            ("D8", Keep::Side { suit: 1 }),
            ("ST", Keep::Side { suit: 2 }),
        ],
        "D8 ST",
    );
}

#[test]
fn null_pickup_and_null_discard_put_away_the_highest_cards() {
    // No Buben, no suit reaches 60; every suit is Null-safe: Null, strength
    // 0, so Easy never plays Hand from the strength count.
    let d = deal_hh("C7 C8 C9 S7 S8 S9 H7 H8 H9 D7", "DA HK");
    let (_, l) = at(d, &[]);
    expect_pickup(&l, GameType::Null, GameBranch::Null, 0);
    // With DA HK only Karo (D7 DA) is unsafe: still Null. DA beats seven
    // Karo cards, HK six Herz cards: they go first.
    let (_, l) = at(d, &[Action::TakeSkat]);
    expect_discard(
        &l,
        GameType::Null,
        GameBranch::Null,
        [("DA", Keep::Null), ("HK", Keep::Null)],
        "DA HK",
    );
}

#[test]
fn hidden_skat_never_changes_the_hand_decision() {
    let a = at(deal_hh(SUIT_HAND, "DQ DK"), &[]).1;
    let b = at(deal_hh(SUIT_HAND, "CK SK"), &[]).1;
    assert_eq!(a, b);
}

#[test]
fn unavailable_wanted_action_is_disclosed_as_fallback() {
    let mut g = Game::from_deal(deal_hh(SUIT_HAND, "DQ DK"), RuleConfig::default());
    for a in [Action::Pass, Action::Bid(18), Action::Pass] {
        g.apply(a).unwrap();
    }
    let mut view = g.view(Seat::Hinterhand);
    view.legal_actions.retain(|&a| a != Action::Hand);
    let l = pre_lesson(&view).unwrap();
    assert!(l.rule.fallback());
    assert_eq!(l.rule.action(), Action::TakeSkat);
    // Ansage has its own lesson (guided_announce.rs).
    g.apply(Action::Hand).unwrap();
    assert_eq!(g.phase(), Phase::Ansage);
    assert!(matches!(
        pre_lesson(&g.view(Seat::Hinterhand)).unwrap().rule,
        PreRule::Announce(_)
    ));
}
