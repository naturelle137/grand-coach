// SPDX-License-Identifier: EUPL-1.2

//! Decision latency of every strength level, the Goldfish and Mephisto per
//! decision phase, on the benchmark positions of `skat-analysis`
//! (`docs/analysis.md`): seeded
//! games 1, 2, … with Kontra/Re in which the rules of thumb decide before
//! card play and the first legal card is played; the first positions of
//! each kind.
//!
//! ```sh
//! cargo run --locked --release -p skat-ai --example latency
//! ```
//!
//! Optional arguments: positions per kind (default 10), then a learned
//! model `nn:<path>` (D-07) measured as the row "Neural": the model decides
//! the phases it covers, the Classic level Strong every other one (its
//! rows then measure that fallback).

use std::sync::Arc;
use std::time::{Duration, Instant};

use skat_ai::{BaselineOpponent, Level, Mephisto, NeuralOpponent, Opponent, Profile, Status};
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig};
use skat_nn::NeuralModel;

enum Measured {
    Classic(Opponent),
    Neural(NeuralOpponent),
    Mephisto,
}

const KINDS: [&str; 8] = [
    "Reizen",
    "Hand/pickup",
    "Drücken",
    "Announcement",
    "Kontra",
    "Card play, trick 1",
    "Card play, trick 4",
    "Card play, trick 8",
];

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

/// Advances a benchmark game: rules of thumb, first card, no Kontra.
fn advance(view: &PlayerView) -> Action {
    match view.phase {
        Phase::Spiel => view
            .legal_actions
            .iter()
            .copied()
            .find(|a| matches!(a, Action::Play(_) | Action::KeinRe | Action::KeinKontra))
            .unwrap_or(view.legal_actions[0]),
        _ => BaselineOpponent.decide(view).expect("legal action"),
    }
}

/// A time in seconds, or in milliseconds below 10 ms (learned models
/// decide in well under a millisecond).
fn show(t: Duration) -> String {
    let secs = t.as_secs_f64();
    if secs < 0.01 {
        format!("{:.3} ms", secs * 1000.0)
    } else {
        format!("{secs:.3} s")
    }
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i]
}

fn main() {
    let per_kind: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10);
    let mut positions: Vec<Vec<Game>> = vec![Vec::new(); KINDS.len()];
    let mut seed = 0;
    while positions.iter().any(|p| p.len() < per_kind) && seed < 10_000 {
        seed += 1;
        let mut game = Game::new(seed, RuleConfig { kontra_re: true });
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if let Some(k) = kind(&view)
                && positions[k].len() < per_kind
                && !(k == 0 && view.bidding.len() > 1)
            {
                positions[k].push(game.clone());
            }
            game.apply(advance(&view)).unwrap();
        }
    }
    println!("{per_kind} positions per kind; release build; times per decision");
    println!();
    println!("| Level | Decision | Median | p90 | Max | Incomplete |");
    println!("|---|---|---|---|---|---|");
    let mut players: Vec<(String, Measured)> = Level::ALL
        .map(|level| {
            (
                format!("{level:?}"),
                Measured::Classic(Opponent::new(level, 1)),
            )
        })
        .into_iter()
        .collect();
    let goldfish = Opponent {
        profile: Profile::GOLDFISH,
        seed: 1,
    };
    players.push(("Goldfish".to_owned(), Measured::Classic(goldfish)));
    players.push(("Mephisto".to_owned(), Measured::Mephisto));
    if let Some(path) = std::env::args().nth(2) {
        let path = path.strip_prefix("nn:").expect("model argument: nn:<path>");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let model = NeuralModel::load(&bytes, &skat_nn::sha256(&bytes))
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
        let neural = NeuralOpponent::new(Ok(Arc::new(model)), Opponent::new(Level::Strong, 1));
        players.push(("Neural".to_owned(), Measured::Neural(neural)));
    }
    for (name, opponent) in &players {
        for (k, games) in positions.iter().enumerate() {
            let mut times = Vec::new();
            let mut incomplete = 0;
            for game in games {
                let view = game.view(game.current_seat().expect("a decision"));
                let start = Instant::now();
                let d = match opponent {
                    Measured::Classic(o) => o.decide(&view, None),
                    Measured::Neural(o) => o.decide_explained(&view, None).map(|d| d.decision),
                    Measured::Mephisto => Mephisto.decide(game, None),
                }
                .expect("a decision");
                times.push(start.elapsed());
                if d.fallback || d.status.is_some_and(|s| s != Status::Complete) {
                    incomplete += 1;
                }
            }
            times.sort();
            println!(
                "| {name} | {} | {} | {} | {} | {incomplete} |",
                KINDS[k],
                show(percentile(&times, 0.5)),
                show(percentile(&times, 0.9)),
                show(*times.last().unwrap()),
            );
        }
    }
}
