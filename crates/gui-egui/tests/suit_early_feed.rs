// SPDX-License-Identifier: EUPL-1.2
//! Ordinary bilingual Farbspiel early-feeding widgets at both supported sizes.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
#[path = "support/suit_early_feed.rs"]
mod suit_early_feed_support;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn suit_early_feed_bilingual_widgets_attempt_reveal_free_card_actual_reply_return() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            for chosen in [0, 1] {
                let mut h = Harness::builder().with_size(size).build_eframe(|cc| {
                    GrandCoachApp::new(
                        &cc.egui_ctx,
                        SessionConfig {
                            seed: 4,
                            series: SeriesLength::Fixed(3),
                            opponents: Opponents::Levels([Level::Easy; 2]),
                            ..SessionConfig::default()
                        },
                    )
                });
                suit_early_feed_support::flow(&mut h, language, 3, chosen);
                let i = I18n::new(language);
                guided_support::click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
            }
        }
    }
}
