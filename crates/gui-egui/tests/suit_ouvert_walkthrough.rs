// SPDX-License-Identifier: EUPL-1.2
//! Actual Farbspiel Ouvert dialogue through ordinary widgets.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_suit_ouvert_bilingual() {
    for language in Language::ALL {
        let output = std::env::var("SUIT_OUVERT_WALKTHROUGH").ok();
        let mut found = false;
        for seed in 0..30 {
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
            for _ in 0..500 {
                settle(&mut h);
                let v = h.state().session().view();
                if v.series_finished {
                    break;
                }
                if v.phase == Phase::Spiel
                    && !v.legal_actions.is_empty()
                    && v.contract
                        .is_some_and(|c| c.ouvert && c.game == GameType::Farbe(Suit::Karo))
                    && v.declarer == Some(PlayerId::HUMAN)
                    && v.current_trick.len() == 1
                {
                    click(&mut h, &i.t("guided-start"));
                    if h.state().session().guided().unwrap().step == GuidedStep::Orient {
                        click(&mut h, &i.t("guided-reveal"));
                        assert_eq!(
                            h.state()
                                .session()
                                .guided()
                                .unwrap()
                                .rule
                                .as_ref()
                                .unwrap()
                                .branch,
                            GuidedBranch::WinEarly
                        );
                        if let Some(path) = &output {
                            h.render()
                                .unwrap()
                                .save(format!("{path}/{language:?}-rule.png"))
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
                    click(&mut h, &i.t("guided-return"));
                }
                let v = h.state().session().view();
                if v.phase == Phase::Ansage && !v.legal_actions.is_empty() {
                    h.get_by_role_and_label(
                        Role::RadioButton,
                        &game_name(&i, GameType::Farbe(Suit::Karo)),
                    )
                    .click();
                    h.run_steps(4);
                    h.get_by_role_and_label(Role::CheckBox, &i.t("contract-ouvert"))
                        .click();
                    h.run_steps(4);
                    click(&mut h, &i.t("button-announce"));
                    continue;
                }
                let bid = v.legal_actions.iter().find_map(|a| match a {
                    Action::Bid(n) if *n <= 30 => Some(*n),
                    _ => None,
                });
                let label = if v.can_next_game {
                    i.t("button-next-game")
                } else if v.legal_actions.contains(&Action::Hold) && v.bid <= 30 {
                    i.tf("button-hold", &[("value", v.bid.into())])
                } else if let Some(n) = bid {
                    i.tf("button-bid", &[("value", n.into())])
                } else if v.legal_actions.contains(&Action::TakeSkat) {
                    i.t("button-hand")
                } else if v.legal_actions.contains(&Action::Pass) {
                    i.t("button-pass")
                } else if let Some(a) = v.legal_actions.first() {
                    gui_egui::coach::action(&i, &v, *a)
                } else {
                    panic!("no action: {v:?}");
                };
                click(&mut h, &label);
            }
            if found {
                break;
            }
        }
        assert!(found, "no real Ouvert WinEarly route for {language:?}");
    }
}
