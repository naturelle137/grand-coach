// SPDX-License-Identifier: EUPL-1.2
//! Ordinary defender decline-fallback dialogue through actual widgets.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use egui_kittest::Harness;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_decline_bilingual() {
    for language in Language::ALL {
        let output = std::env::var("DECLINE_WALKTHROUGH").ok();
        let mut found = false;
        for seed in 0..100 {
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
                        seed,
                        series: SeriesLength::Fixed(3),
                        kontra_re: false,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        ..SessionConfig::default()
                    },
                )
            });
            let i = I18n::new(language);
            h.state_mut().set_language(language);
            h.state_mut().opponent_delay = 0.0;
            h.state_mut().set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            for _ in 0..300 {
                settle(&mut h);
                let v = h.state().session().view();
                if v.series_finished {
                    break;
                }
                if v.phase == Phase::Spiel
                    && v.current_trick.len() == 1
                    && v.legal_actions.len() >= 2
                {
                    click(&mut h, &i.t("guided-start"));
                    if h.state().session().guided().unwrap().step == GuidedStep::Orient {
                        click(&mut h, &i.t("guided-reveal"));
                        let g = h.state().session().guided().unwrap();
                        if g.rule
                            .as_ref()
                            .is_some_and(|r| r.branch == GuidedBranch::Fallback && !r.partner_ahead)
                        {
                            if let Some(path) = &output {
                                h.render()
                                    .unwrap()
                                    .save(format!("{path}/{language:?}-rule.png"))
                                    .unwrap();
                            }
                            let card = *g
                                .legal_cards
                                .iter()
                                .find(|&&card| card != g.rule.as_ref().unwrap().card)
                                .unwrap();
                            click(&mut h, &card_name(&i, card));
                            settle(&mut h);
                            assert_eq!(
                                h.state().session().guided().unwrap().step,
                                GuidedStep::Observe
                            );
                            click(&mut h, &i.t("guided-reveal"));
                            assert!(
                                h.state()
                                    .session()
                                    .guided()
                                    .unwrap()
                                    .observation
                                    .unwrap()
                                    .winner
                                    .is_some()
                            );
                            if let Some(path) = &output {
                                h.render()
                                    .unwrap()
                                    .save(format!("{path}/{language:?}-observed.png"))
                                    .unwrap();
                            }
                            found = true;
                            break;
                        }
                    }
                    click(&mut h, &i.t("guided-return"));
                }
                let v = h.state().session().view();
                let label = if v.can_next_game {
                    i.t("button-next-game")
                } else if v.legal_actions.contains(&Action::Pass) {
                    i.t("button-pass")
                } else if let Some(a) = v.legal_actions.first() {
                    gui_egui::coach::action(&i, &v, *a)
                } else {
                    panic!("no action: {v:?}")
                };
                click(&mut h, &label);
            }
            if found {
                break;
            }
        }
        assert!(found, "no ordinary decline route for {language:?}");
    }
}
