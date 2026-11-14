// SPDX-License-Identifier: EUPL-1.2

//! Core-accepted Ansage fixtures for the guided dialogue (#131 phase 4).
//! Expected values and strengths are worked out by hand in the comments,
//! independent of the rule: plain value = base × (Spitzen + 1, + 1 for Hand)
//! with base Kreuz 12, Pik 11, Herz 10, Karo 9, Grand 24; Null 23, Null Hand
//! 35. Strength as in `guided_preplay.rs`: Trumpf Bube 14 (Kreuz 18), Trumpf
//! Ass/Zehn 11, other Trumpf 8; side Ace 10; side Ten 6 with its Ace or two
//! more cards, else 1; empty side suit 3 (not in Grand). Easy announces the
//! preferred game if its value reaches the bid; otherwise the fitting suit
//! game or Grand with the highest strength (the later one in the order
//! Kreuz, Pik, Herz, Karo, Grand on equal strength); Grand if nothing fits.
//! Null without an unsafe suit among the ten cards is played open.

#[path = "support/guided.rs"]
#[allow(dead_code)]
mod support;
use skat_coach::guided::*;
use skat_core::{Action, Announcement, Deal, Game, GameType, Phase, RuleConfig, Seat};
use support::*;

const KREUZ: GameType = GameType::Farbe(skat_core::Suit::Kreuz);
const PIK: GameType = GameType::Farbe(skat_core::Suit::Pik);
const HERZ: GameType = GameType::Farbe(skat_core::Suit::Herz);

/// HH wins the bidding at `bid` (core-checked), takes `after` and reaches Ansage.
fn at(deal: Deal, bid: u16, after: &[Action]) -> (Game, PreLesson, AnnounceRule) {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    for a in [Action::Pass, Action::Bid(bid), Action::Pass] {
        g.apply(a).unwrap();
    }
    for &a in after {
        g.apply(a).unwrap();
    }
    assert_eq!(g.phase(), Phase::Ansage);
    let view = g.view(g.current_seat().unwrap());
    assert_eq!(view.seat, Seat::Hinterhand);
    let l = pre_lesson(&view).expect("declarer Ansage decision");
    let PreRule::Announce(r) = l.rule else {
        panic!("announce expected");
    };
    // Every legal announcement stays available and accepted by the core.
    for a in &view.legal_actions {
        g.clone().apply(*a).unwrap();
    }
    assert!(view.legal_actions.contains(&r.action));
    assert!(!r.fallback && !r.plain);
    assert_eq!((r.bid, r.hand_game), (bid, after == [Action::Hand]));
    (g, l, r)
}

fn plain(game: GameType) -> Action {
    Action::Announce(Announcement::plain(game))
}

/// Orientation accepts exactly the branch; the threshold exactly the fit.
fn checks(l: &PreLesson, branch: GameBranch, fits: bool) {
    for b in [
        GameBranch::Grand,
        GameBranch::Suit,
        GameBranch::Null,
        GameBranch::Weak,
    ] {
        assert_eq!(l.check(Step::Orient, Answer::Game(b)), b == branch);
    }
    assert!(l.check(Step::Threshold, Answer::Within(fits)));
    assert!(!l.check(Step::Threshold, Answer::Within(!fits)));
    assert!(!l.check(Step::Threshold, Answer::Holds(fits)));
    assert!(!l.check(Step::Threshold, Answer::Card(card("CJ"))));
}

fn candidate(r: &AnnounceRule, game: GameType) -> (u16, i32, bool) {
    let c = r.candidates.iter().find(|c| c.game == game).unwrap();
    (c.value, c.strength, c.fits)
}

/// Herz Hand (strength 106, see `guided_preplay.rs`): mit 1 (CJ, no SJ).
const SUIT_HAND: &str = "CJ HA HT HK HQ H9 H8 H7 CA SA";

#[test]
fn suit_hand_preferred_game_reaches_the_bid() {
    // Herz Hand: 10 × (1 + 1 + 1) = 30 ≥ 18. Hand: the Skat stays unknown.
    let (_, l, r) = at(deal_hh(SUIT_HAND, "DQ DK"), 18, &[Action::Hand]);
    assert_eq!(
        (r.preferred, r.branch, r.value),
        (HERZ, GameBranch::Suit, 30)
    );
    assert!(r.fits && r.skat.is_empty());
    assert_eq!((r.game, r.action), (HERZ, plain(HERZ)));
    assert!(r.tied().is_empty() && !r.no_fit && r.null_risks.is_none());
    checks(&l, GameBranch::Suit, true);
}

#[test]
fn higher_bid_switches_to_the_strongest_fitting_candidate_with_tie() {
    // Bid 33 > Herz Hand 30. Candidates (mit 1, Hand): Kreuz 36, Pik 33,
    // Herz 30, Karo 27, Grand 72. Strength Kreuz: CJ 18 + CA 11 = 29; SA 10;
    // HA 10 + HT 6; Karo and Kreuz side empty 6 → 61. Pik: CJ 18 + SA 11; CA
    // 10; Herz 16; Karo and Pik side empty → 61. Grand: CJ 18; three Aces 30
    // + HT 6 → 54. Kreuz and Pik tie at 61: the later one, Pik, is announced.
    let (_, l, r) = at(deal_hh(SUIT_HAND, "DQ DK"), 33, &[Action::Hand]);
    assert_eq!((r.preferred, r.value, r.fits), (HERZ, 30, false));
    assert_eq!(candidate(&r, KREUZ), (36, 61, true));
    assert_eq!(candidate(&r, PIK), (33, 61, true));
    assert_eq!(candidate(&r, HERZ), (30, 106, false));
    // Karo: 9 × 3 = 27; CJ 18, CA SA 20, HA HT 16, empty Karo side 3 → 57.
    assert_eq!(
        candidate(&r, GameType::Farbe(skat_core::Suit::Karo)),
        (27, 57, false)
    );
    assert_eq!(candidate(&r, GameType::Grand), (72, 54, true));
    assert_eq!(r.tied(), vec![KREUZ, PIK]);
    assert_eq!((r.game, r.action), (PIK, plain(PIK)));
    checks(&l, GameBranch::Suit, false);

    // Bid 36: Pik 33 no longer fits; Kreuz 61 beats Grand 54, no tie.
    let (_, _, r) = at(deal_hh(SUIT_HAND, "DQ DK"), 36, &[Action::Hand]);
    assert_eq!((r.game, r.tied()), (KREUZ, vec![]));

    // Bid 77 > Grand Hand 72: nothing fits, Easy falls back to Grand.
    let (_, l, r) = at(deal_hh(SUIT_HAND, "DQ DK"), 77, &[Action::Hand]);
    assert!(r.no_fit && r.candidates.iter().all(|c| !c.fits));
    assert_eq!(r.action, plain(GameType::Grand));
    checks(&l, GameBranch::Suit, false);
}

#[test]
fn changed_hand_condition_makes_the_preferred_game_fit() {
    // HJ instead of CJ: Herz ohne 2 (no CJ, no SJ) Hand = 10 × 4 = 40 ≥ 33;
    // strength HJ 14 + 22 + 40, CA SA 20, two empty suits 6 → 102, still Herz.
    let d = deal_hh("HJ HA HT HK HQ H9 H8 H7 CA SA", "DQ DK");
    let (_, l, r) = at(d, 33, &[Action::Hand]);
    assert_eq!((r.preferred, r.value, r.fits), (HERZ, 40, true));
    assert_eq!(r.action, plain(HERZ));
    checks(&l, GameBranch::Suit, true);
}

#[test]
fn grand_after_pickup_counts_the_known_skat() {
    // Pickup S7 D7 and put both away (Easy's own discard, see
    // guided_preplay.rs). Four Buben, strength 86 ≥ 70: Grand mit 4 =
    // 24 × 5 = 120 ≥ 18.
    let d = deal_hh("CJ SJ HJ DJ HA HT HK HQ H9 CA", "S7 D7");
    let after = [Action::TakeSkat, Action::Discard(cards("S7 D7"))];
    let (_, l, r) = at(d, 18, &after);
    assert_eq!(r.skat, cards("S7 D7"));
    assert_eq!(
        (r.preferred, r.branch, r.value),
        (GameType::Grand, GameBranch::Grand, 120)
    );
    assert_eq!(r.action, plain(GameType::Grand));
    checks(&l, GameBranch::Grand, true);
}

/// All four suits Null-safe (strength below 60, no Buben).
const NULL_HAND: &str = "C7 C8 C9 S7 S8 S9 H7 H8 H9 D7";

#[test]
fn null_without_risk_is_played_open() {
    // Null Hand 35 ≥ 18; no unsafe suit: Null ouvert Hand.
    let (_, l, r) = at(deal_hh(NULL_HAND, "DA HK"), 18, &[Action::Hand]);
    assert_eq!(
        (r.preferred, r.branch, r.value),
        (GameType::Null, GameBranch::Null, 35)
    );
    assert_eq!((r.null_risks, r.ouvert), (Some(0), true));
    let ouvert = Announcement {
        ouvert: true,
        ..Announcement::plain(GameType::Null)
    };
    assert_eq!(r.action, Action::Announce(ouvert));
    checks(&l, GameBranch::Null, true);
}

#[test]
fn null_with_one_unsafe_suit_is_played_plain_and_a_higher_bid_leaves_null() {
    // Pickup DA HK, put away HK C9: ten cards keep D7 DA (Karo unsafe: the
    // second card DA is above the third Null rank). Twelve cards: only Karo
    // unsafe, no suit reaches 60 → Null 23 ≥ 18, one risk: plain Null.
    let d = deal_hh(NULL_HAND, "DA HK");
    let after = [Action::TakeSkat, Action::Discard(cards("HK C9"))];
    let (_, l, r) = at(d, 18, &after);
    assert_eq!((r.preferred, r.value, r.fits), (GameType::Null, 23, true));
    assert_eq!((r.null_risks, r.ouvert), (Some(1), false));
    assert_eq!(r.action, plain(GameType::Null));
    checks(&l, GameBranch::Null, true);

    // Bid 24 > Null 23. Without Buben every game is "ohne" and fits: Kreuz
    // ohne 8 (C9 is the first held Trumpf) 12 × 9 = 108, Pik 11 × 9 = 99,
    // Herz ohne 6 (HK) 10 × 7 = 70, Karo ohne 4 (DA) 9 × 5 = 45, Grand ohne
    // 4 = 120. Strength (twelve cards): Kreuz C7 C8 C9 24 + DA 10 + empty
    // Kreuz side 3 = 37; Pik 37; Herz H7 H8 H9 HK 32 + 10 + 3 = 45; Karo D7
    // 8 + DA 11 + 3 = 22; Grand DA 10 → Herz.
    let (_, l, r) = at(d, 24, &after);
    assert_eq!((r.preferred, r.fits), (GameType::Null, false));
    assert_eq!(candidate(&r, HERZ), (70, 45, true));
    assert_eq!(candidate(&r, KREUZ), (108, 37, true));
    assert_eq!((r.game, r.action, r.null_risks), (HERZ, plain(HERZ), None));
    checks(&l, GameBranch::Null, false);
}

#[test]
fn hidden_cards_never_change_the_announcement_lesson() {
    // Same permitted view, different unknown Skat (Hand) or opponent cards.
    let a = at(deal_hh(SUIT_HAND, "DQ DK"), 33, &[Action::Hand]).1;
    let b = at(deal_hh(SUIT_HAND, "CK SK"), 33, &[Action::Hand]).1;
    assert_eq!(a, b);
    let d = deal_hh("CJ SJ HJ DJ HA HT HK HQ H9 CA", "S7 D7");
    let after = [Action::TakeSkat, Action::Discard(cards("S7 D7"))];
    let swapped = swap(d, card("C7"), card("DA"));
    assert_ne!(d.hand(Seat::Vorhand), swapped.hand(Seat::Vorhand));
    assert_eq!(at(d, 18, &after).1, at(swapped, 18, &after).1);
}

#[test]
fn unavailable_wanted_action_and_unsupported_phases_are_distinct() {
    let mut g = Game::from_deal(deal_hh(NULL_HAND, "DA HK"), RuleConfig::default());
    for a in [Action::Pass, Action::Bid(18), Action::Pass, Action::Hand] {
        g.apply(a).unwrap();
    }
    let view = g.view(Seat::Hinterhand);
    // Wanted Null ouvert unavailable: the rule's own plain Null fallback.
    let mut v = view.clone();
    v.legal_actions
        .retain(|a| !matches!(a, Action::Announce(x) if x.ouvert && x.game == GameType::Null));
    let PreRule::Announce(r) = pre_lesson(&v).unwrap().rule else {
        panic!("announce expected");
    };
    assert!(r.ouvert && r.plain && !r.fallback);
    assert_eq!(r.action, plain(GameType::Null));
    // Plain Null unavailable as well: the caller's first-legal fallback.
    v.legal_actions
        .retain(|a| !matches!(a, Action::Announce(x) if x.game == GameType::Null));
    let l = pre_lesson(&v).unwrap();
    assert!(l.rule.fallback());
    assert_eq!(l.rule.action(), v.legal_actions[0]);
    // Rule fallback (no fit) is a supported lesson; Spiel has no Ansage rule.
    g.apply(plain(GameType::Null)).unwrap();
    assert_eq!(g.phase(), Phase::Spiel);
    let v = g.view(g.current_seat().unwrap());
    assert!(pre_lesson(&v).is_none());
    assert!(skat_analysis::policy::announce_rule(&v).is_none());
}
