// SPDX-License-Identifier: EUPL-1.2

//! Full widget flow and restart from the real single history file.
use eframe::egui::{self, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::{CoachLevel, CoachSettings, Level, Opponents, SeriesLength, SessionConfig};
#[path = "../examples/support/live_abstich_flow.rs"]
mod flow;

fn harness() -> Harness<'static, GrandCoachApp> {
    let mut h = Harness::builder()
        .with_size([1280.0, 820.0])
        .build_eframe(|cc| {
            GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed: 21,
                    series: SeriesLength::Free,
                    kontra_re: false,
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    ..SessionConfig::default()
                },
            )
        });
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach(CoachSettings {
        level: CoachLevel::Off,
        ..CoachSettings::default()
    });
    h.run_steps(4);
    h
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn live_widgets_bilingual_persistence_help_skip_unsupported_and_return() {
    for language in Language::ALL {
        for stage in ["answer", "help", "skip", "unsupported", "return"] {
            let mut h = harness();
            let dir = std::env::temp_dir().join(format!(
                "grand-coach-live-{}-{language:?}-{stage}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("learning-history.json");
            h.state_mut().use_learning_file(path.clone());
            flow::stage(&mut h, language, stage);
            let in_memory = h.state().learning_history().clone();
            let disk = skat_app::LearningHistory::open_local(&path).unwrap();
            assert_eq!(disk, in_memory);
            assert!(!disk.live_abstich_entries().is_empty());
            let counts = disk.live_abstich_summary();
            match stage {
                "help" => {
                    assert_eq!(counts[1].1.unanswered, 1);
                    assert_eq!(counts[1].1.scored, 0);
                }
                "skip" => {
                    assert_eq!(counts[0].1.skipped, 1);
                    assert_eq!(counts[0].1.scored, 0);
                }
                "unsupported" => assert_eq!(counts[0].1.unsupported, 1),
                _ => assert_eq!(counts[0].1.answered, 1),
            }
            // New application instance, same file: actual persisted observations,
            // never replaying a recording or injecting catalogue records.
            let mut restarted = harness();
            restarted.state_mut().use_learning_file(path);
            restarted.state_mut().set_language(language);
            restarted.state_mut().show_learning();
            restarted.run_steps(5);
            assert_eq!(
                restarted.state().learning_history().live_abstich_summary(),
                counts
            );
            if stage == "help" {
                let i = I18n::new(language);
                flow::click(&mut h, &i.t("focus-possible"));
                assert_eq!(
                    h.state().learning_history().live_abstich_summary()[1]
                        .1
                        .answered,
                    1
                );
                assert_eq!(
                    h.state().learning_history().live_abstich_summary()[0]
                        .1
                        .answered,
                    0
                );
            }
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}

// Keep the frontend's minimum window layout exercised without a renderer.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn live_widgets_small_window_still_returns_without_mutation() {
    let mut h = harness();
    let card = flow::reach(&mut h, Language::De);
    let before = h
        .state()
        .session()
        .export_current_recording_privileged()
        .unwrap();
    h.set_size(egui::Vec2::new(960.0, 640.0));
    h.run_steps(5);
    flow::select(&mut h, card, Language::De);
    flow::close(&mut h, Language::De);
    assert_eq!(
        h.state()
            .session()
            .export_current_recording_privileged()
            .unwrap(),
        before
    );
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn live_history_reset_cancel_and_corrupt_file_recovery() {
    let mut h = harness();
    let dir = std::env::temp_dir().join(format!("grand-coach-live-reset-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("learning-history.json");
    h.state_mut().use_learning_file(path.clone());
    flow::stage(&mut h, Language::En, "overview");
    let before = std::fs::read(&path).unwrap();
    let prior = h.state().learning_history().attempts().last().unwrap().id;
    fn sclick(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
        h.get_by_role_and_label(Role::Button, label).scroll_to_me();
        h.run_steps(5);
        flow::click(h, label);
    }
    sclick(&mut h, "Reset learning history …");
    sclick(&mut h, "Cancel");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    sclick(&mut h, "Reset learning history …");
    sclick(&mut h, "Delete history");
    assert!(h.state().learning_history().is_empty());
    let mut reopened = skat_app::LearningHistory::open_local(&path).unwrap();
    assert!(
        reopened
            .begin_attempt(skat_app::AttemptKind::Series)
            .unwrap()
            > prior
    );
    assert!(h.state().session().live_abstich_observations().is_empty());
    sclick(&mut h, "Close");
    let card = flow::reach(&mut h, Language::En);
    flow::select(&mut h, card, Language::En);
    let after_reset = h.state().learning_history().live_abstich_summary();
    assert_eq!(after_reset[0].1.unanswered, 1);
    assert_eq!(after_reset[0].1.answered, 0);
    assert!(h.state().learning_history().attempts().last().unwrap().id > prior);
    // A corrupt file stays byte-identical until deliberate recovery; new
    // observations in memory must not silently replace it.
    let bad = b"{broken live history";
    std::fs::write(&path, bad).unwrap();
    let mut recovery = harness();
    recovery.state_mut().use_learning_file(path.clone());
    flow::stage(&mut recovery, Language::En, "overview");
    assert_eq!(std::fs::read(&path).unwrap(), bad);
    sclick(&mut recovery, "Rename damaged file and start a new history");
    assert_eq!(
        std::fs::read(dir.join("learning-history.damaged-1.json")).unwrap(),
        bad
    );
    assert!(
        !skat_app::LearningHistory::open_local(&path)
            .unwrap()
            .live_abstich_entries()
            .is_empty()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
