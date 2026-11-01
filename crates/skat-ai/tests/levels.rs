// SPDX-License-Identifier: EUPL-1.2

//! Strength profiles: configuration, reproducible decisions, complete games
//! and the legal fallback when the analysis cannot finish (M4).

use skat_ai::arena::play;
use skat_ai::{BaselineOpponent, Budget, Cancel, Level, Opponent, Profile, Status};
use skat_core::{Game, Phase, PlayerView, RuleConfig, Seat};

const RULES: RuleConfig = RuleConfig { kontra_re: true };

#[test]
fn profiles_are_ordered_as_documented() {
    assert_eq!(
        Level::ALL,
        [Level::Rookie, Level::Easy, Level::Medium, Level::Strong]
    );
    assert!(Level::ALL.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(Level::default(), Level::Medium);
    let [rookie, easy, medium, strong] = Level::ALL.map(Level::profile);
    assert_eq!(easy, Profile::RULES);
    assert_eq!(rookie.budget, None);
    assert!(!rookie.mistakes.is_none());
    assert_eq!(easy.budget, None);
    let medium_budget = medium.budget.expect("Medium searches");
    assert_eq!(medium_budget.samples, 6);
    assert!(!medium.search_bidding && !medium.search_declaring);
    assert_eq!(strong.budget, Some(Budget::INTERACTIVE));
    assert!(!strong.search_bidding && !strong.search_declaring);
    for p in [rookie, easy, medium, strong] {
        assert!(p.temperature >= 0.0);
        assert_eq!(p.forget, 0.0);
    }
    for p in [easy, medium, strong] {
        assert!(p.mistakes.is_none());
    }
    let goldfish = Profile::GOLDFISH;
    assert_eq!(goldfish.budget, strong.budget);
    assert!(goldfish.forget > 0.0 && goldfish.mistakes.is_none());
}

/// The first card-play position of seed `seed` (rules of thumb before).
fn first_card(seed: u64) -> Option<PlayerView> {
    let mut game = Game::new(seed, RULES);
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        if view.phase == Phase::Spiel && view.tricks.len() == 1 {
            return Some(view);
        }
        game.apply(BaselineOpponent.decide(&view).unwrap()).unwrap();
    }
    None
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn decisions_are_reproducible_and_seeded() {
    let views: Vec<PlayerView> = (0..12).filter_map(first_card).take(4).collect();
    assert_eq!(views.len(), 4);
    for level in [Level::Medium, Level::Strong] {
        for view in &views {
            let a = Opponent::new(level, 1);
            let first = a.decide(view, None).unwrap();
            assert_eq!(first.status, Some(Status::Complete));
            assert!(!first.fallback);
            assert_eq!(a.decide(&view.clone(), None), Some(first));
            assert_eq!(Opponent::new(level, 1).decide(view, None), Some(first));
            let b = Opponent::new(level, 2);
            assert_ne!(a.analysis_input(view), b.analysis_input(view));
            assert!(
                view.legal_actions
                    .contains(&b.decide(view, None).unwrap().action)
            );
        }
    }
}

#[test]
fn analysed_decisions_return_their_analysis() {
    let view = first_card(0).or_else(|| first_card(1)).unwrap();
    let small = Profile {
        budget: Some(Budget {
            samples: 2,
            nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
        }),
        ..Profile::RULES
    };
    let searched = Opponent {
        profile: small,
        seed: 3,
    };
    let (decision, analysis) = searched.decide_analysed(&view, None).unwrap();
    assert_eq!(searched.decide(&view, None), Some(decision));
    let analysis = analysis.expect("card play is analysed");
    assert_eq!(decision.status, Some(analysis.status));
    let actions: Vec<_> = analysis.options.iter().map(|o| o.action).collect();
    assert_eq!(actions, view.legal_actions);
    let rules = Opponent::new(Level::Easy, 3);
    let (decision, analysis) = rules.decide_analysed(&view, None).unwrap();
    assert_eq!(analysis, None);
    assert_eq!(decision.status, None);
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn every_level_completes_seeded_games() {
    for seed in 0..2 {
        for players in [
            [Level::Easy, Level::Medium, Level::Easy],
            [Level::Medium, Level::Easy, Level::Medium],
        ] {
            let opponents = Seat::ALL.map(|s| Opponent::new(players[s.index()], seed + 10));
            let game = play(seed, RULES, &opponents);
            assert!(matches!(game.phase(), Phase::Beendet | Phase::Eingepasst));
            assert_eq!(play(seed, RULES, &opponents).result(), game.result());
        }
    }
}

#[test]
fn a_cancelled_analysis_falls_back_to_a_legal_action() {
    let cancel = Cancel::new();
    cancel.cancel();
    for seed in 0..6 {
        let Some(view) = first_card(seed) else {
            continue;
        };
        for level in [Level::Medium, Level::Strong] {
            let d = Opponent::new(level, 3)
                .decide(&view, Some(&cancel))
                .unwrap();
            assert_eq!(d.status, Some(Status::Cancelled));
            assert!(d.fallback);
            assert_eq!(Some(d.action), BaselineOpponent.decide(&view));
        }
        // The rules of thumb never search, so cancelling changes nothing.
        let easy = Opponent::new(Level::Easy, 3).decide(&view, Some(&cancel));
        assert_eq!(easy.map(|d| (d.status, d.fallback)), Some((None, false)));
    }
}

#[test]
fn an_exhausted_budget_falls_back_to_a_legal_action() {
    let tiny = Profile {
        budget: Some(Budget {
            samples: 4,
            nodes_per_task: 1,
        }),
        temperature: 0.0,
        search_bidding: true,
        search_declaring: true,
        ..Profile::RULES
    };
    let opponent = Opponent {
        profile: tiny,
        seed: 5,
    };
    let mut game = Game::new(4, RULES);
    let mut exhausted = 0;
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let d = opponent.decide(&view, None).unwrap();
        assert!(view.legal_actions.contains(&d.action));
        if d.status == Some(Status::BudgetExhausted) {
            exhausted += 1;
            assert!(d.fallback, "{d:?}");
            assert_eq!(Some(d.action), BaselineOpponent.decide(&view));
        }
        game.apply(d.action).unwrap();
    }
    assert!(exhausted > 5, "{exhausted}");
}

#[test]
fn an_inconsistent_view_falls_back_to_a_legal_action() {
    let mut view = first_card(1).unwrap();
    // Claims one card too many for the next seat: no distribution fits.
    let next = (view.seat.index() + 1) % 3;
    view.hand_sizes[next] += 1;
    let d = Opponent::new(Level::Strong, 0).decide(&view, None).unwrap();
    assert_eq!((d.status, d.fallback), (None, true));
    assert!(view.legal_actions.contains(&d.action));
}

#[test]
fn no_decision_without_turn() {
    let game = Game::new(1, RULES);
    let waiting = Seat::ALL
        .into_iter()
        .find(|&s| Some(s) != game.current_seat())
        .unwrap();
    for level in Level::ALL {
        let view = game.view(waiting);
        assert_eq!(Opponent::new(level, 0).decide(&view, None), None);
        assert_eq!(Opponent::new(level, 0).analysis_input(&view), None);
    }
}
