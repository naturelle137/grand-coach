// SPDX-License-Identifier: EUPL-1.2

//! Reproduces the packaged post-jump historical-floor ambiguity through real widgets.

#[path = "support/qa146.rs"]
mod support;

#[test]
fn post_jump_lesson_labels_the_historical_floor_in_both_languages() {
    let mut h = egui_kittest::Harness::builder()
        .with_size([1440.0, 1000.0])
        .build_eframe(|cc| gui_egui::GrandCoachApp::new(&cc.egui_ctx, support::config()));
    support::jump(&mut h);
    support::assert_copy(&mut h, gui_egui::Language::De);
    support::assert_copy(&mut h, gui_egui::Language::En);
}
