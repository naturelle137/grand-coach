// SPDX-License-Identifier: EUPL-1.2

//! Core-replayed non-Ouvert Grand `WinLast` (#157) and `UnderPartner` (#159)
//! fixtures. Expected winners, candidates and Easy cards are derived by hand
//! from the Grand card order (Buben are the only Trumpf; side suits
//! A T K Q 9 8 7) and Easy's documented first minimum of `(is_trump, power)`
//! for WinLast or `(is_trump, Augen, power)` for UnderPartner, never by calling
//! the policy under test.

#[path = "support/guided.rs"]
mod support;
use skat_analysis::baseline::{TieBreak, rule};
use skat_coach::{Source, guided::*};
use skat_core::{Action, Event, Follow, Game, GameType, RuleConfig, Seat, Suit};
use support::*;

const GRAND: GameType = GameType::Grand;

fn grand(deal: skat_core::Deal, declarer: Seat, plays: &[&str]) -> Game {
    contract_position(deal, declarer, hand_game(GRAND, false), plays)
}

fn plays(cards: &[&str]) -> Vec<Action> {
    cards.iter().map(|c| Action::Play(card(c))).collect()
}

fn trick_won(events: &[Event]) -> Option<skat_core::CompletedTrick> {
    events.iter().find_map(|e| match e {
        Event::TrickWon { trick } => Some(*trick),
        _ => None,
    })
}

/// Plays `played` for HH (last seat) and returns the coach observation built
/// from the actual core `TrickWon`, as skat-app does.
fn observe_last(g: &Game, led: &str, played: &str) -> Observation {
    let mut g = g.clone();
    let trick = trick_won(&g.apply(Action::Play(card(played))).unwrap()).expect("trick complete");
    assert_eq!(trick.cards[2], card(played));
    observe(
        GRAND,
        Source::Trick {
            index: 0,
            seat: Seat::Hinterhand,
            led: card(led),
            card: card(played),
        },
        Some(trick.winner),
    )
    .unwrap()
}

/// Shared lesson facts in the last seat: public winner, nobody follows.
fn assert_last_lesson(g: &Game, ahead: Seat, led: &str, best: &str) -> Lesson {
    let view = g.view(Seat::Hinterhand);
    assert_eq!(g.current_seat(), Some(Seat::Hinterhand));
    let l = lesson(&view).expect("Grand WinLast lesson");
    assert_eq!(l.game, GRAND);
    assert_eq!((l.seat, l.remaining), (Seat::Hinterhand, None));
    assert_eq!(
        l.source,
        Source::Trick {
            index: 0,
            seat: ahead,
            led: card(led),
            card: card(best),
        }
    );
    assert_eq!(l.rule.branch, Branch::WinLast);
    assert_eq!(l.rule.tie_break, TieBreak::LowestWinnerFirst);
    assert!(l.rule.last && !l.rule.partner_ahead);
    assert_eq!(l.rule.winner, Some(ahead));
    assert!(l.check(Step::Orient, Answer::Seat(Some(ahead))));
    for wrong in Seat::ALL.into_iter().filter(|&s| s != ahead) {
        assert!(!l.check(Step::Orient, Answer::Seat(Some(wrong))));
    }
    assert!(l.check(Step::Check, Answer::Seat(None)));
    assert!(!l.check(Step::Check, Answer::Seat(Some(Seat::Vorhand))));
    for c in &l.rule.candidates {
        assert!(l.check(Step::Narrow, Answer::Card(*c)));
    }
    for c in &l.rule.losers {
        assert!(!l.check(Step::Narrow, Answer::Card(*c)));
    }
    assert_eq!(Some(l.clone()), lesson(&view), "deterministic");
    l
}

#[test]
fn declarer_grand_hand_cheaper_kreuz_winner_and_changed_lead() {
    // HH: CA CQ SA HA D7 D8 D9 DT DJ DQ declares Grand Hand (mit 1 → 48 ≥ 18).
    let g = grand(deal(), Seat::Hinterhand, &["C9", "C8"]);
    let view = g.view(Seat::Hinterhand);
    // DJ is Trumpf in Grand and does not follow Kreuz.
    assert_eq!(view.legal_actions, plays(&["CQ", "CA"]));
    let l = assert_last_lesson(&g, Seat::Vorhand, "C9", "C9");
    // Both overtake C9; CQ ranks below CA among nontrump winners.
    assert_eq!(l.rule.winners, vec![card("CQ"), card("CA")]);
    assert!(l.rule.losers.is_empty());
    assert_eq!(l.rule.candidates, vec![card("CQ"), card("CA")]);
    assert_eq!(l.rule.card, card("CQ"));
    assert_eq!(l.rule.tie_key.0, 0);
    for played in ["CQ", "CA"] {
        let o = observe_last(&g, "C9", played);
        assert_eq!(
            (o.follow, o.proves_void),
            (Follow::Farbe(Suit::Kreuz), false)
        );
        assert_eq!(o.winner, Some(Seat::Hinterhand));
    }

    // CK led: only CA overtakes; CQ stays a legal losing alternative.
    let g = grand(deal(), Seat::Hinterhand, &["CK", "C8"]);
    assert_eq!(g.view(Seat::Hinterhand).legal_actions, plays(&["CQ", "CA"]));
    let l = assert_last_lesson(&g, Seat::Vorhand, "CK", "CK");
    assert_eq!(l.rule.winners, vec![card("CA")]);
    assert_eq!(l.rule.losers, vec![card("CQ")]);
    assert_eq!(l.rule.candidates, vec![card("CA")]);
    assert_eq!(l.rule.card, card("CA"));
    assert_eq!(observe_last(&g, "CK", "CA").winner, Some(Seat::Hinterhand));
    let losing = observe_last(&g, "CK", "CQ");
    assert_eq!(losing.winner, Some(Seat::Vorhand));
    assert!(!losing.proves_void);
}

#[test]
fn hidden_opponent_card_and_unknown_skat_swap_keep_the_grand_dialogue() {
    // VH's unplayed H7 swaps with the unknown Hand Skat's DK.
    let a = grand(deal(), Seat::Hinterhand, &["C9", "C8"]);
    let b = grand(
        swap(deal(), card("H7"), card("DK")),
        Seat::Hinterhand,
        &["C9", "C8"],
    );
    let (va, vb) = (a.view(Seat::Hinterhand), b.view(Seat::Hinterhand));
    assert_eq!(va, vb);
    let l = lesson(&va).unwrap();
    assert_eq!(Some(l.clone()), lesson(&vb));
    let m = l.memory();
    assert_eq!(m.skat, None);
    assert!(m.unseen.contains(card("H7")) && m.unseen.contains(card("DK")));
    // Grand Trumpf outside HH's own DJ: CJ, SJ and HJ, none played.
    assert_eq!(m.trumpf.unwrap().outside, 3);
    for played in ["CQ", "CA"] {
        assert_eq!(
            observe_last(&a, "C9", played),
            observe_last(&b, "C9", played)
        );
    }
}

#[test]
fn defender_last_with_opposing_declarer_ahead() {
    // VH: CK CJ C9 H7 H8 H9 HT HJ HQ HK declares Grand Hand (mit 1 → 48).
    // MH (partner) plays C7 below the declarer's C9; HH defends last.
    let g = grand(deal(), Seat::Vorhand, &["C9", "C7"]);
    assert_eq!(g.view(Seat::Hinterhand).declarer, Some(Seat::Vorhand));
    assert_eq!(g.view(Seat::Hinterhand).legal_actions, plays(&["CQ", "CA"]));
    let l = assert_last_lesson(&g, Seat::Vorhand, "C9", "C9");
    assert_eq!(l.rule.candidates, vec![card("CQ"), card("CA")]);
    assert_eq!(l.rule.card, card("CQ"));
    for played in ["CQ", "CA"] {
        assert_eq!(
            observe_last(&g, "C9", played).winner,
            Some(Seat::Hinterhand)
        );
    }

    let g = grand(deal(), Seat::Vorhand, &["CK", "C7"]);
    let l = assert_last_lesson(&g, Seat::Vorhand, "CK", "CK");
    assert_eq!(l.rule.candidates, vec![card("CA")]);
    assert_eq!(l.rule.losers, vec![card("CQ")]);
    assert_eq!(l.rule.card, card("CA"));
    assert_eq!(observe_last(&g, "CK", "CQ").winner, Some(Seat::Vorhand));
}

#[test]
fn grand_bube_follows_trumpf_never_its_printed_suit() {
    // HH also holds SJ (MH gets D7). VH leads HJ: HH must follow Trumpf with
    // SJ or DJ, not Pik or Karo; SJ overtakes HJ, DJ does not.
    let d = swap(deal(), card("SJ"), card("D7"));
    let g = grand(d, Seat::Hinterhand, &["HJ", "C7"]);
    assert_eq!(g.view(Seat::Hinterhand).legal_actions, plays(&["SJ", "DJ"]));
    let l = assert_last_lesson(&g, Seat::Vorhand, "HJ", "HJ");
    assert_eq!(l.rule.winners, vec![card("SJ")]);
    assert_eq!(l.rule.losers, vec![card("DJ")]);
    assert_eq!(l.rule.card, card("SJ"));
    let o = observe_last(&g, "HJ", "DJ");
    assert_eq!((o.follow, o.proves_void), (Follow::Trumpf, false));
    assert_eq!(o.winner, Some(Seat::Vorhand));
    assert_eq!(observe_last(&g, "HJ", "SJ").winner, Some(Seat::Hinterhand));

    // HH holds CJ: a Kreuz lead is not followed by the Kreuz-Bube.
    let d = swap(deal(), card("CJ"), card("D7"));
    let g = grand(d, Seat::Hinterhand, &["C9", "C8"]);
    assert_eq!(g.view(Seat::Hinterhand).legal_actions, plays(&["CQ", "CA"]));
    assert_eq!(
        lesson(&g.view(Seat::Hinterhand)).unwrap().rule.card,
        card("CQ")
    );

    // Without plain Kreuz, every card is legal; only CJ and DJ overtake and
    // Easy prefers the lower Trumpf DJ. Playing CJ proves a Kreuz void.
    let d = swap(swap(d, card("CA"), card("H7")), card("CQ"), card("H8"));
    let g = grand(d, Seat::Hinterhand, &["C9", "C8"]);
    let legal = g.view(Seat::Hinterhand).legal_actions;
    assert_eq!(legal.len(), 10);
    let l = assert_last_lesson(&g, Seat::Vorhand, "C9", "C9");
    let mut winners = l.rule.winners.clone();
    winners.sort_by_key(|c| c.suit().index());
    assert_eq!(winners, vec![card("CJ"), card("DJ")]);
    assert_eq!(l.rule.card, card("DJ"));
    assert_eq!(l.rule.tie_key.0, 1);
    let o = observe_last(&g, "C9", "CJ");
    assert_eq!(
        (o.follow, o.proves_void),
        (Follow::Farbe(Suit::Kreuz), true)
    );
    assert_eq!(o.winner, Some(Seat::Hinterhand));
    let o = observe_last(&g, "C9", "H7");
    assert!(o.proves_void);
    assert_eq!(o.winner, Some(Seat::Vorhand));
    for a in legal {
        g.clone().apply(a).unwrap(); // Every legal alternative stays playable.
    }
}

#[test]
fn grand_exclusions_partner_ahead_not_last_lead_no_winner_and_single_card() {
    // Partner ahead: MH's CT overtakes the declarer's C9. HH last selects the
    // earlier partner branch (FeedPartner), supported in last seat.
    let g = grand(deal(), Seat::Vorhand, &["C9", "CT"]);
    let v = g.view(Seat::Hinterhand);
    let r = rule(&v).unwrap();
    assert!(r.partner_ahead && r.last && r.winners.contains(&card("CA")));
    assert_eq!(r.branch, Branch::FeedPartner);
    assert_eq!(lesson(&v).is_some(), r.last);

    // Leading declarer: DrawTrumpf, now an own-lead lesson (#175).
    let g = grand(deal(), Seat::Vorhand, &[]);
    let v = g.view(Seat::Vorhand);
    assert_eq!(rule(&v).unwrap().branch, Branch::DrawTrumpf);
    assert_eq!(lesson(&v).unwrap().rule.branch, Branch::DrawTrumpf);

    // Not last: MH's only winner CT is no master (CA unseen) and no Trumpf.
    let g = grand(deal(), Seat::Vorhand, &["C9"]);
    let v = g.view(Seat::Mittelhand);
    let r = rule(&v).unwrap();
    assert!(!r.last);
    assert_eq!(r.branch, Branch::Fallback);
    assert!(lesson(&v).is_none());

    // No winner: HH holds H8 (VH gets D7); MH trumps VH's H7 with SJ and
    // HH's Herz cannot overtake.
    let d = swap(deal(), card("D7"), card("H8"));
    let g = grand(d, Seat::Hinterhand, &["H7", "SJ"]);
    let v = g.view(Seat::Hinterhand);
    assert_eq!(v.legal_actions, plays(&["H8", "HA"]));
    let r = rule(&v).unwrap();
    assert!(r.winners.is_empty() && r.last);
    assert_eq!(r.branch, Branch::Fallback);
    assert!(lesson(&v).is_none());

    // Single legal card: CQ is HH's only Kreuz and still wins.
    let d = swap(deal(), card("CA"), card("H7"));
    let g = grand(d, Seat::Hinterhand, &["C9", "C8"]);
    let v = g.view(Seat::Hinterhand);
    assert_eq!(v.legal_actions, plays(&["CQ"]));
    assert_eq!(rule(&v).unwrap().branch, Branch::WinLast);
    assert!(lesson(&v).is_none());
}

#[test]
fn null_and_ouvert_grand_stay_unsupported_while_farbspiel_ouvert_is_unchanged() {
    for (game, ouvert, supported) in [
        (GameType::Null, false, false),
        (GRAND, true, false),
        (GameType::Farbe(Suit::Herz), true, true),
        (GameType::Farbe(Suit::Herz), false, true),
    ] {
        let g = contract_position(
            deal(),
            Seat::Hinterhand,
            hand_game(game, ouvert),
            &["C9", "C8"],
        );
        let v = g.view(Seat::Hinterhand);
        assert_eq!(v.contract.unwrap().ouvert, ouvert);
        assert_eq!(v.legal_actions, plays(&["CQ", "CA"]));
        let r = rule(&v).unwrap();
        let expected = if game == GameType::Null {
            Branch::Null
        } else {
            Branch::WinLast
        };
        assert_eq!(r.branch, expected);
        assert_eq!(lesson(&v).is_some(), supported, "{game:?} ouvert {ouvert}");
    }
}

#[test]
fn kontra_offer_keeps_precedence_over_the_grand_card_lesson() {
    // Defender position as above with Kontra/Re on; HH bids 18 before
    // passing, so it is Kontra-eligible (D-26.1) at its first card turn and
    // the Kontra dialogue (not WinLast) applies.
    let mut g = Game::from_deal(deal(), RuleConfig { kontra_re: true });
    for a in [
        Action::Pass,
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Hand,
    ]
    .into_iter()
    .chain([Action::Announce(hand_game(GRAND, false))])
    .chain(plays(&["C9", "C7"]))
    {
        g.apply(a).unwrap();
    }
    let v = g.view(Seat::Hinterhand);
    assert_eq!(v.declarer, Some(Seat::Vorhand));
    assert!(v.legal_actions.contains(&Action::Kontra));
    assert!(rule(&v).is_none() && lesson(&v).is_none());
    assert!(kontra_lesson(&v).is_some());
}

// ---- Grand UnderPartner (#159): MH defends between the partner and HH. ----

/// Plays MH's `mine` and HH's `reply` after VH's `led`; returns the coach
/// observation of the declarer's actual card and the actual completed winner.
fn observe_reply(g: &Game, led: &str, mine: &str, reply: &str) -> Observation {
    let mut g = g.clone();
    g.apply(Action::Play(card(mine))).unwrap();
    assert_eq!(g.current_seat(), Some(Seat::Hinterhand), "declarer next");
    let trick = trick_won(&g.apply(Action::Play(card(reply))).unwrap()).expect("trick complete");
    assert_eq!(trick.cards, [card(led), card(mine), card(reply)]);
    observe(
        GRAND,
        Source::Trick {
            index: 0,
            seat: Seat::Hinterhand,
            led: card(led),
            card: card(reply),
        },
        Some(trick.winner),
    )
    .unwrap()
}

/// Shared UnderPartner facts: VH's `led` ahead, HH (declarer) still to act.
fn assert_under_lesson(g: &Game, led: &str, losers: &[&str], winners: &[&str]) -> Lesson {
    let view = g.view(Seat::Mittelhand);
    assert_eq!(g.current_seat(), Some(Seat::Mittelhand));
    assert_eq!(view.declarer, Some(Seat::Hinterhand));
    let l = lesson(&view).expect("Grand UnderPartner lesson");
    assert_eq!(l.game, GRAND);
    assert_eq!(
        (l.seat, l.remaining),
        (Seat::Mittelhand, Some(Seat::Hinterhand))
    );
    assert_eq!(
        l.source,
        Source::Trick {
            index: 0,
            seat: Seat::Vorhand,
            led: card(led),
            card: card(led),
        }
    );
    let r = &l.rule;
    assert_eq!(r.branch, Branch::UnderPartner);
    assert_eq!(r.tie_break, TieBreak::LowestCostFirst);
    assert!(r.partner_ahead && !r.last && !r.table_master);
    assert_eq!(r.winner, Some(Seat::Vorhand));
    assert_eq!(r.losers, losers.iter().map(|c| card(c)).collect::<Vec<_>>());
    assert_eq!(
        r.winners,
        winners.iter().map(|c| card(c)).collect::<Vec<_>>()
    );
    assert_eq!(r.candidates, r.losers);
    assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
    for wrong in [Seat::Mittelhand, Seat::Hinterhand] {
        assert!(!l.check(Step::Orient, Answer::Seat(Some(wrong))));
    }
    assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
    assert!(!l.check(Step::Check, Answer::Seat(None)));
    assert!(!l.check(Step::Check, Answer::Seat(Some(Seat::Vorhand))));
    for c in losers {
        assert!(l.check(Step::Narrow, Answer::Card(card(c))));
    }
    for c in winners {
        assert!(!l.check(Step::Narrow, Answer::Card(card(c))));
    }
    assert_eq!(Some(l.clone()), lesson(&view), "deterministic");
    l
}

#[test]
fn defender_under_partner_c7_c8_lose_ct_overtakes_and_actual_declarer_reply() {
    // HH: CA CQ SA HA D7 D8 D9 DT DJ DQ declares Grand Hand; VH leads CK.
    // MH: C7 C8 CT; CK is no master for MH (CA unseen), C7/C8 stay below,
    // CT overtakes. Easy: lowest (Trumpf, Augen, power) loser → C7 (0 Augen,
    // below C8). C8 is a correct non-Easy candidate.
    let g = grand(deal(), Seat::Hinterhand, &["CK"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["C7", "C8", "CT"]));
    let l = assert_under_lesson(&g, "CK", &["C7", "C8"], &["CT"]);
    assert_eq!(l.rule.card, card("C7"));
    assert_eq!((l.rule.tie_key.0, l.rule.tie_key.1), (0, 0));
    // Every legal MH card, then each actual HH Kreuz: CA wins; CQ leaves
    // the card ahead (VH's CK, or MH's CT) in front.
    for (mine, reply, winner) in [
        ("C7", "CA", Seat::Hinterhand),
        ("C7", "CQ", Seat::Vorhand),
        ("C8", "CA", Seat::Hinterhand),
        ("C8", "CQ", Seat::Vorhand),
        ("CT", "CA", Seat::Hinterhand),
        ("CT", "CQ", Seat::Mittelhand),
    ] {
        let mut after = g.clone();
        after.apply(Action::Play(card(mine))).unwrap();
        assert_eq!(after.legal_actions(), plays(&["CQ", "CA"]));
        let o = observe_reply(&g, "CK", mine, reply);
        assert_eq!(
            (o.follow, o.proves_void),
            (Follow::Farbe(Suit::Kreuz), false)
        );
        assert_eq!(o.winner, Some(winner), "{mine} then {reply}");
    }
}

#[test]
fn own_higher_kreuz_make_ck_a_follow_class_master_and_early_feed_partner_applies() {
    // CA swaps with MH's C7: MH holds CA C8 CT, so every higher Kreuz is
    // own and VH's CK is a follow-class master (not immunity to trumping).
    let g = grand(
        swap(deal(), card("CA"), card("C7")),
        Seat::Hinterhand,
        &["CK"],
    );
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["C8", "CT", "CA"]));
    let r = rule(&v).unwrap();
    assert!(r.partner_ahead && !r.last && r.table_master);
    assert_eq!(r.branch, Branch::FeedPartner);
    // #131 now admits this earlier #159 exclusion; preserve its complete
    // deal and baseline assertions, and check the newly supported lesson.
    let l = lesson(&v).unwrap();
    assert_eq!(l.rule, r);
    assert_eq!(l.remaining, Some(Seat::Hinterhand));
    assert_eq!(l.rule.candidates, vec![card("C8"), card("CT"), card("CA")]);
    assert_eq!(l.rule.card, card("CA"));
    assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
}

#[test]
fn grand_under_partner_exclusions_last_seat_no_loser_single_card_declarer_role() {
    // Partner ahead in the last seat: FeedPartner, not UnderPartner.
    let g = grand(deal(), Seat::Vorhand, &["C9", "CT"]);
    let v = g.view(Seat::Hinterhand);
    let r = rule(&v).unwrap();
    assert!(r.partner_ahead && r.last);
    assert_eq!(r.branch, Branch::FeedPartner);
    assert_eq!(lesson(&v).is_some(), r.last);

    // No losing candidate: MH holds only CT and CK in Kreuz (VH gets C7/C8
    // instead of CK/H7), both overtake VH's C9 → Fallback.
    let d = swap(swap(deal(), card("C7"), card("CK")), card("C8"), card("H7"));
    let g = grand(d, Seat::Hinterhand, &["C9"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["CT", "CK"]));
    let r = rule(&v).unwrap();
    assert!(r.partner_ahead && !r.last && r.losers.is_empty());
    assert_eq!(r.branch, Branch::Fallback);
    assert!(lesson(&v).is_none());

    // Single legal card: C7 is MH's only Kreuz (C8/CT swap with VH's H7/H8).
    let d = swap(swap(deal(), card("C8"), card("H7")), card("CT"), card("H8"));
    let g = grand(d, Seat::Hinterhand, &["CK"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.legal_actions, plays(&["C7"]));
    let r = rule(&v).unwrap();
    assert_eq!(
        (r.branch, r.losers.clone()),
        (Branch::UnderPartner, vec![card("C7")])
    );
    assert!(lesson(&v).is_none());

    // Declarer in the middle seat: VH's CK is an opponent's card; MH's only
    // winner CT is neither master nor Trumpf → Fallback, no partner branch.
    let g = grand(deal(), Seat::Mittelhand, &["CK"]);
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.declarer, Some(Seat::Mittelhand));
    let r = rule(&v).unwrap();
    assert!(!r.partner_ahead && !r.last);
    assert_eq!(r.branch, Branch::Fallback);
    assert!(lesson(&v).is_none());
}

#[test]
fn grand_under_partner_bube_follows_trumpf_and_declarer_answers_off_trumpf() {
    // HH's DJ swaps with MH's C7. VH leads HJ: MH must follow Trumpf with
    // SJ or DJ (never Pik/Karo); DJ leaves HJ ahead, SJ overtakes. HJ is no
    // master for MH (CJ unseen). Easy: DJ. HH, without Buben, answers D7.
    let d = swap(deal(), card("DJ"), card("C7"));
    let g = grand(d, Seat::Hinterhand, &["HJ"]);
    assert_eq!(g.view(Seat::Mittelhand).legal_actions, plays(&["SJ", "DJ"]));
    let l = assert_under_lesson(&g, "HJ", &["DJ"], &["SJ"]);
    assert_eq!(l.rule.card, card("DJ"));
    assert_eq!(l.rule.tie_key.0, 1);
    for (mine, winner) in [("DJ", Seat::Vorhand), ("SJ", Seat::Mittelhand)] {
        let mut after = g.clone();
        after.apply(Action::Play(card(mine))).unwrap();
        assert_eq!(after.legal_actions().len(), 10, "HH holds no Trumpf");
        let o = observe_reply(&g, "HJ", mine, "D7");
        assert_eq!((o.follow, o.proves_void), (Follow::Trumpf, true));
        assert_eq!(o.winner, Some(winner));
    }
}

#[test]
fn under_does_not_mean_partner_safe_declarer_void_trumps_or_discards() {
    // HH's CA/CQ swap with VH's H7/H8. MH's permitted view is unchanged
    // (CA is unseen either way), so the same C7 lesson applies.
    let base = grand(deal(), Seat::Hinterhand, &["CK"]);
    let d = swap(swap(deal(), card("CA"), card("H7")), card("CQ"), card("H8"));
    let g = grand(d, Seat::Hinterhand, &["CK"]);
    assert_eq!(g.view(Seat::Mittelhand), base.view(Seat::Mittelhand));
    let l = assert_under_lesson(&g, "CK", &["C7", "C8"], &["CT"]);
    assert_eq!(l.rule.card, card("C7"));
    // After C7: HH's DJ trumps and wins; H7 discards and CK stays ahead.
    // Both prove HH's Kreuz void at this play.
    for (reply, winner) in [("DJ", Seat::Hinterhand), ("H7", Seat::Vorhand)] {
        let o = observe_reply(&g, "CK", "C7", reply);
        assert_eq!(
            (o.follow, o.proves_void),
            (Follow::Farbe(Suit::Kreuz), true)
        );
        assert_eq!(o.winner, Some(winner));
    }
}

#[test]
fn hidden_declarer_card_and_unknown_hand_skat_swap_keep_the_under_dialogue() {
    // HH's unplayed D8 swaps with the unknown Hand Skat's DK.
    let a = grand(deal(), Seat::Hinterhand, &["CK"]);
    let b = grand(
        swap(deal(), card("D8"), card("DK")),
        Seat::Hinterhand,
        &["CK"],
    );
    let (va, vb) = (a.view(Seat::Mittelhand), b.view(Seat::Mittelhand));
    assert_eq!(va, vb);
    let l = lesson(&va).unwrap();
    assert_eq!(Some(l.clone()), lesson(&vb));
    assert_eq!(l.memory(), lesson(&vb).unwrap().memory());
    let m = l.memory();
    assert_eq!(m.skat, None);
    assert!(m.unseen.contains(card("D8")) && m.unseen.contains(card("DK")));
    // Grand Trumpf outside MH's own SJ: CJ, HJ and DJ, none played.
    assert_eq!(m.trumpf.unwrap().outside, 3);
    // Same actual public cards → same observation.
    for mine in ["C7", "C8", "CT"] {
        assert_eq!(
            observe_reply(&a, "CK", mine, "CA"),
            observe_reply(&b, "CK", mine, "CA")
        );
    }
}

#[test]
fn kontra_offer_keeps_precedence_over_the_grand_under_partner_lesson() {
    // MH bids 18 before passing and is Kontra-eligible at its first card.
    let mut g = Game::from_deal(deal(), RuleConfig { kontra_re: true });
    for a in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Bid(20),
        Action::Pass,
        Action::Hand,
        Action::Announce(hand_game(GRAND, false)),
        Action::Play(card("CK")),
    ] {
        g.apply(a).unwrap();
    }
    let v = g.view(Seat::Mittelhand);
    assert_eq!(v.declarer, Some(Seat::Hinterhand));
    assert!(v.legal_actions.contains(&Action::Kontra));
    assert!(rule(&v).is_none() && lesson(&v).is_none());
    assert!(kontra_lesson(&v).is_some());
}
