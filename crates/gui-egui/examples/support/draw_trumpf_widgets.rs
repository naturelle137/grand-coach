// SPDX-License-Identifier: EUPL-1.2

//! #175 source-owned screenshot inventory: ordinary seed-1 routes only. The
//! labelled DrawTrumpf boundary fixture frames live in the lead-master family
//! (`lead-master.json`, case 13).
use super::guided_support as support;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language};
use skat_app::*;

pub const STAGES: [&str; 10] = [
    "orient",
    "wrong",
    "candidate",
    "attempt",
    "reveal",
    "memory",
    "details",
    "waiting",
    "observe",
    "complete",
];

pub fn inventory() -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    for language in ["de", "en"] {
        for (suffix, width, height) in [("", 1440, 1000), ("-narrow", 960, 640)] {
            for stage in STAGES {
                result.push(serde_json::json!({"name":format!("guided-draw-trumpf-{stage}-{language}{suffix}"),"stage":stage,"language":language,"width":width,"height":height,"kind":"ordinary"}));
            }
        }
    }
    result
}
pub fn names() -> Vec<String> {
    inventory()
        .iter()
        .map(|v| v["name"].as_str().unwrap().to_owned())
        .collect()
}

pub fn render(name: &str, out: Option<&str>) {
    let entry = inventory().into_iter().find(|v| v["name"] == name).unwrap();
    let language = if entry["language"] == "de" {
        Language::De
    } else {
        Language::En
    };
    let size = [
        entry["width"].as_f64().unwrap() as f32,
        entry["height"].as_f64().unwrap() as f32,
    ];
    let builder = Harness::builder().with_size(size);
    let builder = if out.is_some() {
        builder.wgpu()
    } else {
        builder
    };
    let mut h = builder.build_eframe(|cc| {
        GrandCoachApp::new(
            &cc.egui_ctx,
            SessionConfig {
                seed: 1,
                series: SeriesLength::Free,
                opponents: Opponents::Levels([Level::Easy; 2]),
                kontra_re: false,
                display: TableDisplay::Closed,
                ..SessionConfig::default()
            },
        )
    });
    support::draw_trumpf_flow(&mut h, language, entry["stage"].as_str().unwrap(), 28);
    h.run_steps(4);
    if let Some(out) = out {
        h.render()
            .unwrap()
            .save(format!("{out}/{name}.png"))
            .unwrap();
    }
}
