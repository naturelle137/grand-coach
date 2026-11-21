// SPDX-License-Identifier: EUPL-1.2

//! Explicit fixture component models emitted by the actual Session tests.
//! Never ordinary Session::new / GrandCoachApp::new route coverage.
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{Language, guided, i18n::I18n};
use skat_app::{GuidedView, TableView};

pub fn names() -> Vec<String> {
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/null-last.json")).unwrap();
    frames
        .iter()
        .filter(|f| {
            matches!(
                (f["case"].as_str().unwrap(), f["stage"].as_str().unwrap()),
                ("mixed-ten", "rule" | "observe" | "complete")
                    | ("real-ouvert", "rule" | "observe" | "complete")
                    | ("void-last-tie", "rule" | "complete")
                    | ("pickup-mixed", "memory")
                    | ("void-changed-queen" | "printed-bube", "rule")
            )
        })
        .flat_map(|f| {
            Language::ALL.into_iter().flat_map(move |language| {
                let base = format!(
                    "guided-null-fixture-{}-{}-{}",
                    f["case"].as_str().unwrap(),
                    f["stage"].as_str().unwrap(),
                    if language == Language::De { "de" } else { "en" }
                );
                [base.clone(), format!("{base}-narrow")]
            })
        })
        .collect()
}

pub fn render(name: &str, out: Option<&str>) {
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/null-last.json")).unwrap();
    let f = frames
        .iter()
        .find(|f| {
            name.starts_with(&format!(
                "guided-null-fixture-{}-{}-",
                f["case"].as_str().unwrap(),
                f["stage"].as_str().unwrap()
            ))
        })
        .unwrap();
    let table: TableView = serde_json::from_value(f["table"].clone()).unwrap();
    let g: GuidedView = serde_json::from_value(f["guided"].clone()).unwrap();
    let i = I18n::new(if name.ends_with("-en") || name.ends_with("-en-narrow") {
        Language::En
    } else {
        Language::De
    });
    let expected = if let Some(o) = &g.observation {
        let winner = o.winner.expect("last-seat fixture has an actual winner");
        i.tf(
            "guided-winner",
            &[(
                "seat",
                i.t(match winner {
                    skat_app::Seat::Vorhand => "seat-vorhand",
                    skat_app::Seat::Mittelhand => "seat-mittelhand",
                    skat_app::Seat::Hinterhand => "seat-hinterhand",
                })
                .into(),
            )],
        )
    } else if g.step == skat_app::GuidedStep::Observe {
        i.t("guided-observe")
    } else {
        assert_eq!(
            g.rule.as_ref().unwrap().branch,
            skat_app::GuidedBranch::Null
        );
        i.t("guided-rule-null")
    };
    let size = if name.ends_with("-narrow") {
        [960.0, 640.0]
    } else {
        [1440.0, 1000.0]
    };
    let draw = move |root: &mut egui::Ui| {
        egui::CentralPanel::default().show(root, |ui| {
            ui.label(format!(
                "Fixture component: {} / {} (not an ordinary route)",
                f["case"].as_str().unwrap(),
                f["stage"].as_str().unwrap()
            ));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let _ = guided::panel(ui, &i, &table, &g);
            });
        });
    };
    // Component assertions need a renderer only when saving a PNG.
    let builder = Harness::builder().with_size(size);
    let builder = if out.is_some() {
        builder.wgpu()
    } else {
        builder
    };
    let mut h = builder.build_ui(draw);
    h.run_steps(8);
    h.get_by_label(&expected);
    h.get_by_label(
        &I18n::new(if name.contains("-en") {
            Language::En
        } else {
            Language::De
        })
        .t("guided-title"),
    );
    if let Some(out) = out {
        h.render()
            .unwrap()
            .save(format!("{out}/{name}.png"))
            .unwrap();
    }
}
