// SPDX-License-Identifier: EUPL-1.2

//! #176 real ordinary widgets and labelled components, both sizes/languages.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
#[path = "../examples/support/null_lead_widgets.rs"]
mod views;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language};
use skat_app::*;
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_lead_every_new_stage_bilingual_both_sizes() {
    for name in views::names() {
        views::render(&name, None);
    }
}
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_lead_real_widgets_all_legal_choices_and_free_return() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            for chosen in [8, 14, 15, 22, 23, 25, 26, 28, 29, 31] {
                for stage in ["return", "free"] {
                    let mut h = Harness::builder().with_size(size).build_eframe(|cc| {
                        GrandCoachApp::new(
                            &cc.egui_ctx,
                            SessionConfig {
                                seed: 1,
                                series: SeriesLength::Fixed(1),
                                opponents: Opponents::Levels([Level::Easy; 2]),
                                kontra_re: false,
                                display: TableDisplay::Closed,
                                ..SessionConfig::default()
                            },
                        )
                    });
                    guided_support::null_lead_flow(&mut h, language, stage, chosen);
                }
            }
        }
    }
}
