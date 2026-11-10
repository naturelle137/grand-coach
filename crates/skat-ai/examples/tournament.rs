// SPDX-License-Identifier: EUPL-1.2

//! Role tournament: many players in fixed declarer and defender
//! constellations on the same seeded deals (`docs/ai.md`).
//!
//! ```sh
//! cargo run --locked --release -p skat-ai --example tournament -- \
//!     CONFIG OUT.tsv tuning 2000 20 0
//! ```
//!
//! Arguments: a config file, the output file (tab-separated, appended;
//! games already in it are skipped, so an interrupted run resumes), suite
//! (`tuning` or `validation`), number of deals, parallel games (default 4)
//! and index of the first deal (default 0). Config lines:
//!
//! ```text
//! player NAME SPEC      # SPEC: rookie, easy, medium, strong, goldfish or nn:<path>[@<level>]
//! match DECLARER DEF DEF
//! ```
//!
//! A SPEC ending in `+rk` says Kontra and Re by the rules of thumb (Kontra
//! with five trumps, `skat_analysis::policy`); the player still chooses its
//! card when it does not say Kontra.
//!
//! Every Classic level and the fallback of a learned model bid and declare
//! by the same rules of thumb, so the declarer seat and the contract of a
//! deal do not depend on who sits where (Rookie and analysed pre-play
//! excepted). The runner first plays each deal with three Easy players to
//! find them, skips thrown-in deals, then plays every match once per deal
//! with the match's declarer on the declarer seat. For two different
//! defenders the first sits after the declarer on even deals and the second
//! on odd deals. A game whose declarer or contract differs from the Easy
//! game is written with `mismatch` 1. Each seat's seed depends on suite,
//! deal and seat only, so all matches of a deal share random numbers.
//!
//! One row per game: deal, match index, declarer seat, contract, Kontra
//! level, won, Seeger-Fabian points of declarer, defender after the
//! declarer and the other defender, the two defenders' names, mismatch.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_ai::arena::play_with;
use skat_ai::{
    BaselineOpponent, Cancel, Decision, Level, NeuralOpponent, Opponent, Player, Profile,
};
use skat_analysis::policy;
use skat_core::{Action, GameType, PlayerView, RuleConfig, Seat};
use skat_nn::{Choice, NeuralModel};

/// Keys as in `calibrate`, so suites and seeds mean the same deals.
const TUNING: u64 = 0x7475_6e69_6e67;
const VALIDATION: u64 = 0x7661_6c69_6461;
const AI_KEY: u64 = 0x6f70_706f_6e65;

fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn level(spec: &str) -> Profile {
    match spec {
        "rookie" => Level::Rookie.profile(),
        "easy" => Level::Easy.profile(),
        "goldfish" => Profile::GOLDFISH,
        "medium" => Level::Medium.profile(),
        "strong" => Level::Strong.profile(),
        other => panic!("unknown level {other}"),
    }
}

/// A Classic profile, or a learned model with a Classic fallback.
enum Engine {
    Classic(Profile),
    Neural(Arc<NeuralModel>, Profile, Choice),
}

/// An engine, optionally with Kontra and Re by the rules of thumb.
struct Side {
    engine: Engine,
    rule_kontra: bool,
}

enum Seated {
    Classic(Opponent),
    Neural(NeuralOpponent),
}

impl Seated {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        match self {
            Seated::Classic(o) => Player::decide(o, view, cancel),
            Seated::Neural(o) => Player::decide(o, view, cancel),
        }
    }
}

/// A seated engine; with `rule_kontra` the rules of thumb say Kontra and
/// Re, and the engine chooses among the other legal actions.
struct Contestant {
    seated: Seated,
    rule_kontra: bool,
}

impl Player for Contestant {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        let legal = &view.legal_actions;
        let kontra_choice = |a: &Action| {
            matches!(
                a,
                Action::Kontra | Action::KeinKontra | Action::Re | Action::KeinRe
            )
        };
        if !self.rule_kontra || !legal.iter().any(kontra_choice) {
            return self.seated.decide(view, cancel);
        }
        let rules = |action| {
            Some(Decision {
                action,
                status: None,
                fallback: false,
            })
        };
        // Re answer and the Ouvert Kontra round offer only Kontra/Re actions.
        if legal.iter().all(kontra_choice) {
            return rules(BaselineOpponent.decide(view)?);
        }
        let game = view.contract.map_or(GameType::Grand, |c| c.game);
        if policy::wants_kontra(game, view.hand) {
            return rules(Action::Kontra);
        }
        let mut cards = view.clone();
        cards.legal_actions.retain(|a| !kontra_choice(a));
        self.seated.decide(&cards, cancel)
    }
}

impl Side {
    fn parse(spec: &str) -> Side {
        let (spec, rule_kontra) = match spec.strip_suffix("+rk") {
            Some(rest) => (rest, true),
            None => (spec, false),
        };
        Side {
            engine: Engine::parse(spec),
            rule_kontra,
        }
    }

    fn seat(&self, seed: u64) -> Contestant {
        Contestant {
            seated: self.engine.seat(seed),
            rule_kontra: self.rule_kontra,
        }
    }
}

impl Engine {
    fn parse(spec: &str) -> Engine {
        let (rest, choice) = if let Some(rest) = spec.strip_prefix("nnq:") {
            (rest, Choice::Score)
        } else if let Some(rest) = spec.strip_prefix("nn:") {
            (rest, Choice::Policy)
        } else {
            return Engine::Classic(level(spec));
        };
        let (path, fallback) = rest.rsplit_once('@').unwrap_or((rest, "easy"));
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let hash = skat_nn::sha256(&bytes);
        let model = NeuralModel::load(&bytes, &hash).unwrap_or_else(|e| panic!("{path}: {e:?}"));
        Engine::Neural(Arc::new(model), level(fallback), choice)
    }

    fn seat(&self, seed: u64) -> Seated {
        match self {
            Engine::Classic(p) => Seated::Classic(Opponent { profile: *p, seed }),
            Engine::Neural(model, p, choice) => Seated::Neural(
                NeuralOpponent::new(Ok(model.clone()), Opponent { profile: *p, seed })
                    .with_choice(*choice),
            ),
        }
    }
}

/// Declarer, defender after the declarer and the other defender, by
/// player index.
type Match = [usize; 3];

fn read_config(path: &str) -> (Vec<String>, Vec<Side>, Vec<Match>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let (mut names, mut sides, mut matches) = (Vec::new(), Vec::new(), Vec::new());
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            [] => {}
            ["player", name, spec] => {
                names.push(name.to_string());
                sides.push(Side::parse(spec));
            }
            ["match", d, p, q] => {
                let index = |n: &str| {
                    names
                        .iter()
                        .position(|x| x == n)
                        .unwrap_or_else(|| panic!("unknown player {n}"))
                };
                matches.push([index(d), index(p), index(q)]);
            }
            _ => panic!("bad config line: {line}"),
        }
    }
    (names, sides, matches)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (config, out) = (
        args.first().expect("config file"),
        args.get(1).expect("output file"),
    );
    let key = match args.get(2).map_or("tuning", String::as_str) {
        "tuning" => TUNING,
        "validation" => VALIDATION,
        other => panic!("unknown suite {other}"),
    };
    let deals: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let workers: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let first: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rules = RuleConfig { kontra_re: true };
    let (names, sides, matches) = read_config(config);

    let mut rng = ChaCha8Rng::seed_from_u64(key);
    let seeds: Vec<u64> = (0..first + deals)
        .map(|_| rng.next_u64())
        .skip(first)
        .collect();
    let ai = |d: usize, s: Seat| mix(mix(AI_KEY ^ key, (first + d) as u64), s.index() as u64);

    // Declarer seat and contract of every deal under the rules of thumb.
    let easy = Side {
        engine: Engine::Classic(Level::Easy.profile()),
        rule_kontra: false,
    };
    let reference: Vec<Option<(Seat, String)>> = (0..deals)
        .map(|d| {
            let players = Seat::ALL.map(|s| easy.seat(ai(d, s)));
            let game = play_with(seeds[d], rules, [&players[0], &players[1], &players[2]]);
            game.result()
                .map(|r| (r.outcome.declarer, format!("{:?}", r.outcome.contract)))
        })
        .collect();

    let done: HashSet<(usize, usize)> = std::fs::read_to_string(out)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.parse().ok()?, f.next()?.parse().ok()?))
        })
        .collect();
    let units: Vec<(usize, usize)> = (0..deals)
        .filter(|&d| reference[d].is_some())
        .flat_map(|d| (0..matches.len()).map(move |m| (d, m)))
        .filter(|&(d, m)| !done.contains(&(first + d, m)))
        .collect();
    eprintln!(
        "{} players, {} matches, {} of {deals} deals played, {} games to play ({} done)",
        names.len(),
        matches.len(),
        reference.iter().filter(|r| r.is_some()).count(),
        units.len(),
        done.len()
    );

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out)
        .unwrap_or_else(|e| panic!("{out}: {e}"));
    let file = Mutex::new(file);
    let next = AtomicUsize::new(0);
    let start = Instant::now();
    let progress: Mutex<HashMap<usize, usize>> = Mutex::new(HashMap::new());
    std::thread::scope(|scope| {
        for _ in 0..workers.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(d, m)) = units.get(i) else {
                        break;
                    };
                    let (declarer, contract) = reference[d].as_ref().expect("played deal");
                    let [x, p, q] = matches[m];
                    let (p, q) = if (first + d).is_multiple_of(2) {
                        (p, q)
                    } else {
                        (q, p)
                    };
                    let (s1, s2) = (declarer.next(), declarer.next().next());
                    let mut who = [x; 3];
                    who[s1.index()] = p;
                    who[s2.index()] = q;
                    let players = Seat::ALL.map(|s| sides[who[s.index()]].seat(ai(d, s)));
                    let game = play_with(seeds[d], rules, [&players[0], &players[1], &players[2]]);
                    let r = game.result().expect("the reference game was played");
                    let mismatch = r.outcome.declarer != *declarer
                        || format!("{:?}", r.outcome.contract) != *contract;
                    let sf = r.seeger_fabian();
                    let line = format!(
                        "{}\t{m}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                        first + d,
                        declarer.index(),
                        contract.replace(['\t', '\n'], " "),
                        r.outcome.kontra,
                        u8::from(r.won),
                        sf[declarer.index()],
                        sf[s1.index()],
                        sf[s2.index()],
                        names[p],
                        names[q],
                        u8::from(mismatch),
                    );
                    file.lock()
                        .unwrap()
                        .write_all(line.as_bytes())
                        .expect("write result");
                    let mut progress = progress.lock().unwrap();
                    let count = progress.entry(d).or_insert(0);
                    *count += 1;
                    if *count == matches.len() {
                        eprintln!(
                            "deal {:5} done ({} of {} games, {:.0} s)",
                            first + d,
                            i + 1,
                            units.len(),
                            start.elapsed().as_secs_f64()
                        );
                    }
                }
            });
        }
    });
    eprintln!("finished in {:.0} s", start.elapsed().as_secs_f64());
}
