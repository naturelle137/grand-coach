// SPDX-License-Identifier: EUPL-1.2

//! Assess a searched declaration policy on replayable synthetic views.
use nn_train::selfplay::RunSeed;
use skat_ai::{Level, Opponent};
use skat_analysis::Budget;
use skat_core::PlayerView;
use std::io::{self, BufRead};

fn main() {
    let mut last = None;
    for line in io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let deal = value["deal"].as_u64().unwrap();
        if last == Some(deal) {
            continue;
        }
        last = Some(deal);
        let run = RunSeed(value["run_seed"].as_u64().unwrap());
        let view: PlayerView = serde_json::from_value(value["view"].clone()).unwrap();
        let mut profile = Level::Strong.profile();
        profile.search_declaring = true;
        profile.budget = Some(Budget {
            samples: 1,
            nodes_per_task: 10_000_000,
        });
        let player = Opponent {
            profile,
            seed: run.teacher_seed(deal, view.seat),
        };
        let decision = player.decide(&view, None).unwrap();
        assert!(view.legal_actions.contains(&decision.action));
        println!(
            "{}",
            serde_json::json!({"deal": deal, "split": value["split"], "action": decision.action, "status": decision.status, "fallback": decision.fallback})
        );
    }
}
