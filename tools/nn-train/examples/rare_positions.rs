// SPDX-License-Identifier: EUPL-1.2

//! Diagnostic synthetic positions, not teacher labels or training shards.
//! Strong choices are diagnostic, not assessed training targets.
//! Arguments: run seed and maximum number of deals. JSONL goes to stdout.

use nn_train::selfplay::{RULES, RunSeed, Split};
use serde::Serialize;
use skat_ai::{Level, Opponent};
use skat_core::{Action, Game, Phase, PlayerView, Seat};

#[derive(Debug, Serialize)]
struct Position {
    source: &'static str,
    run_seed: u64,
    deal: u64,
    split: Split,
    prefix: Vec<Action>,
    view: PlayerView,
    synthetic_action: Action,
    strong_action: Action,
}

fn positions(seed: RunSeed, deal: u64) -> Vec<Position> {
    let mut game = Game::new(seed.deal_seed(deal), RULES);
    let players = Seat::ALL.map(|seat| Opponent::new(Level::Easy, seed.teacher_seed(deal, seat)));
    let mut prefix = Vec::new();
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        if view.phase == Phase::Ansage {
            let strong_action = Opponent::new(Level::Strong, seed.teacher_seed(deal, seat))
                .decide(&view, None)
                .expect("turn")
                .action;
            return view
                .legal_actions
                .iter()
                .copied()
                .filter(|action| matches!(action, Action::Announce(a) if a.schneider && !a.ouvert))
                .map(|synthetic_action| Position {
                    source: "synthetic-legal-position-v1",
                    run_seed: seed.0,
                    deal,
                    split: Split::of(seed.deal_seed(deal)),
                    prefix: prefix.clone(),
                    view: view.clone(),
                    synthetic_action,
                    strong_action,
                })
                .collect();
        }
        let action = if view.phase == Phase::SkatAufnahme {
            Action::Hand
        } else {
            players[seat.index()]
                .decide(&view, None)
                .expect("turn")
                .action
        };
        game.apply(action).expect("legal action");
        prefix.push(action);
    }
    Vec::new()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seed = RunSeed(args.next().expect("run seed").parse().expect("run seed"));
    let deals: u64 = args
        .next()
        .expect("maximum deals")
        .parse()
        .expect("maximum deals");
    assert!(args.next().is_none(), "expected run seed and maximum deals");
    for deal in 0..deals {
        for position in positions(seed, deal) {
            println!("{}", serde_json::to_string(&position).expect("JSON"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_positions_replay_with_legal_actions_and_deal_splits() {
        let seed = RunSeed(7);
        let mut counts = [0; 3];
        for deal in 0..100 {
            for position in positions(seed, deal) {
                let mut game = Game::new(seed.deal_seed(deal), RULES);
                for action in &position.prefix {
                    game.apply(*action).unwrap();
                }
                let seat = game.current_seat().unwrap();
                assert_eq!(position.view, game.view(seat));
                assert!(
                    position
                        .view
                        .legal_actions
                        .contains(&position.synthetic_action)
                );
                assert!(
                    position
                        .view
                        .legal_actions
                        .contains(&position.strong_action)
                );
                assert_eq!(
                    position.strong_action,
                    Opponent::new(Level::Strong, seed.teacher_seed(deal, seat))
                        .decide(&position.view, None)
                        .unwrap()
                        .action
                );
                game.apply(position.synthetic_action).unwrap();
                assert_eq!(position.split, Split::of(seed.deal_seed(deal)));
                counts[match position.split {
                    Split::Train => 0,
                    Split::Validation => 1,
                    Split::Test => 2,
                }] += 1;
            }
        }
        assert!(counts.iter().all(|&n| n > 0));
    }
}
