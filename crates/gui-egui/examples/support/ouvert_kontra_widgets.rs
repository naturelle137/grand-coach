// SPDX-License-Identifier: EUPL-1.2

//! The core-validated Ouvert Kontra decision, captured from the app test.
use eframe::egui;
use egui_kittest::Harness;
use gui_egui::{Language, guided, i18n::I18n};
use skat_app::{GuidedView, TableView};

pub fn names() -> Vec<String> {
    ["guided-ouvert-kontra-de", "guided-ouvert-kontra-en"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub fn render(name: &str, out: &str) {
    let language = if name.ends_with("-de") {
        Language::De
    } else {
        Language::En
    };
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ouvert-kontra-round.json"
    ))
    .unwrap();
    let table: TableView = serde_json::from_value(fixture["table"].clone()).unwrap();
    let dialog: GuidedView = serde_json::from_value(fixture["guided"].clone()).unwrap();
    let i = I18n::new(language);
    let mut h = Harness::builder()
        .with_size([1280.0, 820.0])
        .wgpu()
        .build_ui(move |root| {
            egui::CentralPanel::default().show(root, |ui| {
                let _ = guided::panel(ui, &i, &table, &dialog);
            });
        });
    h.run_steps(8);
    h.render()
        .unwrap()
        .save(format!("{out}/{name}.png"))
        .unwrap();
}
