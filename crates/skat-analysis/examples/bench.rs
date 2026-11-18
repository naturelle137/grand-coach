// SPDX-License-Identifier: EUPL-1.2

//! Latency benchmark of [`skat_analysis::analyze`] for every decision phase
//! (see `docs/analysis.md`). Positions come from seeded games in which the
//! continuation policy and double-dummy card play decide every seat.
//!
//! ```sh
//! cargo run --locked --release -p skat-analysis --example bench
//! ```
//!
//! Optional arguments: number of deals per phase (default 10), samples and
//! nodes per task (default `Budget::INTERACTIVE`). `--two-stage` adds a
//! same-seed paired analysis; `--repeat=N` changes the deterministic order.
//! Paired rows include full analyses, screening report, nodes and times.
//!
//! Estimation quality: every position is analysed a second time with
//! another seed. `se` is the median standard error of the best option's
//! expected score; `regret` is the mean loss, valued by the first run, of
//! choosing the second run's best option instead of the first run's.

use std::time::{Duration, Instant};

use skat_analysis::{Analysis, Budget, Status, analyze, policy};
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig};

/// Decision kinds measured.
const KINDS: [&str; 8] = [
    "Reizen",
    "SkatAufnahme",
    "Druecken",
    "Ansage",
    "Kontra/Re",
    "Spiel trick 1",
    "Spiel trick 4",
    "Spiel trick 8",
];

/// Best option: index, expected score and its standard error.
fn best(a: &Analysis) -> Option<(usize, f64, f64)> {
    a.options
        .iter()
        .enumerate()
        .filter_map(|(i, o)| o.estimate.map(|e| (i, e.expected_score, e.score_std_error)))
        .max_by(|x, y| x.1.total_cmp(&y.1))
}

fn kind(view: &PlayerView) -> Option<usize> {
    let trick = view.tricks.len();
    Some(match view.phase {
        Phase::Reizen => 0,
        Phase::SkatAufnahme => 1,
        Phase::Druecken => 2,
        Phase::Ansage => 3,
        Phase::Spiel
            if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, Action::Kontra | Action::Re | Action::KeinRe)) =>
        {
            4
        }
        Phase::Spiel if trick == 0 => 5,
        Phase::Spiel if trick == 3 => 6,
        Phase::Spiel if trick == 7 => 7,
        _ => return None,
    })
}

/// A fixed, cheap move for advancing the benchmark games.
fn advance(view: &PlayerView) -> Action {
    let legal = &view.legal_actions;
    let wanted = match view.phase {
        Phase::Reizen => policy::bid(policy::choose_game(view.hand).1, view.bid, legal),
        Phase::SkatAufnahme => policy::hand_or_pickup(view.hand),
        Phase::Druecken => Action::Discard(policy::discard(view.hand)),
        Phase::Ansage => policy::announce(
            view.hand,
            view.own_skat.unwrap_or_default(),
            view.bid,
            view.hand_game == Some(true),
            legal,
        ),
        _ => legal
            .iter()
            .copied()
            .find(|a| matches!(a, Action::Play(_) | Action::KeinRe | Action::KeinKontra))
            .unwrap_or(legal[0]),
    };
    if legal.contains(&wanted) {
        wanted
    } else {
        legal[0]
    }
}

fn main() {
    let paired = std::env::args().any(|a| a == "--two-stage");
    let repeat = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--repeat=")
                .map(|n| n.parse::<usize>().expect("repeat"))
        })
        .unwrap_or(0);
    let mut args = std::env::args()
        .skip(1)
        .filter(|a| a != "--two-stage" && !a.starts_with("--repeat="));
    let deals: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(10);
    let mut budget = Budget::INTERACTIVE;
    if let Some(samples) = args.next().and_then(|a| a.parse().ok()) {
        budget.samples = samples;
    }
    if let Some(nodes) = args.next().and_then(|a| a.parse().ok()) {
        budget.nodes_per_task = nodes;
    }
    let mut times: Vec<Vec<Duration>> = vec![Vec::new(); KINDS.len()];
    let mut incomplete = vec![0; KINDS.len()];
    let mut nodes: Vec<Vec<u64>> = vec![Vec::new(); KINDS.len()];
    let mut errors: Vec<Vec<f64>> = vec![Vec::new(); KINDS.len()];
    let mut regrets: Vec<Vec<f64>> = vec![Vec::new(); KINDS.len()];
    let mut seed = 0;
    while times.iter().any(|t| t.len() < deals) && seed < 10_000 {
        seed += 1;
        let mut game = Game::new(seed, RuleConfig { kontra_re: true });
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if let Some(k) = kind(&view)
                && times[k].len() < deals
                && !(k == 0 && view.bidding.len() > 1)
            {
                let reference = || {
                    let start = Instant::now();
                    let result = analyze(&view, budget, seed, None).expect("consistent view");
                    (result, start.elapsed())
                };
                let candidate = || {
                    let start = Instant::now();
                    let result = skat_analysis::analyze_two_stage(
                        &view,
                        budget,
                        seed,
                        None,
                        skat_analysis::Kernel::Classic,
                        skat_analysis::TwoStage::PROPOSAL,
                    )
                    .expect("consistent view");
                    (result, start.elapsed())
                };
                let (analysis, elapsed) = if paired {
                    let first = (seed as usize + times[k].len() + repeat).is_multiple_of(2);
                    let (r, c) = if first {
                        let r = reference();
                        (r, candidate())
                    } else {
                        let c = candidate();
                        (reference(), c)
                    };
                    println!(
                        "paired {k} {seed} repeat={repeat} ref_first={first} ref_ms={} candidate_ms={} ref={:?} candidate={:?}",
                        r.1.as_secs_f64() * 1000.0,
                        c.1.as_secs_f64() * 1000.0,
                        r.0,
                        c.0
                    );
                    r
                } else {
                    reference()
                };
                times[k].push(elapsed);
                nodes[k].push(analysis.effort.nodes);
                if analysis.status != Status::Complete {
                    incomplete[k] += 1;
                }
                let again = analyze(&view, budget, seed + 1_000_000, None).expect("consistent");
                if let (Some((_, score, se)), Some((j, _, _))) = (best(&analysis), best(&again)) {
                    errors[k].push(se);
                    let other = analysis.options[j]
                        .estimate
                        .map_or(score, |e| e.expected_score);
                    regrets[k].push(score - other);
                }
            }
            game.apply(advance(&view)).unwrap();
        }
    }
    println!(
        "budget: {} samples, {} nodes per task; {} threads",
        budget.samples,
        budget.nodes_per_task,
        std::thread::available_parallelism().map_or(1, |n| n.get())
    );
    println!(
        "{:<14} {:>5} {:>9} {:>9} {:>9} {:>10} {:>12} {:>6} {:>7}",
        "decision", "n", "median", "p90", "max", "incomplete", "max nodes", "se", "regret"
    );
    for (k, name) in KINDS.iter().enumerate() {
        let t = &mut times[k];
        t.sort();
        if t.is_empty() {
            println!("{name:<14} {:>5}", 0);
            continue;
        }
        let at = |q: f64| t[((t.len() - 1) as f64 * q).round() as usize];
        println!(
            "{name:<14} {:>5} {:>9.0?} {:>9.0?} {:>9.0?} {:>10} {:>12} {:>6.1} {:>7.1}",
            t.len(),
            at(0.5),
            at(0.9),
            t[t.len() - 1],
            incomplete[k],
            nodes[k].iter().max().copied().unwrap_or(0),
            median(&mut errors[k]),
            regrets[k].iter().sum::<f64>() / regrets[k].len().max(1) as f64
        );
    }
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
