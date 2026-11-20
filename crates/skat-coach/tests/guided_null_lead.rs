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
