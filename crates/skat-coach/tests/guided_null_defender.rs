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
fn defender_scope_rejects_missing_role_and_partner_lead() {
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
    for change in 0..5 {
        let mut v = view.clone();
        match change {
            0 => v.declarer = None,
            1 => v.declarer = Some(Seat::Hinterhand),
            2 => v.legal_actions.reverse(),
            3 => v.contract.as_mut().unwrap().ouvert = true,
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
