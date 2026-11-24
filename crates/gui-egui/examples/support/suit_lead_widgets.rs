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
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-null-partner-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"null-partner-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-god-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"god-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-ouvert-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"ouvert-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-ouvert-defender-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"suit-ouvert-defender-lead"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-defender-lead-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-defender-lead"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-suit-draw-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"suit-draw-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-draw-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-draw-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-last-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-last-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-feed-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-feed-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-suit-ouvert-feed-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"suit-ouvert-feed-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-suit-ouvert-early-feed-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"suit-ouvert-early-feed-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-early-feed-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-early-feed-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-early-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-early-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-grand-ouvert-defender-early-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"grand-ouvert-defender-early-fixture"}));
            result.push(serde_json::json!({"name":format!("guided-suit-lead-master-suit-ouvert-last-fixture-{language}{suffix}"),"stage":"reveal","language":language,"width":width,"height":height,"kind":"suit-ouvert-last-fixture"}));
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
    if entry["kind"] == "null-partner-fixture"
        || entry["kind"] == "god-fixture"
        || entry["kind"] == "ouvert-fixture"
        || entry["kind"] == "suit-ouvert-defender-lead"
        || entry["kind"] == "grand-ouvert-defender-lead"
        || entry["kind"] == "grand-ouvert-fixture"
        || entry["kind"] == "grand-ouvert-draw-fixture"
        || entry["kind"] == "suit-draw-fixture"
        || entry["kind"] == "grand-ouvert-last-fixture"
        || entry["kind"] == "grand-ouvert-feed-fixture"
        || entry["kind"] == "suit-ouvert-feed-fixture"
        || entry["kind"] == "suit-ouvert-early-feed-fixture"
        || entry["kind"] == "grand-ouvert-early-feed-fixture"
        || entry["kind"] == "grand-ouvert-early-fixture"
        || entry["kind"] == "grand-ouvert-defender-early-fixture"
        || entry["kind"] == "suit-ouvert-last-fixture"
    {
        let builder = Harness::builder().with_size(size);
        let builder = if out.is_some() {
            builder.wgpu()
        } else {
            builder
        };

        let grand_defender_lead = entry["kind"] == "grand-ouvert-defender-lead";
        let suit_defender_lead = entry["kind"] == "suit-ouvert-defender-lead";
        let grand_defender_early = entry["kind"] == "grand-ouvert-defender-early-fixture";
        let grand_early = entry["kind"] == "grand-ouvert-early-fixture";
        let suit_last = entry["kind"] == "suit-ouvert-last-fixture";
        let grand_feed = entry["kind"] == "grand-ouvert-feed-fixture";
        let suit_feed = entry["kind"] == "suit-ouvert-feed-fixture";
        let suit_early_feed = entry["kind"] == "suit-ouvert-early-feed-fixture";
        let grand_early_feed = entry["kind"] == "grand-ouvert-early-feed-fixture";
        let grand_last = entry["kind"] == "grand-ouvert-last-fixture";
        let grand_draw = entry["kind"] == "grand-ouvert-draw-fixture";
        let suit_draw = entry["kind"] == "suit-draw-fixture";
        let grand_ouvert = grand_defender_lead
            || entry["kind"] == "grand-ouvert-fixture"
            || grand_draw
            || grand_last
            || grand_feed
            || grand_early_feed
            || grand_early
            || grand_defender_early;
        let ouvert = suit_defender_lead
            || entry["kind"] == "ouvert-fixture"
            || grand_ouvert
            || suit_last
            || suit_feed
            || suit_early_feed;
        let fixture: serde_json::Value =
            serde_json::from_str(if entry["kind"] == "null-partner-fixture" {
                include_str!("../../tests/fixtures/null-partner-lead.json")
            } else if suit_draw {
                include_str!("../../tests/fixtures/suit-draw-trumpf.json")
            } else if grand_defender_lead {
                include_str!("../../tests/fixtures/grand-ouvert-defender-lead.json")
            } else if suit_defender_lead {
                include_str!("../../tests/fixtures/suit-ouvert-defender-lead.json")
            } else if grand_early_feed {
                include_str!("../../tests/fixtures/grand-ouvert-early-feed.json")
            } else if grand_defender_early {
                include_str!("../../tests/fixtures/grand-ouvert-defender-early.json")
            } else if suit_early_feed {
                include_str!("../../tests/fixtures/suit-ouvert-early-feed.json")
            } else if suit_feed {
                include_str!("../../tests/fixtures/suit-ouvert-feed.json")
            } else if grand_feed {
                include_str!("../../tests/fixtures/grand-ouvert-feed.json")
            } else if grand_early {
                include_str!("../../tests/fixtures/grand-ouvert-early.json")
            } else if suit_last {
                include_str!("../../tests/fixtures/suit-ouvert-last.json")
            } else if grand_last {
                include_str!("../../tests/fixtures/grand-ouvert-last.json")
            } else if grand_draw {
                include_str!("../../tests/fixtures/grand-ouvert-draw.json")
            } else if grand_ouvert {
                include_str!("../../tests/fixtures/grand-lead-master-ouvert.json")
            } else if ouvert {
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
                                if entry["kind"] == "null-partner-fixture" {
                                    "Null / partner lead"
                                } else if ouvert {
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
