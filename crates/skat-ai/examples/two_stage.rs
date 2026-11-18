// SPDX-License-Identifier: EUPL-1.2

//! Paired measurement of the experimental two-stage card play (#104)
//! against the single-stage reference of the same opponent, view, seed and
//! budget (`docs/analysis.md`, "Two-stage card play"). Measurement only:
//! no default changes, no strength claim.
//!
//! ```sh
//! cargo run --locked --release -p skat-ai --example two_stage -- \
//!     <tuning|holdout> [positions per bucket] [samples] \
//!     [screen_worlds se_multiple se_floor min_keep max_keep]
//! ```
//!
//! Positions: seeded games with Kontra/Re, rules of thumb for every seat;
//! card-play decisions at tricks 1–4, declarer and defender (8 buckets,
//! at most one decision per bucket and game). Tuning deals start at seed
//! 10,001, holdout deals at 2,000,001 (disjoint from each other and from
//! the latency and bench positions). Defaults: 25 positions per bucket,
//! Strong's 24 samples, [`TwoStage::PROPOSAL`].
//!
//! Optional `--repeats=N` (default 1) times every position N times and
//! alternates the order deterministically: the reference runs first iff
//! `(position index + repeat) % 2 == 0`. The raw order, wall times, node
//! counts, analysis status, fallback flag, chosen action and the full
//! per-option utilities and sample counts of both paths are printed, so
//! equivalence is judged on estimates and not only on matching actions.
//! Repeats are timing repeats, never additional positions: the decisions
//! must be identical in every repeat (checked, the tool aborts otherwise).
//!
//! One tab-separated line per position and repeat, then a summary: share of
//! identical decisions, mean utility regret of the differing ones (valued
//! by the reference analysis), screening outcomes, nodes, and sequential
//! wall time per decision (median, p90, max). Wall times are only
//! comparable on an otherwise idle machine with the same thread setup.

use std::time::{Duration, Instant};

use skat_ai::{Analysis, BaselineOpponent, Budget, Level, Opponent, Profile, utility};
use skat_analysis::{StageOutcome, TwoStage};
use skat_core::{Game, Phase, PlayerView, RuleConfig};

/// Status, fallback, nodes and per-option `action=utility@samples` of one
/// path. `analysis` is `None` for rules-only decisions and failed analyses:
/// reported as unavailable, never as complete.
fn describe(view: &PlayerView, analysis: &Option<Analysis>, fallback: bool) -> String {
    let Some(a) = analysis else {
        return format!("unavailable\t{fallback}\t-");
    };
    let options: Vec<String> = a
        .options
        .iter()
        .map(|o| match o.estimate {
            Some(e) => format!("{:?}={:.3}@{}", o.action, utility(view, &e), e.samples),
            None => format!("{:?}=-", o.action),
        })
        .collect();
    format!("{:?}\t{fallback}\t{}", a.status, options.join(";"))
}

/// Tricks 1–4, declarer and defender.
const BUCKETS: usize = 8;

fn bucket(view: &PlayerView) -> Option<usize> {
    let trick = view.tricks.len();
    if view.phase != Phase::Spiel || trick > 3 {
        return None;
    }
    let defender = usize::from(view.declarer != Some(view.seat));
    Some(trick * 2 + defender)
}

fn positions(first_seed: u64, per_bucket: usize) -> Vec<(u64, PlayerView)> {
    let mut counts = [0; BUCKETS];
    let mut out = Vec::new();
    let mut seed = first_seed;
    while counts.iter().any(|&n| n < per_bucket) && seed < first_seed + 1_000_000 {
        let mut game = Game::new(seed, RuleConfig { kontra_re: true });
        let mut taken = [false; BUCKETS];
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if let Some(b) = bucket(&view)
                && !taken[b]
                && counts[b] < per_bucket
            {
                taken[b] = true;
                counts[b] += 1;
                out.push((seed, view.clone()));
            }
            game.apply(BaselineOpponent.decide(&view).expect("legal action"))
                .expect("rules of thumb pick legal actions");
        }
        seed += 1;
    }
    out
}

fn percentile(sorted: &[Duration], q: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

fn main() {
    let mut repeats = 1usize;
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| match a.strip_prefix("--repeats=") {
            Some(n) => {
                repeats = n.parse().ok().filter(|&n| n > 0).expect("repeat count");
                false
            }
            None => true,
        })
        .collect();
    let first_seed = match args.first().map(String::as_str) {
        Some("tuning") => 10_001,
        Some("holdout") => 2_000_001,
        _ => {
            eprintln!(
                "usage: two_stage <tuning|holdout> [positions per bucket] [samples] \
                 [screen_worlds se_multiple se_floor min_keep max_keep] [--repeats=N]"
            );
            std::process::exit(2);
        }
    };
    let arg = |i: usize| args.get(i).map(|a| a.parse::<f64>().expect("number"));
    let per_bucket = arg(1).map_or(25, |n| n as usize);
    let samples = arg(2).map_or(Budget::INTERACTIVE.samples, |n| n as u32);
    let p = TwoStage::PROPOSAL;
    let screening = TwoStage {
        screen_worlds: arg(3).map_or(p.screen_worlds, |n| n as u32),
        se_multiple: arg(4).unwrap_or(p.se_multiple),
        se_floor: arg(5).unwrap_or(p.se_floor),
        min_keep: arg(6).map_or(p.min_keep, |n| n as u32),
        max_keep: arg(7).map_or(p.max_keep, |n| n as u32),
    };
    let opponent = Opponent {
        profile: Profile {
            budget: Some(Budget {
                samples,
                ..Budget::INTERACTIVE
            }),
            ..Level::Strong.profile()
        },
        seed: 1,
    };
    println!(
        "# suite {} first seed {first_seed}; {samples} samples; {screening:?}; {} threads",
        args[0],
        std::thread::available_parallelism().map_or(1, |n| n.get())
    );
    println!(
        "key\tseed\trepeat\tseat\ttrick\trole\toptions\torder\toutcome\tsurvivors\tsame\tregret\t\
         ref_nodes\tts_nodes\tref_ms\tts_ms\tref_action\tts_action\t\
         ref_status\tref_fallback\tref_estimates\tts_status\tts_fallback\tts_estimates"
    );
    let (mut same, mut total, mut regrets) = (0, 0, Vec::new());
    let (mut ref_times, mut ts_times) = (Vec::new(), Vec::new());
    let (mut ref_nodes, mut ts_nodes) = (0u64, 0u64);
    let mut outcomes: Vec<(StageOutcome, usize)> = Vec::new();
    let positions = positions(first_seed, per_bucket);
    let mut first_actions = Vec::new();
    for repeat in 0..repeats {
        for (index, (seed, view)) in positions.iter().enumerate() {
            let reference_first = (index + repeat) % 2 == 0;
            let run_reference = || {
                let start = Instant::now();
                let r = opponent.decide_analysed(view, None).expect("a decision");
                (r, start.elapsed())
            };
            let run_two_stage = || {
                let start = Instant::now();
                let r = opponent
                    .decide_two_stage(view, None, screening)
                    .expect("a decision");
                (r, start.elapsed())
            };
            let (((reference, analysis), ref_time), ((decision, two_stage, report), ts_time)) =
                if reference_first {
                    let r = run_reference();
                    (r, run_two_stage())
                } else {
                    let t = run_two_stage();
                    (run_reference(), t)
                };
            let actions = (reference.action, decision.action);
            if repeat == 0 {
                first_actions.push(actions);
            } else {
                assert_eq!(first_actions[index], actions, "non-deterministic decision");
            }
            let value = |action| {
                analysis.as_ref().and_then(|a| {
                    a.options
                        .iter()
                        .find(|o| o.action == action)
                        .and_then(|o| o.estimate)
                        .map(|e| utility(view, &e))
                })
            };
            let regret = value(reference.action)
                .zip(value(decision.action))
                .map(|(r, c)| r - c);
            let identical = reference.action == decision.action;
            let outcome = report.as_ref().map(|r| r.outcome);
            let nodes = |a: &Option<Analysis>| a.as_ref().map_or(0, |a| a.effort.nodes);
            // Outcomes and decision quality count once per position.
            if repeat == 0 {
                total += 1;
                if identical {
                    same += 1;
                } else if let Some(r) = regret {
                    regrets.push(r);
                }
                if let Some(o) = outcome {
                    match outcomes.iter_mut().find(|(k, _)| *k == o) {
                        Some((_, n)) => *n += 1,
                        None => outcomes.push((o, 1)),
                    }
                }
            }
            ref_nodes += nodes(&analysis);
            ts_nodes += nodes(&two_stage);
            ref_times.push(ref_time);
            ts_times.push(ts_time);
            let role = if view.declarer == Some(view.seat) {
                "declarer"
            } else {
                "defender"
            };
            println!(
                "{seed}-{}-{role}\t{seed}\t{repeat}\t{:?}\t{}\t{role}\t{}\t{}\t{outcome:?}\t{:?}\t{identical}\t{}\t{}\t{}\t{:.1}\t{:.1}\t{:?}\t{:?}\t{}\t{}",
                view.tricks.len() + 1,
                view.seat,
                view.tricks.len() + 1,
                view.legal_actions.len(),
                if reference_first {
                    "ref-first"
                } else {
                    "ts-first"
                },
                report.as_ref().map(|r| r.survivors.clone()),
                regret.map_or("-".into(), |r| format!("{r:.3}")),
                nodes(&analysis),
                nodes(&two_stage),
                ref_time.as_secs_f64() * 1e3,
                ts_time.as_secs_f64() * 1e3,
                reference.action,
                decision.action,
                describe(view, &analysis, reference.fallback),
                describe(view, &two_stage, decision.fallback),
            );
        }
    }
    ref_times.sort();
    ts_times.sort();
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    println!(
        "# repeats {repeats}; positions {total}, identical {same} ({:.1} %)",
        { 100.0 * same as f64 / total.max(1) as f64 }
    );
    println!(
        "# differing {}: mean regret {:.3}, max {:.3}",
        regrets.len(),
        mean(&regrets),
        regrets.iter().copied().fold(0.0, f64::max)
    );
    for (o, n) in &outcomes {
        println!("# outcome {o:?}: {n}");
    }
    println!("# nodes (all repeats) reference {ref_nodes}, two-stage {ts_nodes}");
    for (name, t) in [("reference", &ref_times), ("two-stage", &ts_times)] {
        println!(
            "# wall {name}: median {:.0?}, p90 {:.0?}, max {:.0?}",
            percentile(t, 0.5),
            percentile(t, 0.9),
            t.last().copied().unwrap_or_default()
        );
    }
}
