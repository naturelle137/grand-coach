// SPDX-License-Identifier: EUPL-1.2

//! #171 source-owned screenshot inventory. Ordinary routes and labelled fixtures.
use super::guided_support as support;
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, guided, i18n::I18n};
use skat_app::*;

pub const STAGES: [&str; 12] = [
    "orient",
    "wrong",
    "master",
    "candidate",
    "attempt",
    "reveal",
    "memory",
    "waiting",
    "observe",
    "complete",
    "return",
    "details",
];

pub fn inventory() -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    for language in ["de", "en"] {
        for (suffix, width, height) in [("", 1440, 1000), ("-narrow", 960, 640)] {
            for stage in STAGES {
                result.push(serde_json::json!({"name":format!("guided-lead-master-{stage}-{language}{suffix}"),"stage":stage,"language":language,"width":width,"height":height,"kind":"ordinary"}));
            }
            let frames: Vec<serde_json::Value> =
                serde_json::from_str(include_str!("../../tests/fixtures/lead-master.json"))
                    .unwrap();
            for f in frames {
                result.push(serde_json::json!({"name":format!("guided-lead-master-fixture-{}-{}-{language}{suffix}",f["case"].as_str().unwrap(),f["stage"].as_str().unwrap()),"stage":f["stage"],"case":f["case"],"language":language,"width":width,"height":height,"kind":"fixture"}));
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
    if entry["kind"] == "ordinary" {
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
                    seed: 3,
                    series: SeriesLength::Fixed(1),
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    kontra_re: false,
                    display: TableDisplay::Closed,
                },
            )
        });
        let stage = STAGES.iter().position(|s| entry["stage"] == *s).unwrap() as u8;
        support::lead_master_flow(&mut h, language, stage, 5);
        h.run_steps(4);
        if let Some(out) = out {
            h.render()
                .unwrap()
                .save(format!("{out}/{name}.png"))
                .unwrap();
        }
    } else {
        let builder = Harness::builder().with_size(size);
        let builder = if out.is_some() {
            builder.wgpu()
        } else {
            builder
        };
        let frames: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/lead-master.json")).unwrap();
        let f = frames
            .into_iter()
            .find(|f| f["case"] == entry["case"] && f["stage"] == entry["stage"])
            .unwrap();
        let table: TableView = serde_json::from_value(f["table"].clone()).unwrap();
        let g: GuidedView = serde_json::from_value(f["guided"].clone()).unwrap();
        let i = I18n::new(language);
        let mut h = builder.build_ui(move |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.label(i.tf(
                    "guided-lead-fixture-label",
                    &[
                        ("case", f["case"].as_str().unwrap().into()),
                        ("stage", f["stage"].as_str().unwrap().into()),
                    ],
                ));
                let _ = guided::panel(ui, &i, &table, &g);
            });
        });
        h.run_steps(8);
        h.get_by_label(&I18n::new(language).t("guided-title"));
        if let Some(out) = out {
            h.render()
                .unwrap()
                .save(format!("{out}/{name}.png"))
                .unwrap();
        }
    }
}
