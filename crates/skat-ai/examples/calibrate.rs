// SPDX-License-Identifier: EUPL-1.2

//! Calibration runner for the strength levels: AI-vs-AI duplicate games on
//! a fixed suite of seeded deals (protocol in `docs/ai.md`).
//!
//! ```sh
//! cargo run --locked --release -p skat-ai --example calibrate -- \
//!     validation medium strong 200
//! ```
//!
//! Arguments: suite (`tuning` or `validation`), profile A, profile B,
//! number of deals (default 100), parallel deals (default 4), index of the
//! first deal in the suite (default 0). A profile is
//! a level (`rookie`, `easy`, `medium`, `strong`, or `goldfish`) or a custom profile `s<samples>t<temperature>`
//! for tuning, e.g. `s6t4` (suffix `p`: analysis in card
//! play and Kontra/Re only; `d`: also Hand, Drücken and announcement), or a
//! learned model `nn:<path>[@<level>]` (D-07): the model file at `<path>`
//! decides and the Classic `<level>` (default `easy`) takes every fallback.
//! Kontra/Re is on.
//!
//! Prints a Markdown summary: the mean per-deal difference of Seeger-Fabian
//! points per seat-game (A minus B) with its standard error and 95 %
//! interval, plus declarer and defender results per profile.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_ai::arena::{DealComparison, Interval, Tally, compare_with};
use skat_ai::{Budget, Cancel, Decision, Level, NeuralOpponent, Opponent, Player, Profile};
use skat_core::{PlayerView, RuleConfig, Seat};
use skat_nn::NeuralModel;

/// Key of the deal seeds of each suite; tuning and validation never share
/// deals.
const TUNING: u64 = 0x7475_6e69_6e67;
const VALIDATION: u64 = 0x7661_6c69_6461;
/// Key of the opponents' seeds, unrelated to the deal seeds.
const AI_KEY: u64 = 0x6f70_706f_6e65;

fn profile(spec: &str) -> Profile {
    match spec {
        "rookie" => Level::Rookie.profile(),
        "easy" => Level::Easy.profile(),
        "goldfish" => Profile::GOLDFISH,
        "medium" => Level::Medium.profile(),
        "strong" => Level::Strong.profile(),
        custom => {
            // Suffix `p`: analysis in card play only; `d`: also Hand or
            // pickup, Drücken and announcement; none: every phase.
            let (custom, bidding, declaring) = if let Some(rest) = custom.strip_suffix('p') {
                (rest, false, false)
            } else if let Some(rest) = custom.strip_suffix('d') {
                (rest, false, true)
            } else {
                (custom, true, true)
            };
            let (samples, temperature) = custom
                .strip_prefix('s')
                .and_then(|r| r.split_once('t'))
                .expect("profile: easy, medium, strong or s<samples>t<temperature>");
            Profile {
                budget: Some(Budget {
                    samples: samples.parse().expect("samples"),
                    nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
                }),
                temperature: temperature.parse().expect("temperature"),
                search_bidding: bidding,
                search_declaring: declaring,
                ..Profile::RULES
            }
        }
    }
}

/// One side of the comparison: a Classic profile, or a learned model with
/// a Classic fallback profile.
#[derive(Clone)]
enum Side {
    Classic(Profile),
    Neural(Arc<NeuralModel>, Profile),
}

enum Seated {
    Classic(Opponent),
    Neural(NeuralOpponent),
}

impl Player for Seated {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        match self {
            Seated::Classic(o) => Player::decide(o, view, cancel),
            Seated::Neural(o) => Player::decide(o, view, cancel),
        }
    }
}

impl Side {
    fn parse(spec: &str) -> Side {
        let Some(rest) = spec.strip_prefix("nn:") else {
            return Side::Classic(profile(spec));
        };
        let (path, fallback) = rest.rsplit_once('@').unwrap_or((rest, "easy"));
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let hash = skat_nn::sha256(&bytes);
        let model = NeuralModel::load(&bytes, &hash).unwrap_or_else(|e| panic!("{path}: {e:?}"));
        Side::Neural(Arc::new(model), profile(fallback))
    }

    fn seat(&self, seed: u64) -> Seated {
        match self {
            Side::Classic(p) => Seated::Classic(Opponent { profile: *p, seed }),
            Side::Neural(model, p) => Seated::Neural(NeuralOpponent::new(
                Ok(model.clone()),
                Opponent { profile: *p, seed },
            )),
        }
    }

    fn describe(&self) -> String {
        match self {
            Side::Classic(p) => describe(p),
            Side::Neural(model, p) => {
                let hash: String = model.sha256()[..8]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                format!(
                    "learned model {hash}…, hidden width {}, fallback {}",
                    model.header().shape.hidden,
                    describe(p)
                )
            }
        }
    }
}

fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn describe(p: &Profile) -> String {
    match p.budget {
        None if !p.mistakes.is_none() => format!("rules of thumb with mistakes {:?}", p.mistakes),
        None => "rules of thumb".to_string(),
        Some(b) => format!(
            "{} samples, {} nodes/task, temperature {}, forget {}, {}",
            b.samples,
            b.nodes_per_task,
            p.temperature,
            p.forget,
            match (p.search_bidding, p.search_declaring) {
                (true, _) => "analysis in every phase",
                (false, true) => "analysis in every phase but bidding",
                (false, false) => "analysis in card play and Kontra/Re only",
            }
        ),
    }
}

fn rate(part: u32, all: u32) -> String {
    if all == 0 {
        "–".to_string()
    } else {
        format!("{:.1} %", 100.0 * f64::from(part) / f64::from(all))
    }
}

fn per(points: i64, games: u32) -> String {
    if games == 0 {
        "–".to_string()
    } else {
        format!("{:.1}", points as f64 / f64::from(games))
    }
}

fn row(name: &str, t: &Tally) {
    println!(
        "| {name} | {:.2} | {} | {} | {} | {} | {} | {} |",
        t.mean(),
        t.declarer_games,
        rate(t.declarer_won, t.declarer_games),
        per(t.declarer_points, t.declarer_games),
        t.defender_games,
        rate(t.defender_won, t.defender_games),
        per(t.defender_points, t.defender_games),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let suite = args.first().map_or("tuning", String::as_str);
    let key = match suite {
        "tuning" => TUNING,
        "validation" => VALIDATION,
        other => panic!("unknown suite {other}"),
    };
    let (name_a, name_b) = (
        args.get(1).map_or("medium", String::as_str),
        args.get(2).map_or("strong", String::as_str),
    );
    let (a, b) = (Side::parse(name_a), Side::parse(name_b));
    let deals: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let workers: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let first: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rules = RuleConfig { kontra_re: true };

    let mut rng = ChaCha8Rng::seed_from_u64(key);
    let seeds: Vec<u64> = (0..first + deals)
        .map(|_| rng.next_u64())
        .skip(first)
        .collect();
    let results: Mutex<Vec<Option<DealComparison>>> = Mutex::new(vec![None; deals]);
    let next = AtomicUsize::new(0);
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..workers.max(1) {
            scope.spawn(|| {
                loop {
                    let d = next.fetch_add(1, Ordering::Relaxed);
                    if d >= deals {
                        break;
                    }
                    let ai = |g: usize, s: Seat| {
                        let index = (first + d) as u64;
                        mix(mix(mix(AI_KEY ^ key, index), g as u64), s.index() as u64)
                    };
                    let c = compare_with(
                        seeds[d],
                        rules,
                        |g, s| a.seat(ai(g, s)),
                        |g, s| b.seat(ai(g, s)),
                    );
                    eprintln!(
                        "deal {:4} seed {:20}: {:+7.2} ({:.0} s)",
                        first + d,
                        seeds[d],
                        c.difference(),
                        start.elapsed().as_secs_f64()
                    );
                    results.lock().unwrap()[d] = Some(c);
                }
            });
        }
    });
    let results: Vec<DealComparison> = results
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|r| r.expect("every deal played"))
        .collect();
    let diffs: Vec<f64> = results.iter().map(DealComparison::difference).collect();
    let interval = Interval::of(&diffs);
    let (mut ta, mut tb) = (Tally::default(), Tally::default());
    let mut thrown_in = 0;
    for r in &results {
        ta.merge(&r.a);
        tb.merge(&r.b);
        thrown_in += r.thrown_in;
    }
    let verdict = if interval.low() > 0.0 {
        "A stronger"
    } else if interval.high() < 0.0 {
        "B stronger"
    } else {
        "inconclusive"
    };
    println!(
        "Suite `{suite}`, deals {first}–{}, {deals} deals × 6 games, Kontra/Re on",
        first + deals - 1
    );
    println!();
    println!("- A = `{name_a}`: {}", a.describe());
    println!("- B = `{name_b}`: {}", b.describe());
    println!(
        "- A − B: {:+.2} points per seat-game, standard error {:.2}, 95 % interval [{:+.2}, {:+.2}]: {verdict}",
        interval.mean,
        interval.std_error,
        interval.low(),
        interval.high()
    );
    println!(
        "- thrown in: {thrown_in} of {} games; wall time {:.0} s",
        6 * deals,
        start.elapsed().as_secs_f64()
    );
    println!();
    println!(
        "| Profile | Points/seat-game | Declarer games | Won | Points/declarer game | Defender games | Won | Points/defender game |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    row(&format!("A `{name_a}`"), &ta);
    row(&format!("B `{name_b}`"), &tb);
}
