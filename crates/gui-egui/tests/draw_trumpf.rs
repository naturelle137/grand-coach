// SPDX-License-Identifier: EUPL-1.2

//! #175 ordinary widget stages (seed 1) and every legal move with return.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
#[path = "../examples/support/draw_trumpf_widgets.rs"]
mod views;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

fn build(size: [f32; 2]) -> Harness<'static, GrandCoachApp> {
    Harness::builder().with_size(size).build_eframe(|cc| {
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
    })
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn draw_trumpf_every_ordinary_stage() {
    for name in views::names() {
        views::render(&name, None);
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn draw_trumpf_ordinary_widgets_every_legal_move_and_immediate_return() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            for n in [8, 14, 15, 22, 23, 25, 26, 28, 29, 31] {
                let i = I18n::new(language);
                let mut h = build(size);
                guided_support::draw_trumpf_flow(&mut h, language, "complete", n);
                guided_support::null_click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
                let mut h = build(size);
                guided_support::draw_trumpf_flow(&mut h, language, "reveal", n);
                assert!(h.state().session().guided().unwrap().rule.is_some());
                let rec = h.state().session().export_current_recording_privileged();
                guided_support::null_click(&mut h, &i.t("guided-return"));
                assert_eq!(
                    h.state().session().export_current_recording_privileged(),
                    rec
                );
                guided_support::null_click(
                    &mut h,
                    &guided_support::card_name(&i, Card::from_index(n).unwrap()),
                );
            }
        }
    }
}
