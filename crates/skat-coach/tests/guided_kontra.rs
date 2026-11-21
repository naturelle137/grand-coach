// SPDX-License-Identifier: EUPL-1.2

//! Core-accepted Kontra/Re fixtures for the guided dialogue (#153, #155,
//! #131 phase 4). Every position is reached by legal core actions from a fixed
//! deal; no PlayerView is hand-made except the labelled restricted views.
//!
//! Expected values are worked out by hand, independent of the rule. Easy says
//! Kontra as a defender with at least five Trumpf in its own hand (never in
//! Null). It answers Kontra with Re from strength 90 for its own hand plus its
//! known Skat (the discard; none in a Hand game). Strength: Trumpf Bube 14
//! (Kreuz 18), Trumpf Ass/Zehn 11, other Trumpf 8; side Ace 10; side Ten 6
//! with its Ace or two more cards, else 1; empty side suit 3 (not in Grand;
//! in a suit game the trump suit's side part is always empty, so +3).
//!
//! Bidding (core-checked): MH bids 18, VH holds, MH passes, HH bids 20, VH
//! passes. HH is declarer; VH (held) and MH (bid) may say Kontra (D-26.1).
//! VH leads the first trick, so Kontra is offered to VH at its first card.

#[path = "support/guided.rs"]
#[allow(dead_code)]
mod support;
use skat_coach::guided::*;
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Event, Game, GameType, KontraLevel, Phase,
    PlayerView, RuleConfig, Seat, Suit,
};
use support::*;

const HERZ: GameType = GameType::Farbe(Suit::Herz);
const KREUZ: GameType = GameType::Farbe(Suit::Kreuz);
const ON: RuleConfig = RuleConfig { kontra_re: true };

/// VH holds `vh`, HH holds `hh`, the Skat is `skat`; MH gets the rest.
fn deal_vh(vh: &str, hh: &str, skat: &str) -> Deal {
    let (vh, hh, skat) = (cards(vh), cards(hh), cards(skat));
    let mh = CardSet::ALL - vh - hh - skat;
    Deal::new([vh, mh, hh], skat).unwrap()
}

/// The bidding above, then `after` (Hand or pickup and discard) and `game`.
fn play_phase(deal: Deal, rules: RuleConfig, after: &[Action], game: Announcement) -> Game {
    let mut g = Game::from_deal(deal, rules);
    for a in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Bid(20),
        Action::Pass,
    ] {
        g.apply(a).unwrap();
    }
    assert_eq!(g.phase(), Phase::SkatAufnahme, "HH declares");
    for &a in after {
        g.apply(a).unwrap();
    }
    g.apply(Action::Announce(game)).unwrap();
    g
}

fn hand(game: GameType, deal: Deal) -> Game {
    play_phase(deal, ON, &[Action::Hand], Announcement::plain(game))
}

fn current(g: &Game) -> PlayerView {
    g.view(g.current_seat().unwrap())
}

/// Every legal action stays accepted by the core.
fn all_free(g: &Game, view: &PlayerView) {
    for &a in &view.legal_actions {
        g.clone().apply(a).unwrap();
    }
}

/// Orientation accepts exactly the factor; the threshold exactly the condition.
fn checks(l: &KontraLesson, factor: i32, wants: bool) {
    for f in [1, 2, 4, 8] {
        assert_eq!(l.check(Step::Orient, Answer::Factor(f)), f == factor);
    }
    assert!(l.check(Step::Threshold, Answer::Within(wants)));
    assert!(!l.check(Step::Threshold, Answer::Within(!wants)));
    assert!(!l.check(Step::Orient, Answer::Within(wants)));
    assert!(!l.check(Step::Threshold, Answer::Factor(factor)));
    assert!(!l.check(Step::Threshold, Answer::Holds(wants)));
}

/// VH: CJ HA H7 H8 + side SA ST S9 DA D7, and `fifth` (another Herz or a side card).
fn vh(fifth: &str) -> String {
    format!("CJ HA H7 H8 SA ST S9 DA D7 {fifth}")
}
const HH: &str = "SJ HJ DJ HT HK HQ CA CT C9 C8";

#[test]
fn defender_with_five_trumpf_is_offered_kontra_and_easy_says_it() {
    // Herz: Trumpf = four Buben + seven Herz. VH: CJ HA H7 H8 H9 = 5 ≥ 5.
    let g = hand(HERZ, deal_vh(&vh("H9"), HH, "D8 D9"));
    let view = current(&g);
    assert_eq!(view.seat, Seat::Vorhand);
    assert!(view.legal_actions.contains(&Action::Kontra));
    all_free(&g, &view);
    let l = kontra_lesson(&view).expect("Farbspiel Kontra opportunity");
    let r = &l.rule;
    assert_eq!(
        (r.call, r.game, r.level),
        (KontraCall::Kontra, HERZ, KontraLevel::None)
    );
    assert_eq!((r.count, r.threshold, r.wants), (5, 5, true));
    assert_eq!((r.action, r.card.as_ref()), (Action::Kontra, None));
    assert!(!r.null && !r.ouvert && !r.fallback && r.skat.is_empty());
    assert_eq!(r.legal, view.legal_actions);
    assert_eq!(kontra_exclusion(&view), None);
    assert_eq!(l.factor(), 2);
    checks(&l, 2, true);
    // The ordinary card lesson stays out of a Kontra decision.
    assert!(lesson(&view).is_none());
}

#[test]
fn defender_with_four_trumpf_declines_with_easys_exact_card() {
    // S8 instead of H9: CJ HA H7 H8 = 4 < 5. Easy plays a card. Leading as a
    // defender, it leads the side master with most Augen, the last one on a
    // tie in core legal order: masters SA and DA (11 Augen each) → DA.
    let g = hand(HERZ, deal_vh(&vh("S8"), HH, "D8 D9"));
    let view = current(&g);
    let l = kontra_lesson(&view).unwrap();
    let r = &l.rule;
    assert_eq!((r.count, r.threshold, r.wants), (4, 5, false));
    let rule = r.card.as_ref().expect("declining card rule");
    assert_eq!(rule.card, card("DA"));
    assert_eq!(rule.branch, Branch::LeadMaster);
    assert_eq!(r.action, Action::Play(card("DA")));
    // The same card as Easy's legal card path with Kontra unavailable.
    let mut plain = view.clone();
    plain.legal_actions.retain(|a| *a != Action::Kontra);
    assert_eq!(skat_analysis::baseline::play(&plain), r.action);
    checks(&l, 2, false);
    all_free(&g, &view);
}

#[test]
fn another_trump_suit_counts_its_own_trumpf_and_the_buben() {
    // Kreuz: CJ SJ C7 C8 C9 = 5 Trumpf → Kontra. The same hand against
    // Herz holds only CJ SJ (2, no Herz card) → a card.
    let vh = "CJ SJ C7 C8 C9 SA ST S9 DA D7";
    let hh = "HJ DJ HA HT HK HQ CA CT H9 H8";
    let g = hand(KREUZ, deal_vh(vh, hh, "D8 D9"));
    let r = kontra_lesson(&current(&g)).unwrap().rule;
    assert_eq!(
        (r.game, r.count, r.wants, r.action),
        (KREUZ, 5, true, Action::Kontra)
    );
    let g = hand(HERZ, deal_vh(vh, hh, "D8 D9"));
    let r = kontra_lesson(&current(&g)).unwrap().rule;
    assert_eq!((r.game, r.count, r.wants), (HERZ, 2, false));
    assert!(matches!(r.action, Action::Play(_)));
}

#[test]
fn unseen_cards_do_not_change_the_kontra_lesson() {
    // VH's permitted input is its own hand and public bidding; MH, HH and the
    // Skat are unseen. Swapping unseen cards keeps rule and dialogue equal.
    let base = deal_vh(&vh("H9"), HH, "D8 D9");
    let expected = kontra_lesson(&current(&hand(HERZ, base))).unwrap();
    let mh = base.hand(Seat::Mittelhand).iter().next().unwrap();
    for (a, b) in [(card("HJ"), mh), (card("D8"), mh), (card("CA"), card("D9"))] {
        let view = current(&hand(HERZ, swap(base, a, b)));
        assert_eq!(view.hand, base.hand(Seat::Vorhand));
        assert_eq!(kontra_lesson(&view).unwrap(), expected, "{a:?} {b:?}");
    }
}

/// HH declarer, `after` (Hand or pickup), Herz; VH says Kontra; HH answers.
fn re_at(deal: Deal, after: &[Action]) -> (Game, KontraLesson) {
    let mut g = play_phase(deal, ON, after, Announcement::plain(HERZ));
    g.apply(Action::Kontra).unwrap();
    let view = current(&g);
    assert_eq!(view.seat, Seat::Hinterhand);
    assert_eq!(view.legal_actions, vec![Action::Re, Action::KeinRe]);
    all_free(&g, &view);
    let l = kontra_lesson(&view).expect("immediate Re reply");
    assert_eq!(
        (l.rule.call, l.rule.level),
        (KontraCall::Re, KontraLevel::Kontra)
    );
    assert_eq!(l.factor(), 4);
    assert!(!l.rule.fallback && l.rule.card.is_none());
    (g, l)
}

/// Herz Hand cards: CJ 18 + SJ 14 + HJ 14 + HA 11 + HT 11 + H9 8 = 76; Herz
/// side empty 3; Pik {SA} 10; Kreuz {CT} alone 1; Karo {D7 D8} 0 → 90.
const RE_90: &str = "CJ SJ HJ HA HT H9 SA CT D7 D8";
/// C7 instead of CT: Kreuz {C7} 0 → 89.
const RE_89: &str = "CJ SJ HJ HA HT H9 SA C7 D7 D8";

/// HH holds `hh`, the Skat `skat`; VH and MH get the rest in card order.
/// VH's Kontra is a legal fixture action, not Easy's choice.
fn deal_hh_vh(hh: &str, skat: &str) -> Deal {
    let (hh, skat) = (cards(hh), cards(skat));
    let rest: Vec<Card> = (CardSet::ALL - hh - skat).iter().collect();
    let vh: CardSet = rest[..10].iter().copied().collect();
    let mh: CardSet = rest[10..].iter().copied().collect();
    Deal::new([vh, mh, hh], skat).unwrap()
}

#[test]
fn hand_declarer_re_at_and_below_the_strength_threshold() {
    let (_, l) = re_at(deal_hh_vh(RE_90, "D9 DT"), &[Action::Hand]);
    let r = &l.rule;
    assert!(r.hand_game && r.skat.is_empty());
    assert_eq!(
        (r.count, r.threshold, r.wants, r.action),
        (90, 90, true, Action::Re)
    );
    checks(&l, 4, true);
    let (_, l) = re_at(deal_hh_vh(RE_89, "D9 DT"), &[Action::Hand]);
    let r = &l.rule;
    assert_eq!((r.count, r.wants, r.action), (89, false, Action::KeinRe));
    checks(&l, 4, false);
}

#[test]
fn hand_unknown_skat_and_unseen_hands_do_not_change_the_re_lesson() {
    // In a Hand game the Skat stays unknown: a Skat with SA's partner ST or
    // the CA (which would raise Kreuz/Pik strength) changes nothing.
    let base = deal_hh_vh(RE_89, "D9 DT");
    let (_, expected) = re_at(base, &[Action::Hand]);
    for (a, b) in [(card("D9"), card("ST")), (card("DT"), card("CA"))] {
        let (_, l) = re_at(swap(base, a, b), &[Action::Hand]);
        assert_eq!(l, expected, "{a:?} {b:?}");
    }
}

#[test]
fn pickup_known_discard_decides_re() {
    // Pickup: HH holds RE_89 minus D8 plus CT, picks up D8 D9 and discards.
    // Discard CT D9: held cards = RE_89; with the known Skat Kreuz {C7 CT}
    // (Ten without Ace, two cards) 1 → 90 → Re.
    let initial = "CJ SJ HJ HA HT H9 SA C7 D7 CT";
    let (_, l) = re_at(
        deal_hh_vh(initial, "D8 D9"),
        &[Action::TakeSkat, Action::Discard(cards("CT D9"))],
    );
    let r = &l.rule;
    assert!(!r.hand_game);
    assert_eq!(r.skat, cards("CT D9"));
    assert_eq!((r.count, r.wants, r.action), (90, true, Action::Re));
    // Same ten held cards, discard C8 D9 (C8 instead of CT in the deal):
    // Kreuz {C7 C8} 0 → 89 → KeinRe.
    let initial = "CJ SJ HJ HA HT H9 SA C7 D7 C8";
    let (g, l) = re_at(
        deal_hh_vh(initial, "D8 D9"),
        &[Action::TakeSkat, Action::Discard(cards("C8 D9"))],
    );
    assert_eq!(current(&g).hand, cards(RE_89));
    assert_eq!(
        (l.rule.count, l.rule.wants, l.rule.action),
        (89, false, Action::KeinRe)
    );
}

#[test]
fn ineligible_bidding_off_setting_first_card_and_existing_kontra_offer_no_lesson() {
    // Ineligible: MH passes at once, HH bids 18, VH passes; neither defender
    // bid or held, so VH's lead offers only cards.
    let mut g = Game::from_deal(deal_vh(&vh("H9"), HH, "D8 D9"), ON);
    for a in [Action::Pass, Action::Bid(18), Action::Pass, Action::Hand] {
        g.apply(a).unwrap();
    }
    g.apply(Action::Announce(Announcement::plain(HERZ)))
        .unwrap();
    let view = current(&g);
    assert!(!view.legal_actions.contains(&Action::Kontra));
    assert!(kontra_lesson(&view).is_none() && kontra_exclusion(&view).is_none());

    // Optional setting off: the same eligible bidding offers no Kontra.
    let off = play_phase(
        deal_vh(&vh("H9"), HH, "D8 D9"),
        RuleConfig::default(),
        &[Action::Hand],
        Announcement::plain(HERZ),
    );
    let view = current(&off);
    assert!(!view.legal_actions.contains(&Action::Kontra));
    assert!(kontra_lesson(&view).is_none() && kontra_exclusion(&view).is_none());

    // After its first card VH loses the opportunity; MH (eligible, not yet
    // played) is offered Kontra in the same trick.
    let mut g = hand(HERZ, deal_vh(&vh("H9"), HH, "D8 D9"));
    g.apply(Action::Play(card("DA"))).unwrap();
    let mh = current(&g);
    assert_eq!(mh.seat, Seat::Mittelhand);
    assert!(kontra_lesson(&mh).is_some());
    let mh_card = mh
        .legal_actions
        .iter()
        .copied()
        .find(|a| matches!(a, Action::Play(_)))
        .unwrap();
    g.apply(mh_card).unwrap();
    let hh_card = current(&g).legal_actions[0];
    g.apply(hh_card).unwrap();
    while current(&g).seat != Seat::Vorhand {
        let a = current(&g).legal_actions[0];
        g.apply(a).unwrap();
    }
    let view = current(&g);
    assert!(!view.legal_actions.contains(&Action::Kontra));
    assert!(kontra_lesson(&view).is_none() && kontra_exclusion(&view).is_none());

    // Already Kontra / Re: MH is offered nothing after VH's Kontra and HH's reply.
    for reply in [Action::Re, Action::KeinRe] {
        let mut g = hand(HERZ, deal_vh(&vh("H9"), HH, "D8 D9"));
        g.apply(Action::Kontra).unwrap();
        g.apply(reply).unwrap();
        g.apply(Action::Play(card("DA"))).unwrap();
        let mh = current(&g);
        assert_eq!(mh.seat, Seat::Mittelhand);
        assert!(!mh.legal_actions.contains(&Action::Kontra));
        assert!(kontra_lesson(&mh).is_none() && kontra_exclusion(&mh).is_none());
    }
}

#[test]
fn ouvert_gets_honest_exclusions_without_hidden_cards() {
    let deal = deal_vh(&vh("H9"), HH, "D8 D9");
    // Null ouvert: the separate round before the reveal offers KeinKontra;
    // the declarer's cards stay hidden from the defender's view.
    let ouvert = Announcement {
        ouvert: true,
        ..Announcement::plain(GameType::Null)
    };
    let mut g = play_phase(deal, ON, &[Action::Hand], ouvert);
    let view = current(&g);
    assert_eq!(view.legal_actions, vec![Action::Kontra, Action::KeinKontra]);
    assert_eq!(view.declarer_cards, None);
    assert_eq!(kontra_exclusion(&view), Some(KontraExclusion::OuvertRound));
    assert!(kontra_lesson(&view).is_none());
    g.apply(Action::Kontra).unwrap();
    let view = current(&g);
    assert_eq!(view.legal_actions, vec![Action::Re, Action::KeinRe]);
    let lesson = kontra_lesson(&view).expect("Null Ouvert Re is guided");
    assert_eq!(kontra_exclusion(&view), None);
    assert!(lesson.rule.null && lesson.rule.ouvert && !lesson.rule.wants);
    assert_eq!(lesson.rule.action, Action::KeinRe);
    checks_null(&lesson, 4);
    all_free(&g, &view);
}

/// Ouvert Hand game by HH, VH says Kontra in the round before the reveal; the
/// declarer's Re/KeinRe reply (Ouvert Farbspiel and Grand, #271).
fn ouvert_re(game: GameType, deal: Deal) -> (Game, PlayerView) {
    let ann = Announcement {
        schneider: true,
        schwarz: true,
        ouvert: true,
        ..Announcement::plain(game)
    };
    let mut g = play_phase(deal, ON, &[Action::Hand], ann);
    assert!(kontra_lesson(&current(&g)).is_some());
    g.apply(Action::Kontra).unwrap();
    let view = current(&g);
    (g, view)
}

#[test]
fn ouvert_farbspiel_and_grand_re_reply_is_guided_from_the_own_view() {
    // Positive, boundary (90) and counterexample (89): the count uses only the
    // declarer's own cards; the open declarer hand is never an input.
    for (game, hh, skat, wants, action) in [
        (HERZ, RE_90, "D9 DT", true, Action::Re),
        (HERZ, RE_89, "D9 DT", false, Action::KeinRe),
        (GRAND, GRAND_90, "D9 D8", true, Action::Re),
        (GRAND, GRAND_89, "D9 D8", false, Action::KeinRe),
    ] {
        let (g, view) = ouvert_re(game, deal_hh_vh(hh, skat));
        assert_eq!(view.seat, Seat::Hinterhand);
        all_free(&g, &view);
        let l = kontra_lesson(&view).expect("Ouvert Re reply is guided");
        let r = &l.rule;
        assert!(r.ouvert && r.hand_game && !r.null && !r.fallback);
        assert_eq!((r.wants, r.action), (wants, action), "{game:?}");
        assert_eq!(r.threshold, 90);
        assert_eq!(kontra_exclusion(&view), None);
        checks(&l, 4, wants);
        // Swapping the defenders' hidden cards leaves the dialogue unchanged.
        let hh_set = cards(hh);
        let swapped = {
            let skat = cards(skat);
            let rest: Vec<Card> = (CardSet::ALL - hh_set - skat).iter().collect();
            let mh: CardSet = rest[..10].iter().copied().collect();
            let vh: CardSet = rest[10..].iter().copied().collect();
            Deal::new([vh, mh, hh_set], skat).unwrap()
        };
        let (_, other) = ouvert_re(game, swapped);
        assert_eq!(kontra_lesson(&other).unwrap().rule, l.rule);
    }
}

// Grand and Null (#155). Grand: only the four Buben are Trumpf, so a
// defender holds at most four and Easy's five-Trumpf Kontra never holds.
// Null: no Trumpf; Easy never says Kontra and its strength is 0, so it never
// says Re. The Null threshold asks whether the generic rule applies at all.

/// Grand/Null threshold: Grand checks the count condition; Null checks rule
/// applicability only and rejects any strength or count comparison.
fn checks_null(l: &KontraLesson, factor: i32) {
    assert!(!l.applies());
    for f in [1, 2, 4, 8] {
        assert_eq!(l.check(Step::Orient, Answer::Factor(f)), f == factor);
    }
    assert!(l.check(Step::Threshold, Answer::Applies(false)));
    assert!(!l.check(Step::Threshold, Answer::Applies(true)));
    for w in [true, false] {
        assert!(!l.check(Step::Threshold, Answer::Within(w)));
    }
    assert!(!l.check(Step::Orient, Answer::Applies(false)));
}

const GRAND: GameType = GameType::Grand;
const NULL: GameType = GameType::Null;
/// VH with all four Buben: CJ SJ HJ DJ + SA ST S9 DA D7 D8.
const VH_FOUR: &str = "CJ SJ HJ DJ SA ST S9 DA D7 D8";
/// VH without a Bube and a single Ace: CA C7 C8 S7 S8 S9 H7 H8 H9 D7.
const VH_ZERO: &str = "CA C7 C8 S7 S8 S9 H7 H8 H9 D7";
const HH_GRAND: &str = "HA HT HK HQ CT C9 SK SQ DK DQ";

#[test]
fn grand_kontra_with_four_and_zero_buben_declines_with_easys_exact_card() {
    // Four Buben = all Grand Trumpf, 4 < 5. Leading as a defender, Easy leads
    // the side master with most Augen, the last on a tie in core legal order:
    // SA and DA (11 Augen each) → DA. Zero Buben: the only side master is CA.
    for (vh, count, expected) in [(VH_FOUR, 4, "DA"), (VH_ZERO, 0, "CA")] {
        let g = hand(GRAND, deal_vh(vh, HH_GRAND, "D9 DT"));
        let view = current(&g);
        assert_eq!(view.seat, Seat::Vorhand);
        assert!(view.legal_actions.contains(&Action::Kontra));
        all_free(&g, &view);
        let l = kontra_lesson(&view).expect("Grand Kontra opportunity");
        assert_eq!(kontra_exclusion(&view), None);
        let r = &l.rule;
        assert_eq!(
            (r.call, r.game, r.level),
            (KontraCall::Kontra, GRAND, KontraLevel::None)
        );
        assert_eq!((r.count, r.threshold, r.wants), (count, 5, false));
        assert!(l.applies() && !r.null && !r.ouvert && !r.fallback && r.skat.is_empty());
        let rule = r.card.as_ref().expect("declining card rule");
        assert_eq!(
            (rule.card, rule.branch),
            (card(expected), Branch::LeadMaster)
        );
        assert_eq!(r.action, Action::Play(card(expected)));
        // The same card as Easy's legal card path with Kontra unavailable.
        let mut plain = view.clone();
        plain.legal_actions.retain(|a| *a != Action::Kontra);
        assert_eq!(skat_analysis::baseline::play(&plain), r.action);
        assert_eq!(skat_analysis::baseline::play(&view), r.action);
        assert_eq!(l.factor(), 2);
        checks(&l, 2, false);
        assert!(!l.check(Step::Threshold, Answer::Applies(false)));
        assert_eq!(r.legal, view.legal_actions);
    }
}

#[test]
fn null_kontra_never_applies_easys_rule_yet_kontra_stays_free() {
    // Null has no Trumpf. Leading, Easy plays its lowest card: Augen first,
    // then the fewest cards beaten in the suit → C7 (S9, H9 beat two).
    let vh = "C7 CA SA ST S9 H9 DA CJ SJ HJ";
    let g = hand(NULL, deal_vh(vh, HH_GRAND, "D9 DT"));
    let view = current(&g);
    assert!(view.legal_actions.contains(&Action::Kontra));
    all_free(&g, &view);
    let l = kontra_lesson(&view).expect("Null Kontra opportunity");
    let r = &l.rule;
    assert_eq!((r.call, r.game), (KontraCall::Kontra, NULL));
    assert!(r.null && !r.wants && !r.fallback);
    let rule = r.card.as_ref().expect("declining card rule");
    assert_eq!((rule.card, rule.branch), (card("C7"), Branch::Null));
    assert_eq!(r.action, Action::Play(card("C7")));
    checks_null(&l, 2);
    // Kontra itself is accepted by the core and observed as the reply.
    let mut taken = g.clone();
    taken.apply(Action::Kontra).unwrap();
    assert_eq!(taken.kontra(), KontraLevel::Kontra);
    assert_eq!(l.awaits(Action::Kontra), Some(KontraAwait::Reply));
}

/// HH declarer of `game`, `after` (Hand or pickup); VH says Kontra; HH answers.
fn re_in(game: GameType, deal: Deal, after: &[Action]) -> (Game, KontraLesson) {
    let mut g = play_phase(deal, ON, after, Announcement::plain(game));
    g.apply(Action::Kontra).unwrap();
    let view = current(&g);
    assert_eq!(view.seat, Seat::Hinterhand);
    assert_eq!(view.legal_actions, vec![Action::Re, Action::KeinRe]);
    all_free(&g, &view);
    let l = kontra_lesson(&view).expect("immediate Re reply");
    assert_eq!(
        (l.rule.call, l.rule.level, l.rule.game),
        (KontraCall::Re, KontraLevel::Kontra, game)
    );
    assert_eq!(l.factor(), 4);
    assert!(!l.rule.fallback && l.rule.card.is_none());
    (g, l)
}

/// Grand Hand cards, counted by hand: CJ 18 + DJ 14 = 32; Kreuz {CA} 10;
/// Pik {SA ST S7} 10 + 6; Herz {HA HT} 10 + 6; Karo {DA DT} 10 + 6 → 90.
const GRAND_90: &str = "CJ DJ CA SA ST S7 HA HT DA DT";
/// SJ instead of HA: Buben 46; Kreuz 10; Pik 16; Herz {HT} alone 1;
/// Karo 16 → 89.
const GRAND_89: &str = "CJ SJ DJ CA SA ST S7 HT DA DT";

#[test]
fn grand_hand_re_straddles_the_strength_threshold() {
    let (_, l) = re_in(GRAND, deal_hh_vh(GRAND_90, "D9 D8"), &[Action::Hand]);
    let r = &l.rule;
    assert!(r.hand_game && r.skat.is_empty() && l.applies());
    assert_eq!(
        (r.count, r.threshold, r.wants, r.action),
        (90, 90, true, Action::Re)
    );
    checks(&l, 4, true);
    let (_, l) = re_in(GRAND, deal_hh_vh(GRAND_89, "D9 D8"), &[Action::Hand]);
    let r = &l.rule;
    assert_eq!((r.count, r.wants, r.action), (89, false, Action::KeinRe));
    checks(&l, 4, false);
    // Unknown Hand Skat and unseen hands: a Skat holding HA or the missing
    // Bube would raise the count if it were counted; it is not.
    let base = deal_hh_vh(GRAND_89, "D9 D8");
    let (_, expected) = re_in(GRAND, base, &[Action::Hand]);
    for (a, b) in [(card("D9"), card("HA")), (card("D8"), card("HJ"))] {
        let (_, l) = re_in(GRAND, swap(base, a, b), &[Action::Hand]);
        assert_eq!(l, expected, "{a:?} {b:?}");
    }
}

#[test]
fn grand_pickup_known_discard_decides_re() {
    // Held after the discard: CJ 18 + SJ 14; Kreuz {CA CT} 16; Herz
    // {HA HT H9} 16; Karo {DA DQ} 10; Pik {S8} 0 → 74, below 90 alone.
    // Discard SA ST: Pik {S8 SA ST} 10 + 6 → 90 → Re.
    let held = cards("CJ SJ CA CT HA HT H9 DA DQ S8");
    let (g, l) = re_in(
        GRAND,
        deal_hh_vh("CJ SJ CA CT HA HT DA DQ SA ST", "H9 S8"),
        &[Action::TakeSkat, Action::Discard(cards("SA ST"))],
    );
    assert_eq!(current(&g).hand, held);
    let r = &l.rule;
    assert!(!r.hand_game);
    assert_eq!(r.skat, cards("SA ST"));
    assert_eq!((r.count, r.wants, r.action), (90, true, Action::Re));
    checks(&l, 4, true);
    // Discard HJ ST instead: HJ 14; Pik {S8 ST} Ten alone with one card 1 →
    // 89 → KeinRe.
    let (g, l) = re_in(
        GRAND,
        deal_hh_vh("CJ SJ CA CT HA HT DA DQ HJ ST", "H9 S8"),
        &[Action::TakeSkat, Action::Discard(cards("HJ ST"))],
    );
    assert_eq!(current(&g).hand, held);
    assert_eq!(
        (l.rule.skat, l.rule.count, l.rule.wants),
        (cards("HJ ST"), 89, false)
    );
    assert_eq!(l.rule.action, Action::KeinRe);
    // Unseen defender cards do not change the lesson.
    let base = deal_hh_vh("CJ SJ CA CT HA HT DA DQ HJ ST", "H9 S8");
    let rest: Vec<Card> = base.hand(Seat::Vorhand).iter().take(2).collect();
    let mh = base.hand(Seat::Mittelhand).iter().next().unwrap();
    let (_, swapped) = re_in(
        GRAND,
        swap(base, rest[0], mh),
        &[Action::TakeSkat, Action::Discard(cards("HJ ST"))],
    );
    assert_eq!(swapped, l);
}

#[test]
fn null_re_never_applies_easys_rule_whatever_the_cards() {
    // The same 90-strength Grand cards as Null declarer: Null has no Trumpf
    // and no strength score, so Easy answers KeinRe; Re stays free.
    for (after, hh, skat) in [
        (vec![Action::Hand], GRAND_90, "D9 D8"),
        (
            vec![Action::TakeSkat, Action::Discard(cards("SA ST"))],
            "CJ SJ CA CT HA HT DA DQ SA ST",
            "H9 S8",
        ),
    ] {
        let (g, l) = re_in(NULL, deal_hh_vh(hh, skat), &after);
        let r = &l.rule;
        assert!(r.null && !r.wants);
        assert_eq!(r.action, Action::KeinRe);
        checks_null(&l, 4);
        let mut taken = g.clone();
        taken.apply(Action::Re).unwrap();
        assert_eq!(taken.kontra(), KontraLevel::Re);
        assert_eq!(l.awaits(Action::Re), Some(KontraAwait::Resumed));
        assert_eq!(l.awaits(Action::KeinRe), Some(KontraAwait::Resumed));
    }
}

#[test]
fn restricted_or_inconsistent_views_fall_back_honestly() {
    let g = hand(HERZ, deal_vh(&vh("H9"), HH, "D8 D9"));
    let view = current(&g);
    // Restricted legal lists (synthetic, labelled): Kontra without cards and
    // Easy declining, or Re without KeinRe and Easy declining.
    let mut v = PlayerView {
        legal_actions: vec![Action::Kontra],
        ..view.clone()
    };
    v.hand = cards("CJ HA H7 S8 SA ST S9 DA D7 D8");
    assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
    let mut g2 = play_phase(
        deal_hh_vh(RE_89, "D9 DT"),
        ON,
        &[Action::Hand],
        Announcement::plain(HERZ),
    );
    g2.apply(Action::Kontra).unwrap();
    let v = PlayerView {
        legal_actions: vec![Action::Re],
        ..current(&g2)
    };
    assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
    // Absent contract, or a declarer offered Kontra.
    let v = PlayerView {
        contract: None,
        ..view.clone()
    };
    assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
    let v = PlayerView {
        declarer: Some(view.seat),
        ..view.clone()
    };
    assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
    // Grand and Null: the same restricted lists and inconsistent roles.
    for game in [GameType::Grand, GameType::Null] {
        let view = current(&hand(game, deal_vh(&vh("H9"), HH, "D8 D9")));
        let v = PlayerView {
            legal_actions: vec![Action::Kontra],
            ..view.clone()
        };
        assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
        let v = PlayerView {
            declarer: Some(view.seat),
            ..view.clone()
        };
        assert_eq!(kontra_exclusion(&v), Some(KontraExclusion::Restricted));
    }
    // No turn: no legal actions, no lesson and no exclusion.
    let waiting = g.view(Seat::Mittelhand);
    assert!(kontra_lesson(&waiting).is_none() && kontra_exclusion(&waiting).is_none());
}

#[test]
fn observations_follow_the_actual_next_public_transition() {
    // Kontra by VH: the declarer's actual reply.
    let base = hand(HERZ, deal_vh(&vh("H9"), HH, "D8 D9"));
    let l = kontra_lesson(&current(&base)).unwrap();
    assert_eq!(l.awaits(Action::Kontra), Some(KontraAwait::Reply));
    let mut g = base.clone();
    let events = g.apply(Action::Kontra).unwrap();
    assert_eq!(
        observe_kontra(KontraAwait::Reply, Seat::Vorhand, &events, g.kontra(), 0),
        None
    );
    let events = g.apply(Action::KeinRe).unwrap();
    assert_eq!(
        observe_kontra(KontraAwait::Reply, Seat::Vorhand, &events, g.kontra(), 0),
        Some(KontraObservation::Reply {
            seat: Seat::Hinterhand,
            re: false,
            level: KontraLevel::Kontra
        })
    );
    // Re by HH: the actual resumed card play, factor ×4.
    let events = {
        let mut g = base.clone();
        g.apply(Action::Kontra).unwrap();
        g.apply(Action::Re).unwrap();
        let e = g.apply(Action::Play(card("DA"))).unwrap();
        (e, g.kontra())
    };
    assert_eq!(
        observe_kontra(
            KontraAwait::Resumed,
            Seat::Hinterhand,
            &events.0,
            events.1,
            0
        ),
        Some(KontraObservation::Resumed {
            seat: Seat::Vorhand,
            card: card("DA"),
            level: KontraLevel::Re
        })
    );
    // Declining with a card: that card, not a hypothetical Kontra reply.
    let mut g = base;
    let events = g.apply(Action::Play(card("D7"))).unwrap();
    assert!(matches!(events[0], Event::CardPlayed { .. }));
    assert_eq!(
        l.awaits(Action::Play(card("D7"))),
        Some(KontraAwait::Declined)
    );
    assert_eq!(
        observe_kontra(KontraAwait::Declined, Seat::Vorhand, &events, g.kontra(), 0),
        Some(KontraObservation::Declined {
            seat: Seat::Vorhand,
            card: card("D7"),
            trick: 0,
            winner: None,
            level: KontraLevel::None
        })
    );
    assert_eq!(l.awaits(Action::Re), None);
}

#[test]
fn ouvert_round_uses_easys_actual_unconditional_choice_without_hidden_cards() {
    for game in [HERZ, GameType::Grand] {
        for fifth in ["H9", "S8"] {
            let deal = deal_vh(&vh(fifth), HH, "D8 D9");
            let ann = Announcement {
                schneider: true,
                schwarz: true,
                ouvert: true,
                ..Announcement::plain(game)
            };
            let g = play_phase(deal, ON, &[Action::Hand], ann);
            let view = current(&g);
            assert_eq!(view.declarer_cards, None);
            all_free(&g, &view);
            let lesson = kontra_lesson(&view).unwrap();
            assert_eq!(lesson.rule.action, Action::KeinKontra);
            assert_eq!(lesson.rule.action, skat_analysis::baseline::play(&view));
            assert!(!lesson.applies());
            assert!(lesson.check(Step::Threshold, Answer::Applies(false)));
            let mh = deal.hand(Seat::Mittelhand).iter().next().unwrap();
            let other = play_phase(swap(deal, card("CA"), mh), ON, &[Action::Hand], ann);
            assert_eq!(kontra_lesson(&current(&other)), Some(lesson));
        }
    }
}
