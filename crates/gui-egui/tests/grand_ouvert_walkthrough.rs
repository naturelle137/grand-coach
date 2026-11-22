// SPDX-License-Identifier: EUPL-1.2
//! Ordinary Grand Ouvert walkthrough through actual widgets in both languages.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_grand_ouvert_bilingual() {
    for language in Language::ALL {
        let output = std::env::var("OUVERT_WALKTHROUGH").ok();
        let builder = Harness::builder().with_size([1440.0, 1000.0]);
        let builder = if output.is_some() {
            builder.wgpu()
        } else {
            builder
        };
        let mut h = builder.build_eframe(|cc| {
            GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed: 0,
                    series: SeriesLength::Fixed(3),
                    kontra_re: false,
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    ..SessionConfig::default()
                },
            )
        });
        enter_grand_contract(&mut h, language, true, Some(GuidedBranch::WinEarly), true);
        let i = I18n::new(language);
        click(&mut h, &i.t("guided-reveal"));
        assert_eq!(
            h.state().session().guided().unwrap().rule.unwrap().card,
            Card::new(Suit::Karo, Rank::Bube)
        );
        if let Some(path) = &output {
            h.render()
                .unwrap()
                .save(format!("{path}/{language:?}.png"))
                .unwrap();
        }
        let card = h
            .state()
            .session()
            .view()
            .legal_actions
            .iter()
            .find_map(|a| {
                if let Action::Play(c) = a {
                    Some(*c)
                } else {
                    None
                }
            })
            .unwrap();
        click(&mut h, &card_name(&i, card));
        settle(&mut h);
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        click(&mut h, &i.t("guided-reveal"));
        assert_eq!(
            h.state()
                .session()
                .guided()
                .unwrap()
                .observation
                .unwrap()
                .winner,
            Some(Seat::Vorhand)
        );
        if let Some(path) = &output {
            h.render()
                .unwrap()
                .save(format!("{path}/{language:?}-observed.png"))
                .unwrap();
        }
    }
}
