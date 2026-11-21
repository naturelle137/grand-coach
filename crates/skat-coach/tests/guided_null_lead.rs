// SPDX-License-Identifier: EUPL-1.2

//! Accepted176 complete lessons, admission separate from broader shared policy.
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/null_lead_policy.rs"]
mod fixture;
use fixture::{card, cases};
use skat_coach::{
    Source,
    guided::{Answer, Step, lesson},
};
use skat_core::*;

#[test]
fn ouvert_declarer_opening_uses_the_same_null_rule() {
    for c in cases() {
        let mut game = Game::from_deal(c.deal, RuleConfig { kontra_re: false });
        for action in c.history.iter().copied() {
            let action = match action {
                Action::Announce(mut announcement) => {
                    announcement.ouvert = true;
                    Action::Announce(announcement)
                }
                other => other,
            };
            game.apply(action).unwrap();
        }
        let view = game.view(Seat::Vorhand);
        assert_eq!(view.declarer_cards, Some(view.hand));
        let l = lesson(&view).expect("Null Ouvert opening");
        assert_eq!(l.rule, c.expected);
        assert_eq!(l.knowledge, c.knowledge);
        assert!(l.check(Step::Narrow, Answer::Card(l.rule.card)));
        let mut unrevealed = view.clone();
        unrevealed.declarer_cards = None;
        assert!(lesson(&unrevealed).is_none());
        let (cards, winner) = c.paths[0];
        for card in cards {
            game.apply(Action::Play(card)).unwrap();
        }
        assert_eq!(game.view(Seat::Vorhand).tricks[0].winner, winner);
    }
}

#[test]
fn null_lead_whole_lesson_and_every_answer() {
    for c in cases() {
        let l = lesson(&c.view).expect(c.id);
        assert_eq!(l.rule, c.expected);
        assert_eq!(l.knowledge, c.knowledge);
        assert_eq!(
            l.source,
            Source::Lead {
                index: 0,
                seat: Seat::Vorhand
            }
        );
        assert_eq!(l.remaining, None);
        assert_eq!(l.lead_declarer, Some(true));
        assert_eq!(l.memory().skat, c.view.own_skat);
        assert!(l.memory().trumpf.is_none());
        assert!(l.check(
            Step::Orient,
            Answer::Lead {
                seat: Seat::Vorhand,
                declarer: true
            }
        ));
        assert!(!l.check(
            Step::Orient,
            Answer::Lead {
                seat: Seat::Vorhand,
                declarer: false
            }
        ));
        for n in 0..32 {
            let card = card(n);
            assert_eq!(
                l.check(Step::Check, Answer::Card(card)),
                l.rule.candidates.contains(&card)
            );
            assert_eq!(
                l.check(Step::Narrow, Answer::Card(card)),
                card == l.rule.card
            );
        }
    }
}
#[test]
fn null_lead_non_absent_entire_lesson_privacy() {
    let cs = cases();
    for (a, b) in [(0, 1), (2, 3)] {
        let l = lesson(&cs[a].view).expect("new full lesson, never absent equality");
        assert_eq!(Some(l), lesson(&cs[b].view));
    }
}
#[test]
fn null_lead_named_boundaries_never_new_family_and_real_otherlead_histories() {
    for (id, v, _) in fixture::boundaries() {
        if id == "defender" {
            continue;
        }
        assert!(
            lesson(&v).is_none_or(|l| l.game != GameType::Null || l.lead_declarer.is_none()),
            "{id}"
        );
    }
    let c = cases().remove(0);
    for history in [
        vec![Action::Bid(18), Action::Pass, Action::Pass],
        vec![Action::Pass, Action::Bid(18), Action::Pass],
    ] {
        let mut g = Game::from_deal(c.deal, RuleConfig { kontra_re: false });
        for a in history.into_iter().chain([
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
        ]) {
            g.apply(a).unwrap();
        }
        let seat = g.declarer().unwrap();
        assert_ne!(seat, Seat::Vorhand);
        assert!(lesson(&g.view(seat)).is_none());
        assert_eq!(g.current_seat(), Some(Seat::Vorhand));
    }
}

#[test]
fn null_lead_additional_inconsistent_and_nonexact_api_guards() {
    let v = cases().remove(0).view;
    type Edit = fn(&mut PlayerView);
    let edits: &[Edit] = &[
        |v| v.hand_game = None,
        |v| v.hand_game = Some(false),
        |v| v.contract.as_mut().unwrap().schneider = true,
        |v| v.contract.as_mut().unwrap().schwarz = true,
        |v| v.legal_actions.push(Action::Play(card(1))),
        |v| v.legal_actions.push(Action::Play(card(0))),
        |v| v.hand_sizes = [9, 10, 10],
        |v| v.declarer_cards = Some(v.hand),
        |v| v.current_seat = None,
        |v| v.declarer = None,
        |v| v.contract = None,
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut x = v.clone();
        edit(&mut x);
        assert!(lesson(&x).is_none(), "edit{i}");
    }
    let mut pickup = cases().remove(2).view;
    pickup.own_skat = Some(CardSet::from_bits(1));
    assert!(lesson(&pickup).is_none());
    pickup.own_skat = Some(CardSet::from_bits((1 << 2) | (1 << 3)));
    assert!(lesson(&pickup).is_none());
}

#[test]
fn defender_opening_is_core_validated_and_keeps_hidden_deals_private() {
    let mut games = Vec::new();
    let mut hands = [
        CardSet::from_bits(2214726045),
        CardSet::from_bits(156542018),
        CardSet::from_bits(313086496),
    ];
    let mut skat = CardSet::from_bits(1610612736);
    for swapped in [false, true] {
        if swapped {
            let a = hands[1].iter().next().unwrap();
            let b = skat.iter().next().unwrap();
            hands[1].remove(a);
            hands[1].insert(b);
            skat.remove(b);
            skat.insert(a);
        }
        let mut g = Game::from_deal(
            Deal::new(hands, skat).unwrap(),
            RuleConfig { kontra_re: false },
        );
        for a in [
            Action::Bid(18),
            Action::Pass,
            Action::Pass,
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
        ] {
            g.apply(a).unwrap();
        }
        games.push(g);
    }
    let view = games[0].view(Seat::Vorhand);
    assert_ne!(view.declarer, Some(Seat::Vorhand));
    let l = lesson(&view).expect("defender opening");
    assert_eq!(l, lesson(&games[1].view(Seat::Vorhand)).unwrap());
    assert_eq!(l.rule.card, card(0)); // first zero-Augen seven in legal order
    assert_eq!(l.lead_declarer, Some(false));
    assert!(l.check(
        Step::Orient,
        Answer::Lead {
            seat: Seat::Vorhand,
            declarer: false
        }
    ));
    for action in view.legal_actions {
        let mut game = games[0].clone();
        game.apply(action).unwrap();
        for _ in 0..2 {
            let seat = game.current_seat().unwrap();
            let action = game.view(seat).legal_actions[0];
            game.apply(action).unwrap();
        }
        assert_eq!(game.view(Seat::Vorhand).tricks.len(), 1);
    }
    let mut bad = games[0].view(Seat::Vorhand);
    bad.contract.as_mut().unwrap().ouvert = true;
    assert!(lesson(&bad).is_none());
}

#[test]
fn ouvert_defender_opening_respects_reveal_and_hidden_partner() {
    let mut hands = [
        CardSet::from_bits(2214726045),
        CardSet::from_bits(156542018),
        CardSet::from_bits(313086496),
    ];
    let mut skat = CardSet::from_bits(1610612736);
    let mut games = Vec::new();
    for swapped in [false, true] {
        if swapped {
            let a = hands[2].iter().next().unwrap();
            let b = skat.iter().next().unwrap();
            hands[2].remove(a);
            hands[2].insert(b);
            skat.remove(b);
            skat.insert(a);
        }
        let mut game = Game::from_deal(
            Deal::new(hands, skat).unwrap(),
            RuleConfig { kontra_re: false },
        );
        let mut announcement = Announcement::plain(GameType::Null);
        announcement.ouvert = true;
        for action in [
            Action::Bid(18),
            Action::Pass,
            Action::Pass,
            Action::Hand,
            Action::Announce(announcement),
        ] {
            game.apply(action).unwrap();
        }
        games.push(game);
    }
    let view = games[0].view(Seat::Vorhand);
    assert_ne!(view.declarer, Some(Seat::Vorhand));
    let revealed = view.declarer_cards.unwrap();
    assert_eq!(revealed.len(), 10);
    let l = lesson(&view).unwrap();
    assert_eq!(l, lesson(&games[1].view(Seat::Vorhand)).unwrap());
    assert_eq!(l.knowledge.known_elsewhere, revealed);
    assert_eq!(l.rule, skat_analysis::baseline::rule(&view).unwrap());
    assert_eq!(l.lead_declarer, Some(false));
    for action in &view.legal_actions {
        let Action::Play(card) = action else {
            panic!("card action")
        };
        assert!(l.check(Step::Check, Answer::Card(*card)));
        let mut game = games[0].clone();
        game.apply(*action).unwrap();
        for _ in 0..2 {
            let seat = game.current_seat().unwrap();
            let reply = game.view(seat).legal_actions[0];
            game.apply(reply).unwrap();
        }
        assert_eq!(game.view(Seat::Vorhand).tricks.len(), 1);
    }
    for cards in [None, Some(view.hand), Some(CardSet::EMPTY)] {
        let mut bad = view.clone();
        bad.declarer_cards = cards;
        assert!(lesson(&bad).is_none());
    }
    let mut bad = view.clone();
    bad.legal_actions.pop();
    assert!(lesson(&bad).is_none());
}
