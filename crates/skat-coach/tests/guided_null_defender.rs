// SPDX-License-Identifier: EUPL-1.2

//! A defender follows the declarer's Kreuz-Dame in a legal Null Hand game.
#[path = "../../skat-analysis/tests/null_middle_policy.rs"]
mod fixture;
use fixture::{card, cases};
use skat_coach::guided::{Answer, Step, lesson};
use skat_core::{Action, Announcement, Game, GameType, RuleConfig, Seat};

#[test]
fn defender_follows_declarer_and_observes_reply() {
    let deal = cases().remove(0).deal;
    let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
        Action::Play(card(5)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Mittelhand);
    assert_eq!(view.declarer, Some(Seat::Vorhand));
    assert_eq!(
        view.legal_actions,
        [0, 3, 4, 6].map(|n| Action::Play(card(n)))
    );
    let l = lesson(&view).expect("defender lesson");
    assert_eq!(l.rule.candidates, [card(0), card(3), card(4)]);
    assert_eq!(l.rule.card, card(4));
    assert!(l.check(Step::Narrow, Answer::Card(card(4))));
    assert!(!l.check(Step::Narrow, Answer::Card(card(6))));
    game.apply(Action::Play(card(4))).unwrap();
    game.apply(Action::Play(card(2))).unwrap();
    assert_eq!(game.view(Seat::Mittelhand).tricks[0].winner, Seat::Vorhand);
}

#[test]
fn defender_scope_rejects_missing_role_and_invalid_views() {
    let deal = cases().remove(0).deal;
    let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
        Action::Play(card(5)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Mittelhand);
    for change in 0..4 {
        let mut v = view.clone();
        match change {
            0 => v.declarer = None,
            1 => v.legal_actions.reverse(),
            2 => v.contract.as_mut().unwrap().ouvert = true,
            _ => v.own_skat = Some(fixture::set(&[30, 31])),
        }
        assert!(lesson(&v).is_none(), "{change}");
    }
}

#[test]
fn defender_hidden_deals_do_not_change_lesson() {
    let cases = cases();
    let view = |id| {
        let deal = cases.iter().find(|c| c.id == id).unwrap().deal;
        let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
        for action in [
            Action::Bid(18),
            Action::Hold,
            Action::Pass,
            Action::Pass,
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
            Action::Play(card(5)),
        ] {
            game.apply(action).unwrap();
        }
        game.view(Seat::Mittelhand)
    };
    let original = view("mixed-hand");
    let l = lesson(&original).unwrap();
    for id in [
        "privacy-opponent-opponent",
        "privacy-opponent-unknown-Hand-Skat",
    ] {
        let other = view(id);
        assert_eq!(original, other);
        assert_eq!(Some(l.clone()), lesson(&other));
    }
}

#[test]
fn defender_after_partner_lead_uses_public_trick() {
    let mut game = Game::from_deal(cases().remove(0).deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Pass,
        Action::Bid(18),
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
        Action::Play(card(5)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Mittelhand);
    assert_eq!(view.declarer, Some(Seat::Hinterhand));
    let l = lesson(&view).expect("partner lead lesson");
    assert_eq!(l.rule.candidates, [card(0), card(3), card(4), card(6)]);
    assert_eq!(l.rule.card, card(0));
    assert!(l.check(Step::Orient, Answer::Seat(Some(Seat::Vorhand))));
    assert!(l.check(Step::Check, Answer::Seat(Some(Seat::Hinterhand))));
    assert!(l.check(Step::Narrow, Answer::Card(card(0))));
    assert!(!l.check(Step::Narrow, Answer::Card(card(4))));
    for id in [
        "privacy-opponent-opponent",
        "privacy-opponent-unknown-Hand-Skat",
    ] {
        let mut other = Game::from_deal(
            cases().into_iter().find(|c| c.id == id).unwrap().deal,
            RuleConfig { kontra_re: false },
        );
        for action in [
            Action::Pass,
            Action::Bid(18),
            Action::Pass,
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
            Action::Play(card(5)),
        ] {
            other.apply(action).unwrap();
        }
        assert_eq!(view, other.view(Seat::Mittelhand));
        assert_eq!(Some(l.clone()), lesson(&other.view(Seat::Mittelhand)));
    }
    game.apply(Action::Play(card(0))).unwrap();
    game.apply(Action::Play(card(7))).unwrap();
    assert_eq!(
        game.view(Seat::Mittelhand).tricks[0].winner,
        Seat::Hinterhand
    );
}

#[test]
fn partner_lead_with_no_losing_card_still_uses_fallback() {
    let original = cases().remove(0).deal;
    let mut vh = original.hand(Seat::Vorhand);
    let mut mh = original.hand(Seat::Mittelhand);
    assert!(vh.remove(card(10)) && mh.remove(card(0)));
    vh.insert(card(0));
    mh.insert(card(10));
    let deal =
        skat_core::Deal::new([vh, mh, original.hand(Seat::Hinterhand)], original.skat()).unwrap();
    let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Pass,
        Action::Bid(18),
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
        Action::Play(card(1)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Mittelhand);
    let l = lesson(&view).unwrap();
    assert!(l.rule.losers.is_empty());
    assert_eq!(l.rule, skat_analysis::baseline::rule(&view).unwrap());
    assert_eq!(l.rule.card, card(4));
}

#[test]
fn ouvert_defender_follows_revealed_declarer() {
    let mut game = Game::from_deal(cases().remove(0).deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement {
            ouvert: true,
            ..Announcement::plain(GameType::Null)
        }),
        Action::Play(card(5)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Mittelhand);
    let l = lesson(&view).expect("revealed defender lesson");
    assert_eq!(l.rule.candidates, [card(0), card(3), card(4)]);
    assert_eq!(l.rule.card, card(4));
    for change in 0..3 {
        let mut bad = view.clone();
        match change {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(bad.hand),
            _ => bad.legal_actions.reverse(),
        }
        assert!(lesson(&bad).is_none());
    }
}

#[test]
fn ouvert_defender_after_partner_lead_uses_only_permitted_view() {
    let make = |deal| {
        let mut game = Game::from_deal(deal, RuleConfig { kontra_re: false });
        for action in [
            Action::Pass,
            Action::Bid(18),
            Action::Pass,
            Action::Hand,
            Action::Announce(Announcement {
                ouvert: true,
                ..Announcement::plain(GameType::Null)
            }),
            Action::Play(card(5)),
        ] {
            game.apply(action).unwrap();
        }
        game
    };
    let game = make(cases().remove(0).deal);
    let view = game.view(Seat::Mittelhand);
    let l = lesson(&view).expect("revealed partner lead lesson");
    assert_eq!(l.rule.candidates, [card(0), card(3), card(4), card(6)]);
    assert_eq!(l.rule.card, card(0));
    for change in 0..3 {
        let mut bad = view.clone();
        match change {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(bad.hand),
            _ => bad.legal_actions.reverse(),
        }
        assert!(lesson(&bad).is_none());
    }
    let original = cases().remove(0).deal;
    let mut partner = original.hand(Seat::Vorhand);
    let mut skat = original.skat();
    let partner_card = card(10);
    let skat_card = skat.iter().next().unwrap();
    assert!(partner.remove(partner_card) && skat.remove(skat_card));
    partner.insert(skat_card);
    skat.insert(partner_card);
    let other = skat_core::Deal::new(
        [
            partner,
            original.hand(Seat::Mittelhand),
            original.hand(Seat::Hinterhand),
        ],
        skat,
    )
    .unwrap();
    let other_view = make(other).view(Seat::Mittelhand);
    assert_eq!(view, other_view);
    assert_eq!(Some(l), lesson(&other_view));
}

#[test]
fn last_defender_leaves_declarer_ahead() {
    let setup = || {
        let mut game = Game::from_deal(cases().remove(0).deal, RuleConfig { kontra_re: false });
        for action in [
            Action::Bid(18),
            Action::Hold,
            Action::Pass,
            Action::Pass,
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
            Action::Play(card(5)),
        ] {
            game.apply(action).unwrap();
        }
        game
    };
    let mut game = setup();
    game.apply(Action::Play(card(4))).unwrap();
    let view = game.view(Seat::Hinterhand);
    assert_eq!(view.legal_actions, [2, 7].map(|n| Action::Play(card(n))));
    let l = lesson(&view).expect("last defender lesson");
    assert_eq!(l.rule.candidates, [card(2)]);
    assert_eq!(l.rule.card, card(2));
    assert!(l.rule.last);
    assert!(l.check(Step::Check, Answer::Seat(None)));
    assert!(l.check(Step::Narrow, Answer::Card(card(2))));
    assert!(!l.check(Step::Narrow, Answer::Card(card(7))));
    game.apply(Action::Play(card(2))).unwrap();
    assert_eq!(game.view(Seat::Hinterhand).tricks[0].winner, Seat::Vorhand);

    let mut partner_ahead = setup();
    partner_ahead.apply(Action::Play(card(6))).unwrap();
    // Admitted separately: the partner-ahead Fallback (see the partner test file).
    let l = lesson(&partner_ahead.view(Seat::Hinterhand)).unwrap();
    assert!(l.rule.winner != partner_ahead.view(Seat::Hinterhand).declarer);
}

#[test]
fn ouvert_last_defender_requires_consistent_revealed_hand() {
    let mut game = Game::from_deal(cases().remove(0).deal, RuleConfig { kontra_re: false });
    for action in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement {
            ouvert: true,
            ..Announcement::plain(GameType::Null)
        }),
        Action::Play(card(5)),
        Action::Play(card(4)),
    ] {
        game.apply(action).unwrap();
    }
    let view = game.view(Seat::Hinterhand);
    let l = lesson(&view).expect("revealed last defender lesson");
    assert_eq!(l.rule.candidates, [card(2)]);
    assert_eq!(l.rule.card, card(2));
    assert!(l.rule.last);
    for change in 0..4 {
        let mut bad = view.clone();
        match change {
            0 => bad.declarer_cards = None,
            1 => bad.declarer_cards = Some(bad.hand),
            2 => bad.legal_actions.reverse(),
            _ => bad.current_seat = Some(Seat::Mittelhand),
        }
        assert!(lesson(&bad).is_none(), "{change}");
    }
    for action in view.legal_actions {
        let mut alternative = game.clone();
        alternative.apply(action).unwrap();
        assert_eq!(alternative.view(Seat::Hinterhand).tricks.len(), 1);
    }
}
