// SPDX-License-Identifier: EUPL-1.2

//! All #171 ordinary widget stages, both languages/sizes and every legal move.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
#[path = "../examples/support/lead_master_widgets.rs"]
mod views;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn lead_master_every_ordinary_stage_and_labelled_fixture() {
    for name in views::names() {
        views::render(&name, None);
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn lead_master_ordinary_widgets_every_legal_move_and_immediate_return() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            for n in [5, 6, 8, 13, 14, 22, 23, 27, 28, 29] {
                let build = || {
                    Harness::builder().with_size(size).build_eframe(|cc| {
                        GrandCoachApp::new(
                            &cc.egui_ctx,
                            SessionConfig {
                                seed: 3,
                                series: SeriesLength::Fixed(1),
                                opponents: Opponents::Levels([Level::Easy; 2]),
                                kontra_re: false,
                                display: TableDisplay::Closed,
                                ..SessionConfig::default()
                            },
                        )
                    })
                };
                let mut h = build();
                guided_support::lead_master_flow(&mut h, language, 9, n);
                let i = I18n::new(language);
                guided_support::null_click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
                let mut h = build();
                guided_support::lead_master_flow(&mut h, language, 0, n);
                assert!(h.state().session().guided().unwrap().rule.is_none());
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
