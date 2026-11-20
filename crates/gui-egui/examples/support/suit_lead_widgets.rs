// SPDX-License-Identifier: EUPL-1.2

//! #182 source-owned screenshot inventory: ordinary Farbspiel LeadMaster route (seed 2969).
use super::guided_support as support;
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, guided, i18n::I18n};
use skat_app::*;

pub const STAGES: [&str; 8] = [
    "orient",
    "wrong",
    "master",
    "candidate",
    "reveal",
    "waiting",
    "observe",
    "complete",
];

pub fn inventory() -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    for language in ["de", "en"] {
        for (suffix, width, height) in [("", 1440, 1000), ("-narrow", 960, 640)] {
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-god-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"god-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-ouvert-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"ouvert-fixture"}));
            for stage in STAGES {
                result.push(serde_json::json!({"name":format!("guided-suit-lead-master-{stage}-{language}{suffix}"),"stage":stage,"language":language,"width":width,"height":height,"kind":"ordinary"}));
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
    if entry["kind"] == "god-fixture" || entry["kind"] == "ouvert-fixture" {
        let builder = Harness::builder().with_size(size);
        let builder = if out.is_some() {
            builder.wgpu()
        } else {
            builder
        };

        let ouvert = entry["kind"] == "ouvert-fixture";
        let fixture: serde_json::Value = serde_json::from_str(if ouvert {
            include_str!("../../tests/fixtures/suit-lead-master-ouvert.json")
        } else {
            include_str!("../../tests/fixtures/suit-lead-master-god.json")
        })
        .unwrap();
        let table: TableView = serde_json::from_value(fixture["table"].clone()).unwrap();
        let dialog: GuidedView = serde_json::from_value(fixture["guided"].clone()).unwrap();
        let i = I18n::new(language);
        let mut h = builder.build_ui(move |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.label(
                    i.tf(
                        "guided-lead-fixture-label",
                        &[
                            (
                                "case",
                                if ouvert {
                                    "Ouvert / open display"
                                } else {
                                    "God Mode"
                                }
                                .into(),
                            ),
                            ("stage", "reveal".into()),
                        ],
                    ),
                );
                let _ = guided::panel(ui, &i, &table, &dialog);
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
        return;
    }
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
                seed: 2969,
                series: SeriesLength::Fixed(1),
                opponents: Opponents::Levels([Level::Easy; 2]),
                kontra_re: false,
                display: TableDisplay::Closed,
                ..SessionConfig::default()
            },
        )
    });
    support::suit_lead_flow(&mut h, language, entry["stage"].as_str().unwrap(), 31);
    h.run_steps(4);
    if let Some(out) = out {
        h.render()
            .unwrap()
            .save(format!("{out}/{name}.png"))
            .unwrap();
    }
}
