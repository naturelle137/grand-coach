// SPDX-License-Identifier: EUPL-1.2

//! A Null defender playing last while the partner currently wins the trick.
use skat_coach::guided::{Answer, Step, lesson};
use skat_core::{Action, Announcement, Deal, Game, GameType, RuleConfig, Seat};

fn game(seed: u64, ouvert: bool) -> Game {
    let mut game = Game::from_deal(Deal::from_seed(seed), RuleConfig { kontra_re: false });
    for action in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement {
            ouvert,
            ..Announcement::plain(GameType::Null)
        }),
    ] {
        game.apply(action).unwrap();
    }
    for _ in 0..2 {
        let seat = game.current_seat().unwrap();
        let legal = game.view(seat).legal_actions;
        game.apply(legal[legal.len() / 2]).unwrap();
    }
    game
}

#[test]
fn defender_last_with_partner_ahead_uses_easy_fallback_for_every_legal_card() {
    for ouvert in [false, true] {
        let game = game(2, ouvert);
        let view = game.view(Seat::Hinterhand);
        assert_ne!(view.declarer, Some(Seat::Hinterhand));
        let l = lesson(&view).expect("last-seat partner-ahead lesson");
        assert_eq!(l.rule, skat_analysis::baseline::rule(&view).unwrap());
        assert!(l.rule.last && l.remaining.is_none());
        assert_ne!(l.rule.winner, view.declarer);
        assert_eq!(l.rule.candidates.len(), view.legal_actions.len());
        assert!(l.check(Step::Orient, Answer::Seat(l.rule.winner)));
        assert!(l.check(Step::Narrow, Answer::Card(l.rule.card)));
        assert!(
            l.rule
                .candidates
                .iter()
                .filter(|&&c| c != l.rule.card)
                .all(|&c| !l.check(Step::Narrow, Answer::Card(c)))
        );
        // Every legal card stays playable and completes the trick.
        for action in &view.legal_actions {
            let mut next = game.clone();
            next.apply(*action).unwrap();
            assert_eq!(next.view(Seat::Hinterhand).tricks.len(), 1);
        }
        let mut bad = view.clone();
        bad.legal_actions.reverse();
        assert!(lesson(&bad).is_none());
        if ouvert {
            let mut bad = view.clone();
            bad.declarer_cards = None;
            assert!(lesson(&bad).is_none());
        }
    }
}
