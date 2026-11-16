// SPDX-License-Identifier: EUPL-1.2

//! #169 ordinary widgets, both languages/sizes and every legal alternative.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use guided_support as support;
use skat_app::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_middle_ordinary_widgets_all_legal_cards_bilingual_both_sizes() {
    for language in Language::ALL {
        for size in [[1440.0, 1000.0], [960.0, 640.0]] {
            for n in [9, 10, 13, 14] {
                let mut h = Harness::builder().with_size(size).build_eframe(|cc| {
                    GrandCoachApp::new(
                        &cc.egui_ctx,
                        SessionConfig {
                            seed: 1,
                            series: SeriesLength::Fixed(3),
                            opponents: Opponents::Levels([Level::Easy; 2]),
                            kontra_re: false,
                            display: TableDisplay::Closed,
                        },
                    )
                });
                support::null_middle_flow(&mut h, language, 0, n);
                let i = I18n::new(language);
                let v = h.state().session().view();
                let recording = h.state().session().export_current_recording_privileged();
                let history = h.state().learning_history().clone();
                if n == 9 {
                    let old_id = h.state().session().guided().unwrap().id;
                    let file = std::env::temp_dir().join(format!(
                        "null-middle-replay-{}-{language:?}-{}.json",
                        std::process::id(),
                        size[0]
                    ));
                    recording.as_ref().unwrap().save_local_new(&file).unwrap();
                    let t = |de, en| if language == Language::De { de } else { en };
                    support::click(&mut h, t("Spiel", "Game"));
                    support::click(&mut h, t("Aufnahme öffnen …", "Open recording …"));
                    let path_label = t("Dateipfad:", "File path:");
                    h.get_by_role_and_label(eframe::egui::Role::TextInput, path_label)
                        .focus();
                    h.run_steps(1);
                    h.key_press_modifiers(eframe::egui::Modifiers::COMMAND, eframe::egui::Key::A);
                    h.get_by_role_and_label(eframe::egui::Role::TextInput, path_label)
                        .type_text(file.to_str().unwrap());
                    h.run_steps(4);
                    support::click(&mut h, t("Datei öffnen", "Open file"));
                    assert!(h.state().session().guided().is_none());
                    h.run_steps(20);
                    support::click(&mut h, t("Aufnahme schließen", "Close recording"));
                    h.run_steps(12);
                    support::click(&mut h, &i.t("guided-start"));
                    assert_ne!(h.state().session().guided().unwrap().id, old_id);
                    std::fs::remove_file(file).unwrap();
                }

                assert!(h.query_by_label(&i.t("guided-rule-null-middle")).is_none());
                assert!(h.query_by_label(&i.t("guided-memory")).is_none());
                h.get_by_label(&i.t("guided-null-middle-role"));
                support::null_click(&mut h, &i.t("guided-return"));
                assert_eq!(
                    h.state().session().export_current_recording_privileged(),
                    recording
                );
                assert_eq!(h.state().session().view().legal_actions, v.legal_actions);
                support::click(&mut h, &i.t("guided-start"));
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
                    10
                );
                support::null_click(&mut h, &i.t("guided-memory"));
                let m = h.state().session().guided().unwrap().memory.unwrap();
                assert!(m.skat.is_none() && m.trumpf.is_none());
                support::null_click(&mut h, &i.t("guided-memory-hide"));
                support::null_click(
                    &mut h,
                    &support::card_name(&i, Card::from_index(n).unwrap()),
                );
                for _ in 0..5000 {
                    h.run_steps(1);
                    if h.state().session().guided().unwrap().step == GuidedStep::Observe {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                h.state_mut().opponent_delay = 1000.0;
                assert_eq!(
                    h.state().session().guided().unwrap().step,
                    GuidedStep::Observe
                );
                assert!(h.state().session().guided().unwrap().observation.is_none());
                support::null_click(&mut h, &i.t("guided-void-yes"));
                assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
                support::null_click(&mut h, &i.t("guided-retry"));
                support::null_click(&mut h, &i.t("guided-void-no"));
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
                assert_eq!(
                    h.state()
                        .session()
                        .view()
                        .last_trick
                        .unwrap()
                        .cards
                        .into_iter()
                        .map(|p| p.card.index())
                        .collect::<Vec<_>>(),
                    [12, n, 15]
                );
                support::null_click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
                assert_eq!(h.state().learning_history(), &history);
            }
        }
    }
}

#[path = "../examples/support/null_middle_widgets.rs"]
mod views;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_middle_all_screenshot_stages_and_labelled_boundaries() {
    for name in views::names() {
        views::render(&name, None);
    }
}
