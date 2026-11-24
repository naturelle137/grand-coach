// SPDX-License-Identifier: EUPL-1.2

//! Compare declaration policies on held-out views and fresh sampled worlds.
//! Read rare_positions JSONL on stdin. Argument: samples per view and seed.
use nn_train::selfplay::{RunSeed, Split};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use skat_ai::{Level, Opponent};
use skat_analysis::{
    Budget,
    sampling::{HiddenCards, World},
};
use skat_core::{Action, Call, Deal, Game, Phase, PlayerView};
use std::io::{self, BufRead};

fn replay(view: &PlayerView, world: World) -> Option<Game> {
    if view.phase != Phase::Ansage
        || view.hand_game != Some(true)
        || view.declarer != Some(view.seat)
        || view.current_seat != Some(view.seat)
    {
        return None;
    }
    let mut game = Game::from_deal(Deal::new(world.hands, world.skat).ok()?, view.config);
    for action in view
        .bidding
        .iter()
        .map(|r| match r.call {
            Call::Bid(v) => Action::Bid(v),
            Call::Hold(_) => Action::Hold,
            Call::Pass => Action::Pass,
        })
        .chain([Action::Hand])
    {
        game.apply(action).ok()?;
    }
    (game.view(view.seat) == *view).then_some(game)
}

fn play(mut game: Game, action: Action, players: &[Opponent; 3]) -> Option<(i32, u32)> {
    let declarer = game.current_seat()?;
    game.apply(action).ok()?;
    let mut fallbacks = 0;
    while let Some(seat) = game.current_seat() {
        let decision = players[seat.index()].decide(&game.view(seat), None)?;
        fallbacks += u32::from(decision.fallback);
        game.apply(decision.action).ok()?;
    }
    Some((game.result()?.seeger_fabian()[declarer.index()], fallbacks))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let samples: u32 = args.next().expect("samples").parse().expect("samples");
    let seed: u64 = args.next().expect("seed").parse().expect("seed");
    assert!(
        samples > 0 && args.next().is_none(),
        "expected positive samples and seed"
    );
    let mut last = None;
    let mut differences = Vec::new();
    let mut teacher_fallbacks = 0u32;
    let mut play_fallbacks = [0u32; 2];
    let mut candidate_fallbacks = 0u32;
    for line in io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        if value["split"] != "Test" && value["split"] != "test" {
            continue;
        }
        let deal = value["deal"].as_u64().unwrap();
        if last == Some(deal) {
            continue;
        }
        assert!(
            last.is_none_or(|previous| deal > previous),
            "input must be ordered by deal"
        );
        last = Some(deal);
        let run = RunSeed(value["run_seed"].as_u64().unwrap());
        assert_eq!(
            Split::of(run.deal_seed(deal)),
            Split::Test,
            "incorrect test split"
        );
        let view: PlayerView = serde_json::from_value(value["view"].clone()).unwrap();
        let mut profile = Level::Strong.profile();
        profile.search_declaring = true;
        profile.budget = Some(Budget {
            samples: 1,
            nodes_per_task: 10_000_000,
        });
        let searched = Opponent {
            profile,
            seed: run.teacher_seed(deal, view.seat),
        };
        let classic = Opponent::new(Level::Strong, run.teacher_seed(deal, view.seat));
        let decisions = [
            classic.decide(&view, None).unwrap(),
            searched.decide(&view, None).unwrap(),
        ];
        teacher_fallbacks += u32::from(decisions[1].fallback);
        let players = skat_core::Seat::ALL
            .map(|seat| Opponent::new(Level::Easy, run.teacher_seed(deal, seat)));
        let hidden = HiddenCards::from_view(&view).expect("consistent permitted view");
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ deal);
        let mut sums = [0i64; 2];
        let candidates: Vec<_> = view
            .legal_actions
            .iter()
            .copied()
            .filter(|action| matches!(action, Action::Announce(a) if a.schneider && !a.ouvert))
            .collect();
        assert!(!candidates.is_empty(), "no announcement candidates");
        let mut candidate_differences = vec![(0.0, 0.0); candidates.len()];
        for _ in 0..samples {
            let world = hidden.sample(&mut rng);
            let mut scores = [0; 2];
            for i in 0..2 {
                let game = replay(&view, world).expect("replay sampled view");
                let (score, fallbacks) =
                    play(game, decisions[i].action, &players).expect("complete game");
                scores[i] = score;
                sums[i] += i64::from(score);
                play_fallbacks[i] += fallbacks;
            }
            for (i, action) in candidates.iter().enumerate() {
                let game = replay(&view, world).expect("replay sampled view");
                let (score, fallbacks) = play(game, *action, &players).expect("complete game");
                candidate_fallbacks += fallbacks;
                let difference = f64::from(score - scores[0]);
                candidate_differences[i].0 += difference;
                candidate_differences[i].1 += difference * difference;
            }
        }
        let candidate_results: Vec<_> = candidates.iter().zip(candidate_differences).map(|(action, (sum, squares))| {
            let n = f64::from(samples);
            let mean = sum / n;
            let se = (samples > 1).then(|| ((squares - sum * sum / n).max(0.0) / (n * (n - 1.0))).sqrt());
            serde_json::json!({"action": action, "minus_classic": mean, "standard_error_across_worlds": se})
        }).collect();
        let diff = (sums[1] - sums[0]) as f64 / f64::from(samples);
        differences.push(diff);
        println!(
            "{}",
            serde_json::json!({"deal": deal, "classic": decisions[0].action, "searched": decisions[1].action, "searched_fallback": decisions[1].fallback, "classic_mean": sums[0] as f64 / f64::from(samples), "searched_mean": sums[1] as f64 / f64::from(samples), "candidates": candidate_results})
        );
    }
    let n = differences.len();
    assert!(n > 0, "no held-out test views");
    let mean = differences.iter().sum::<f64>() / n as f64;
    let se = (n > 1).then(|| {
        (differences.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / ((n - 1) * n) as f64).sqrt()
    });
    eprintln!(
        "{}",
        serde_json::json!({"test_views": n, "samples_per_view": samples, "sample_seed": seed, "searched_minus_classic": mean, "standard_error_across_deals": se, "searched_fallbacks": teacher_fallbacks, "play_fallbacks_classic_and_searched": play_fallbacks, "play_fallbacks_candidates": candidate_fallbacks})
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::Seat;

    #[test]
    fn sampled_hand_views_replay_and_play_reproducibly() {
        let run = RunSeed(7);
        let players = Seat::ALL.map(|seat| Opponent::new(Level::Easy, run.teacher_seed(0, seat)));
        let mut game = Game::new(run.deal_seed(0), nn_train::selfplay::RULES);
        let view = loop {
            let seat = game.current_seat().unwrap();
            let view = game.view(seat);
            if view.phase == Phase::Ansage {
                break view;
            }
            let action = if view.phase == Phase::SkatAufnahme {
                Action::Hand
            } else {
                players[seat.index()].decide(&view, None).unwrap().action
            };
            game.apply(action).unwrap();
        };
        let hidden = HiddenCards::from_view(&view).unwrap();
        let world = hidden.sample(&mut ChaCha8Rng::seed_from_u64(42));
        let action = players[view.seat.index()]
            .decide(&view, None)
            .unwrap()
            .action;
        let a = play(replay(&view, world).unwrap(), action, &players).unwrap();
        assert_eq!(
            a,
            play(replay(&view, world).unwrap(), action, &players).unwrap()
        );
        let mut invalid = view.clone();
        invalid.hand_game = Some(false);
        assert!(replay(&invalid, world).is_none());
    }
}
