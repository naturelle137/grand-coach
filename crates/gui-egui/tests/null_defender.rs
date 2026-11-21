// SPDX-License-Identifier: EUPL-1.2

//! Ordinary Null defender dialogue through the real widgets.

#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_null_defender_widgets() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            let output = std::env::var("NULL_DEFENDER_SCREENSHOTS").ok();
            let builder = Harness::builder().with_size(size);
            let builder = if output.is_some() {
                builder.wgpu()
            } else {
                builder
            };
            let mut h = builder.build_eframe(|cc| {
                GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 2115,
                        series: SeriesLength::Fixed(3),
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        kontra_re: false,
                        display: TableDisplay::Closed,
                        ..SessionConfig::default()
                    },
                )
            });
            h.state_mut().set_language(language);
            h.state_mut().opponent_delay = 0.0;
            h.state_mut().set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            let i = I18n::new(language);
            for cards in [
                [1, 2, 5, 6, 8, 7, 11, 24, 29, 31],
                [22, 3, 6, 8, 28, 9, 24, 29, 10, 13],
            ] {
                support::settle(&mut h);
                support::click(&mut h, &i.t("button-pass"));
                for n in cards {
                    support::settle(&mut h);
                    support::click(
                        &mut h,
                        &support::card_name(&i, Card::from_index(n).unwrap()),
                    );
                }
                support::settle(&mut h);
                support::click(&mut h, &i.t("button-next-game"));
            }
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-pass"));
            support::settle(&mut h);
            let v = h.state().session().view();
            assert_eq!(
                (v.game_number, v.tricks_played, v.players[0].seat),
                (3, 0, Seat::Mittelhand)
            );
            assert_eq!(v.current_trick[0].card.index(), 0);
            support::click(&mut h, &i.t("guided-start"));
            h.get_by_label(&i.t("guided-null-defender-role"));
            assert!(h.state().session().guided().unwrap().rule.is_none());
            support::null_click(&mut h, &i.t("guided-unknown"));
            assert_eq!(
                h.state()
                    .session()
                    .guided()
                    .unwrap()
                    .rule
                    .unwrap()
                    .card
                    .index(),
                31
            );
            h.get_by_label(&i.t("guided-rule-null-defender"));
            if let Some(dir) = &output {
                h.render()
                    .unwrap()
                    .save(format!(
                        "{dir}/null-defender-rule-{language:?}-{}.png",
                        size[0]
                    ))
                    .unwrap();
            }
            support::null_click(
                &mut h,
                &support::card_name(&i, Card::from_index(31).unwrap()),
            );
            for _ in 0..5000 {
                h.run_steps(1);
                if h.state().session().guided().unwrap().step == GuidedStep::Observe {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_eq!(
                h.state().session().guided().unwrap().step,
                GuidedStep::Observe
            );
            support::null_click(&mut h, &i.t("guided-unknown"));
            assert_eq!(
                h.state()
                    .session()
                    .guided()
                    .unwrap()
                    .observation
                    .unwrap()
                    .winner,
                Some(Seat::Hinterhand)
            );
            if let Some(dir) = &output {
                h.render()
                    .unwrap()
                    .save(format!(
                        "{dir}/null-defender-observation-{language:?}-{}.png",
                        size[0]
                    ))
                    .unwrap();
            }
        }
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_null_last_defender_widgets() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            let output = std::env::var("NULL_LAST_DEFENDER_SCREENSHOTS").ok();
            let builder = Harness::builder().with_size(size);
            let builder = if output.is_some() {
                builder.wgpu()
            } else {
                builder
            };
            let mut h = builder.build_eframe(|cc| {
                GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 3211,
                        series: SeriesLength::Fixed(3),
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        kontra_re: false,
                        ..SessionConfig::default()
                    },
                )
            });
            h.state_mut().set_language(language);
            h.state_mut().opponent_delay = 0.0;
            h.state_mut().set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            let i = I18n::new(language);
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-pass"));
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-next-game"));
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-pass"));
            for n in [11, 1, 15, 5, 28, 17, 24, 22, 23, 29] {
                support::settle(&mut h);
                support::click(
                    &mut h,
                    &support::card_name(&i, Card::from_index(n).unwrap()),
                );
            }
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-next-game"));
            support::settle(&mut h);
            support::click(&mut h, &i.t("button-pass"));
            support::settle(&mut h);
            support::click(
                &mut h,
                &support::card_name(&i, Card::from_index(1).unwrap()),
            );
            support::settle(&mut h);
            assert_eq!(h.state().session().view().current_trick.len(), 2);
            support::click(&mut h, &i.t("guided-start"));
            h.get_by_label(&i.t("guided-null-defender-last-role"));
            assert!(h.state().session().guided().unwrap().rule.is_none());
            support::null_click(&mut h, &i.t("guided-unknown"));
            h.get_by_label(&i.t("guided-rule-null-defender-last"));
            if let Some(dir) = &output {
                h.render()
                    .unwrap()
                    .save(format!(
                        "{dir}/last-defender-rule-{language:?}-{}.png",
                        size[0]
                    ))
                    .unwrap();
            }
            let card = h.state().session().guided().unwrap().rule.unwrap().card;
            support::null_click(&mut h, &support::card_name(&i, card));
            assert_eq!(
                h.state().session().guided().unwrap().step,
                GuidedStep::Observe
            );
            support::null_click(&mut h, &i.t("guided-unknown"));
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
            if let Some(dir) = &output {
                h.render()
                    .unwrap()
                    .save(format!(
                        "{dir}/last-defender-observation-{language:?}-{}.png",
                        size[0]
                    ))
                    .unwrap();
            }
        }
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn ordinary_null_defender_opening_widgets() {
    for ouvert in [false, true] {
        for language in Language::ALL {
            for size in [[1440.0, 1000.0], [960.0, 640.0]] {
                let output = std::env::var("NULL_DEFENDER_LEAD_SCREENSHOTS").ok();
                let builder = Harness::builder().with_size(size);
                let builder = if output.is_some() {
                    builder.wgpu()
                } else {
                    builder
                };
                let mut h = builder.build_eframe(|cc| {
                    GrandCoachApp::new(
                        &cc.egui_ctx,
                        SessionConfig {
                            seed: if ouvert { 2345 } else { 1282 },
                            series: SeriesLength::Fixed(1),
                            opponents: Opponents::Levels([Level::Easy; 2]),
                            kontra_re: false,
                            ..SessionConfig::default()
                        },
                    )
                });
                h.state_mut().set_language(language);
                h.state_mut().opponent_delay = 0.0;
                h.state_mut().set_coach_budget(Budget {
                    samples: 1,
                    nodes_per_task: 1,
                });
                let i = I18n::new(language);
                support::settle(&mut h);
                support::click(&mut h, &i.t("button-pass"));
                support::settle(&mut h);
                assert_eq!(
                    h.state().session().view().contract.unwrap().game,
                    GameType::Null
                );
                assert_eq!(h.state().session().view().contract.unwrap().ouvert, ouvert);
                support::click(&mut h, &i.t("guided-start"));
                h.get_by_label(&i.t(if ouvert {
                    "guided-null-ouvert-defender-lead-role"
                } else {
                    "guided-null-defender-lead-role"
                }));
                support::null_click(&mut h, &i.t("guided-lead-defender"));
                support::null_click(&mut h, &i.t("guided-unknown"));
                h.get_by_label(&i.t("guided-rule-null-lead"));
                if let Some(dir) = &output {
                    h.render()
                        .unwrap()
                        .save(format!(
                            "{dir}/null-defender-lead-{ouvert}-rule-{language:?}-{}.png",
                            size[0]
                        ))
                        .unwrap();
                }
                let card = h.state().session().guided().unwrap().rule.unwrap().card;
                support::null_click(&mut h, &support::card_name(&i, card));
                for _ in 0..5000 {
                    h.run_steps(1);
                    if h.state().session().guided().unwrap().step == GuidedStep::Observe {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                assert_eq!(h.state().session().guided().unwrap().lead_replies.len(), 2);
                support::null_click(&mut h, &i.t("guided-unknown"));
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
                if let Some(dir) = &output {
                    h.render()
                        .unwrap()
                        .save(format!(
                            "{dir}/null-defender-lead-{ouvert}-observe-{language:?}-{}.png",
                            size[0]
                        ))
                        .unwrap();
                }
                support::null_click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
            }
        }
    }
}
