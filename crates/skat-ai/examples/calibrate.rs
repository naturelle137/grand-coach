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
//! play and Re only; `d`: also Hand, Drücken and announcement), or a
//! learned model `nn:<path>[@<level>]` (D-07): the model file at `<path>`
//! decides and the Classic `<level>` (default `easy`) takes every fallback;
//! `^<bits>` after the fallback restricts the model to phases (bit 0 Reizen,
//! 1 Hand/pickup, 2 Drücken, 3 Ansage, 4 Spiel; e.g. `nn:m.gcnn@strong^23`
//! leaves the Ansage to Classic); the others use the Classic fallback.
//! `nnq:` instead of `nn:` picks the action with the highest expected
//! points of the model's option heads instead of the policy. A trailing
//! `+ak` lets the analysis or the model decide Kontra instead of the rules
//! of thumb (`Profile::search_kontra`). Kontra/Re is on.
//! `nn-easy:<path>`, `nn-medium:<path>` and `nn-strong:<path>` use the
//! shared experimental app temperature profiles and matching Classic fallback.
//! `two-stage` uses the opt-in two-stage Strong proposal.
//! `hybrid:<path>` uses experimental learned proposals verified by Strong,
//! with fixed Strong fallback; it accepts no level or `+ak` suffix.
//!
//! Prints a Markdown summary: the mean per-deal difference of Seeger-Fabian
//! points per seat-game (A minus B) with its standard error and 95 %
//! interval, plus declarer/defender results, bidding action rates,
//! Einpassen participation and declarer contract mix per profile.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_ai::arena::{DealComparison, Interval, Tally, compare_with};
use skat_ai::{
    Budget, Cancel, Decision, HybridOpponent, Level, NeuralOpponent, Opponent, Player, Profile,
};
use skat_core::{PlayerView, RuleConfig, Seat};
use skat_nn::{Choice, NeuralModel, Phases, PolicyLevel};

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
    TwoStage,
    Experimental,
    Classic(Profile),
    Neural(Arc<NeuralModel>, Profile, Choice, Phases),
    Hybrid(Arc<NeuralModel>),
}

enum Seated {
    TwoStage(Opponent),
    Experimental(skat_ai::ExperimentalOpponent),
    Classic(Opponent),
    Neural(NeuralOpponent),
    Hybrid(HybridOpponent),
}

impl Player for Seated {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        match self {
            Seated::TwoStage(o) => o
                .decide_two_stage(view, cancel, skat_analysis::TwoStage::PROPOSAL)
                .map(|(decision, _, _)| decision),
            Seated::Hybrid(o) => Player::decide(o, view, cancel),
            Seated::Experimental(o) => o.decide(view, cancel),
            Seated::Classic(o) => Player::decide(o, view, cancel),
            Seated::Neural(o) => Player::decide(o, view, cancel),
        }
    }
}

impl Side {
    fn parse(spec: &str) -> Side {
        if let Some(path) = spec.strip_prefix("hybrid:") {
            let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
            let model = NeuralModel::load(&bytes, &skat_nn::sha256(&bytes))
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            return Side::Hybrid(Arc::new(model));
        }
        if spec == "two-stage" {
            return Side::TwoStage;
        }
        if spec == "experimental" {
            return Side::Experimental;
        }
        let (spec, search_kontra) = match spec.strip_suffix("+ak") {
            Some(rest) => (rest, true),
            None => (spec, false),
        };
        let profile = |spec: &str| Profile {
            search_kontra,
            ..profile(spec)
        };
        let learned = ["easy", "medium", "strong"].into_iter().find_map(|name| {
            spec.strip_prefix(&format!("nn-{name}:"))
                .map(|path| (path, name))
        });
        if let Some((path, name)) = learned {
            assert!(!search_kontra, "learned levels retain app Kontra rules");
            let level = match name {
                "easy" => PolicyLevel::Easy,
                "medium" => PolicyLevel::Medium,
                _ => PolicyLevel::Strong,
            };
            let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
            let model = NeuralModel::load(&bytes, &skat_nn::sha256(&bytes)).unwrap();
            return Side::Neural(
                Arc::new(model),
                profile(name),
                Choice::Level(level),
                Phases::ALL,
            );
        }
        let (rest, choice) = if let Some(rest) = spec.strip_prefix("nnq:") {
            (rest, Choice::Score)
        } else if let Some(rest) = spec.strip_prefix("nn:") {
            (rest, Choice::Policy)
        } else {
            return Side::Classic(profile(spec));
        };
        let (rest, learned) = match rest.rsplit_once('^') {
            Some((rest, bits)) => {
                let bits = bits.parse().unwrap_or_else(|_| panic!("phase bits {bits}"));
                (rest, Phases::from_bits(bits).expect("phase bits 1..=31"))
            }
            None => (rest, Phases::ALL),
        };
        let (path, fallback) = rest.rsplit_once('@').unwrap_or((rest, "easy"));
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let hash = skat_nn::sha256(&bytes);
        let model = NeuralModel::load(&bytes, &hash).unwrap_or_else(|e| panic!("{path}: {e:?}"));
        Side::Neural(Arc::new(model), profile(fallback), choice, learned)
    }

    fn seat(&self, seed: u64) -> Seated {
        match self {
            Side::TwoStage => Seated::TwoStage(Opponent::new(Level::Strong, seed)),
            Side::Hybrid(model) => Seated::Hybrid(HybridOpponent::new(Ok(model.clone()), seed)),
            Side::Experimental => Seated::Experimental(skat_ai::ExperimentalOpponent {
                seed,
                samples: skat_ai::ExperimentalOpponent::SAMPLES,
            }),
            Side::Classic(p) => Seated::Classic(Opponent { profile: *p, seed }),
            Side::Neural(model, p, choice, learned) => Seated::Neural(
                NeuralOpponent::new(Ok(model.clone()), Opponent { profile: *p, seed })
                    .with_choice(*choice)
                    .with_learned_phases(*learned),
            ),
        }
    }

    fn describe(&self) -> String {
        match self {
            Side::TwoStage => "experimental two-stage Strong proposal".into(),
            Side::Hybrid(model) => {
                let hash: String = model.sha256().iter().map(|b| format!("{b:02x}")).collect();
                format!(
                    "experimental hybrid proposals top-2 + rule, model {hash}, encoder {}, Strong budget/fallback",
                    model.header().encoder_version
                )
            }
            Side::Experimental => format!(
                "experimental, {} samples",
                skat_ai::ExperimentalOpponent::SAMPLES
            ),
            Side::Classic(p) => describe(p),
            Side::Neural(model, p, choice, learned) => {
                let hash: String = model.sha256().iter().map(|b| format!("{b:02x}")).collect();
                let policy = match choice {
                    Choice::Level(level) => format!(
                        "{}, temperature {}",
                        PolicyLevel::REVISION,
                        level.temperature()
                    ),
                    _ => "greedy-policy-1".into(),
                };
                let phases = if *learned == Phases::ALL {
                    String::new()
                } else {
                    format!(", learned phase bits {}", learned.bits())
                };
                format!(
                    "learned model {hash}, encoder {}, {policy}, {} hidden layers of width {}, choice {choice:?}{phases}, fallback {}",
                    model.header().encoder_version,
                    model.header().shape.layers,
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
        Some(b) => {
            format!(
                "{} samples, {} nodes/task, temperature {}, forget {}, {}",
                b.samples,
                b.nodes_per_task,
                p.temperature,
                p.forget,
                match (p.search_bidding, p.search_declaring) {
                    (true, _) => "analysis in every phase",
                    (false, true) => "analysis in every phase but bidding",
                    (false, false) => "analysis in card play and Re only",
                },
            ) + if p.search_kontra {
                ", analysed Kontra"
            } else {
                ""
            }
        }
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

fn behaviour(name: &str, t: &Tally) -> String {
    let actions = t.bidding_actions();
    format!(
        "| {name} | {actions} | {} ({}) | {} ({}) | {} ({}) | {} / {} ({}) |",
        t.bids,
        rate(t.bids, actions),
        t.holds,
        rate(t.holds, actions),
        t.passes,
        rate(t.passes, actions),
        t.thrown_in(),
        t.seat_games,
        rate(t.thrown_in(), t.seat_games),
    )
}

fn contracts(name: &str, t: &Tally) -> String {
    let counts = t.contracts.into_iter().chain([
        t.hand,
        t.schneider_announced,
        t.schwarz_announced,
        t.ouvert,
    ]);
    let cells: Vec<_> = counts
        .map(|n| format!("{n} ({})", rate(n, t.declarer_games)))
        .collect();
    format!("| {name} | {} | {} |", t.declarer_games, cells.join(" | "))
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
    println!();
    println!(
        "Bidding rates use all Reizen actions (Bid + Hold + Pass). Einpassen uses seat-games; mixed-table participation does not assign causation."
    );
    println!();
    println!("| Profile | Reizen actions | Bid | Hold | Pass | Einpassen seat-games |");
    println!("|---|---|---|---|---|---|");
    println!("{}", behaviour(&format!("A `{name_a}`"), &ta));
    println!("{}", behaviour(&format!("B `{name_b}`"), &tb));
    println!();
    println!(
        "Contract rates use declarer games only. Hand and announcements overlap the six contract bins; Schneider/Schwarz mean announced, not achieved."
    );
    println!();
    println!(
        "| Profile | Declarer games | Kreuz | Pik | Herz | Karo | Grand | Null | Hand | Schneider announced | Schwarz announced | Ouvert |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    println!("{}", contracts(&format!("A `{name_a}`"), &ta));
    println!("{}", contracts(&format!("B `{name_b}`"), &tb));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavioural_rows_show_counts_denominators_and_absent_contracts() {
        let passive = Tally {
            seat_games: 9,
            passes: 9,
            ..Tally::default()
        };
        assert_eq!(
            behaviour("passive", &passive),
            "| passive | 9 | 0 (0.0 %) | 0 (0.0 %) | 9 (100.0 %) | 9 / 9 (100.0 %) |"
        );
        assert_eq!(
            contracts("passive", &passive),
            "| passive | 0 | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) | 0 (–) |"
        );
        let active = Tally {
            seat_games: 9,
            declarer_games: 2,
            defender_games: 7,
            bids: 1,
            holds: 1,
            passes: 2,
            contracts: [1, 0, 0, 0, 0, 1],
            hand: 1,
            ouvert: 1,
            ..Tally::default()
        };
        assert_eq!(
            behaviour("active", &active),
            "| active | 4 | 1 (25.0 %) | 1 (25.0 %) | 2 (50.0 %) | 0 / 9 (0.0 %) |"
        );
        assert_eq!(
            contracts("active", &active),
            "| active | 2 | 1 (50.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 1 (50.0 %) | 1 (50.0 %) | 0 (0.0 %) | 0 (0.0 %) | 1 (50.0 %) |"
        );
        assert_eq!(rate(0, 0), "–");
    }

    #[test]
    fn learned_level_specs_reuse_app_profiles_and_report_full_identity() {
        let bytes = skat_nn::fixture_model(22, skat_nn::Phases::SPIEL);
        let path =
            std::env::temp_dir().join(format!("gc-level-calibrate-{}.gcnn", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        for (name, level, fallback) in [
            ("easy", PolicyLevel::Easy, Level::Easy),
            ("medium", PolicyLevel::Medium, Level::Medium),
            ("strong", PolicyLevel::Strong, Level::Strong),
        ] {
            let side = Side::parse(&format!("nn-{name}:{}", path.display()));
            let Seated::Neural(player) = side.seat(123) else {
                panic!("neural seat")
            };
            assert_eq!(player.choice, Choice::Level(level));
            assert_eq!(player.classic.seed, 123);
            assert_eq!(player.classic.profile, fallback.profile());
            let hash: String = skat_nn::sha256(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert!(side.describe().contains(&hash));
            assert!(side.describe().contains(PolicyLevel::REVISION));
            assert!(side.describe().contains("encoder 2"));
        }
        let Seated::Neural(legacy) = Side::parse(&format!("nn:{}@easy", path.display())).seat(123)
        else {
            panic!("legacy")
        };
        assert_eq!(legacy.choice, Choice::Policy);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn hybrid_spec_loads_the_named_model_and_preserves_the_seed() {
        let bytes = skat_nn::fixture_model(19, skat_nn::Phases::SPIEL);
        let path =
            std::env::temp_dir().join(format!("gc-hybrid-calibrate-{}.gcnn", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let side = Side::parse(&format!("hybrid:{}", path.display()));
        std::fs::remove_file(path).unwrap();
        let Side::Hybrid(ref model) = side else {
            panic!("hybrid spec")
        };
        assert_eq!(model.sha256(), skat_nn::sha256(&bytes));
        assert!(side.describe().contains("experimental hybrid"));
        let Seated::Hybrid(player) = side.seat(123) else {
            panic!("hybrid seat")
        };
        assert_eq!(player.seed, 123);
        assert_eq!(player.budget, Budget::INTERACTIVE);
    }
}

#[cfg(test)]
mod two_stage_tests {
    use super::*;
    use skat_ai::BaselineOpponent;
    use skat_core::{Game, Phase};

    #[test]
    fn two_stage_profile_dispatches_the_actual_candidate() {
        let mut game = Game::new(17, RuleConfig { kontra_re: false });
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if view.phase == Phase::Spiel {
                let expected = Opponent::new(Level::Strong, 42)
                    .decide_two_stage(&view, None, skat_analysis::TwoStage::PROPOSAL)
                    .unwrap()
                    .0;
                assert_eq!(
                    Side::parse("two-stage").seat(42).decide(&view, None),
                    Some(expected)
                );
                return;
            }
            game.apply(BaselineOpponent.decide(&view).unwrap()).unwrap();
        }
        panic!("fixture must reach card play");
    }
}
