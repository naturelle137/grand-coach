// SPDX-License-Identifier: EUPL-1.2

//! Real de/en widgets: four families, feedback concealment, persisted exposure,
//! sticky help, retry/next/exit and an unchanged normal game/series.
use eframe::egui::{self, Role};
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language};
use skat_app::{FocusAction, LearningHistory, Level, Opponents, SessionConfig};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "focus-gui-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("history.json")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn app(file: PathBuf, language: Language) -> Harness<'static, GrandCoachApp> {
    let mut h = Harness::builder()
        .with_size([1600.0, 1200.0])
        .build_eframe(move |cc| {
            let mut app = GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed: 11,
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    ..Default::default()
                },
            );
            app.set_language(language);
            app.use_learning_file(file.clone());
            app.opponent_delay = 0.0;
            app
        });
    for _ in 0..2000 {
        if !h.state().live_session().view().opponent_to_move {
            break;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!h.state().live_session().view().opponent_to_move);
    h.state_mut().open_training();
    h.run_steps(3);
    h
}
fn button(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(3);
}

fn catalogue_flow_preserves_live_game_and_persists_exposure(language: Language) {
    let en = language == Language::En;
    let t = |de, english| if en { english } else { de };
    for (topic, intro, changed, boundary) in [
        (
            t("Fokus: Verbleibende Trümpfe", "Focus: Remaining Trumpf"),
            Some(5),
            Some(1),
            Some(0),
        ),
        (
            t("Fokus: Nachgewiesene Fehlfarben", "Focus: Proven voids"),
            Some(1),
            Some(1),
            Some(0),
        ),
        (
            t(
                "Fokus: Abstich einer vollen Nebenkarte",
                "Focus: Trumping a full side card",
            ),
            None,
            None,
            None,
        ),
        (
            t(
                "Fokus: Übernehmen durch den Partner",
                "Focus: Partner overtaking",
            ),
            None,
            None,
            None,
        ),
    ] {
        let dir = Temp::new();
        LearningHistory::new().save_local(dir.file()).unwrap();
        let mut h = app(dir.file(), language);
        let live = h.state().live_session().view();
        let recording = h
            .state()
            .live_session()
            .export_current_recording_privileged();
        button(&mut h, topic);
        assert!(h.state().focus_view().unwrap().feedback.is_none());
        let answer = |h: &mut Harness<'_, GrandCoachApp>, value: Option<u8>, absent: bool| {
            match value {
                Some(value) => {
                    for _ in 0..value {
                        button(h, "+ 1");
                    }
                    button(h, t("Antworten", "Answer"));
                }
                None => button(
                    h,
                    if absent {
                        t(
                            "Dieser Mechanismus ist nicht vorhanden",
                            "This mechanism is absent",
                        )
                    } else {
                        t("Möglich, aber nicht sicher", "Possible, but not certain")
                    },
                ),
            }
            assert_eq!(h.state().focus_view().unwrap().entry.correct(), Some(true));
        };
        answer(&mut h, intro, false);
        button(&mut h, t("Nächste Stellung", "Next position"));
        let v = h.state().focus_view().unwrap();
        assert!(v.entry.eligible);
        assert!(v.feedback.is_none());
        // Exit with the changed task unanswered, then restart on the same file.
        button(&mut h, t("Zurück zur Partie", "Back to the game"));
        assert_eq!(h.state().live_session().view(), live);
        assert_eq!(
            h.state()
                .live_session()
                .export_current_recording_privileged(),
            recording
        );
        drop(h);
        let persisted = LearningHistory::open_local(dir.file()).unwrap();
        assert_eq!(persisted.focus_entries().len(), 3);
        assert_eq!(
            persisted.focus_entries().last().unwrap().action,
            FocusAction::Presented
        );
        let mut h = app(dir.file(), language);
        let live = h.state().live_session().view();
        button(&mut h, topic);
        button(&mut h, t("Überspringen", "Skip"));
        button(&mut h, t("Nächste Stellung", "Next position"));
        assert!(!h.state().focus_view().unwrap().entry.eligible);
        button(&mut h, t("Hilfe einblenden", "Show help"));
        assert!(h.state().focus_view().unwrap().entry.assisted);
        assert!(h.state().focus_view().unwrap().feedback.is_some());
        answer(&mut h, changed, false);
        button(&mut h, t("Noch einmal", "Try again"));
        assert!(!h.state().focus_view().unwrap().entry.eligible);
        assert!(h.state().focus_view().unwrap().feedback.is_none());
        button(&mut h, t("Überspringen", "Skip"));
        button(&mut h, t("Nächste Stellung", "Next position"));
        answer(&mut h, boundary, true);
        button(&mut h, t("Zurück zur Partie", "Back to the game"));
        assert_eq!(h.state().live_session().view(), live);
        assert!(
            LearningHistory::open_local(dir.file())
                .unwrap()
                .focus_entries()
                .iter()
                .any(|e| e.assisted)
        );
    }
}

#[test]
fn german_catalogue_flow_preserves_live_game_and_persists_exposure() {
    catalogue_flow_preserves_live_game_and_persists_exposure(Language::De);
}

#[test]
fn english_catalogue_flow_preserves_live_game_and_persists_exposure() {
    catalogue_flow_preserves_live_game_and_persists_exposure(Language::En);
}

#[test]
fn missing_corrupt_and_unsupported_history_allow_practice_without_credit() {
    for bytes in [None, Some("broken"), Some(r#"{"format":999}"#)] {
        let dir = Temp::new();
        if let Some(bytes) = bytes {
            fs::write(dir.file(), bytes).unwrap();
        }
        let mut h = app(dir.file(), Language::En);
        button(&mut h, "Focus: Remaining Trumpf");
        button(&mut h, "Skip");
        button(&mut h, "Next position");
        assert!(!h.state().focus_view().unwrap().entry.eligible);
        if let Some(bytes) = bytes {
            assert_eq!(fs::read_to_string(dir.file()).unwrap(), bytes);
        }
    }
}

#[test]
fn failed_atomic_save_withholds_the_next_position_until_it_is_durable() {
    let dir = Temp::new();
    LearningHistory::new().save_local(dir.file()).unwrap();
    let mut h = app(dir.file(), Language::En);
    button(&mut h, "Focus: Remaining Trumpf");
    button(&mut h, "Skip");
    let previous = h.state().focus_view().unwrap();
    // A directory cannot be atomically replaced by the history file.
    fs::remove_file(dir.file()).unwrap();
    fs::create_dir(dir.file()).unwrap();
    button(&mut h, "Next position");
    let after = h.state().focus_view().unwrap();
    assert_eq!(after.entry.task, previous.entry.task);
    assert_eq!(after.position, previous.position);
    assert_eq!(after.number, 1);
    assert!(!after.entry.eligible);
    h.get_by_label("Position or history unavailable; no reliable score or novelty credit.");
}

// No screenshots or debug dumps of reserved task views. These are simulated
// technical input events, never human UAT records.
fn validation_widgets(language: Language) {
    let en = language == Language::En;
    let t = |de, english| if en { english } else { de };
    let dir = Temp::new();
    LearningHistory::new().save_local(dir.file()).unwrap();
    let mut h = app(dir.file(), language);
    h.set_size(egui::vec2(1600.0, 1800.0));
    h.run_steps(3);
    let live = h.state().live_session().view();
    button(
        &mut h,
        t("Lernvalidierung vorbereiten", "Prepare learner validation"),
    );
    assert!(
        h.state().focus_view().is_none(),
        "entry chooser cannot expose reserved views"
    );
    button(
        &mut h,
        t(
            "Anfangssitzung bewusst starten",
            "Deliberately start initial session",
        ),
    );
    assert!(h.state().focus_view().unwrap().feedback.is_none());
    h.get_by_label(t("Aufgabe 1 von 4", "Task 1 of 4"));
    h.get_by_role_and_label(
        Role::RadioButton,
        t("Noch nie gesehen", "Never seen before"),
    )
    .click();
    h.run_steps(2);
    let words = t("Eigene Worte (freiwillig)", "Own words (optional)");
    h.get_by_role_and_label(Role::MultilineTextInput, words)
        .focus();
    h.run_steps(1);
    h.get_by_role_and_label(Role::MultilineTextInput, words)
        .type_text("My unedited observation");
    h.run_steps(2);
    button(&mut h, "+ 1");
    button(&mut h, "+ 1");
    button(&mut h, t("Antworten", "Answer"));
    assert_eq!(h.state().focus_view().unwrap().entry.correct(), Some(true));
    let stored = LearningHistory::open_local(dir.file()).unwrap();
    assert_eq!(
        stored
            .focus_entries()
            .last()
            .unwrap()
            .validation
            .as_ref()
            .unwrap()
            .own_words,
        "My unedited observation"
    );
    button(&mut h, t("Noch einmal", "Try again"));
    assert!(h.state().focus_view().unwrap().entry.assisted);
    button(&mut h, t("Hilfe einblenden", "Show help"));
    assert!(h.state().focus_view().unwrap().feedback.is_some());
    button(&mut h, t("Aufgabe unklar", "Task unclear"));
    h.get_by_label(t(
        "Als unklar erfasst – keine Wertung.",
        "Recorded as unclear – not scored.",
    ));
    button(&mut h, t("Nächste Stellung", "Next position"));
    assert!(h.state().focus_view().unwrap().feedback.is_none());
    button(&mut h, t("Überspringen", "Skip"));
    button(&mut h, t("Nächste Stellung", "Next position"));
    button(&mut h, t("Zurück zur Partie", "Back to the game"));
    assert_eq!(h.state().live_session().view(), live);
    let counts = LearningHistory::open_local(dir.file())
        .unwrap()
        .validation_summary()
        .counts;
    assert_eq!(counts.opportunities, 4);
    assert_eq!(counts.answers, 1);
    assert_eq!(counts.unclear, 1);
    assert_eq!(counts.skipped, 1);
    assert_eq!(counts.unanswered, 1);
    assert_eq!(counts.help, 1);
    assert_eq!(counts.repeats, 1);
    assert_eq!(counts.eligible_correct, 1);
}

#[test]
fn reserved_validation_widgets_de() {
    validation_widgets(Language::De);
}
#[test]
fn reserved_validation_widgets_en() {
    validation_widgets(Language::En);
}

#[test]
fn validation_failed_save_blocks_feedback_and_advancement() {
    let dir = Temp::new();
    LearningHistory::new().save_local(dir.file()).unwrap();
    let mut h = app(dir.file(), Language::En);
    h.set_size(egui::vec2(1600.0, 1800.0));
    h.run_steps(3);
    button(&mut h, "Prepare learner validation");
    button(&mut h, "Deliberately start initial session");
    let before = h.state().focus_view().unwrap().entry.attempt;
    let backup = dir.0.join("retained.json");
    fs::rename(dir.file(), &backup).unwrap();
    fs::create_dir(dir.file()).unwrap();
    button(&mut h, "Answer");
    assert_eq!(h.state().focus_view().unwrap().entry.attempt, before);
    assert!(h.state().focus_view().unwrap().feedback.is_none());
    assert!(h.state().focus_view().unwrap().entry.correct().is_none());
    assert!(!h.state().focus_view().unwrap().entry.eligible);
    h.get_by_label("Validation stopped: this task cannot be reliably saved to durable storage. Return and report the failure. Do not open another task.");
    assert_eq!(
        LearningHistory::open_local(backup)
            .unwrap()
            .focus_entries()
            .len(),
        1
    );
}

#[test]
fn unexpected_aid_widget_records_assistance_and_stops_without_feedback() {
    let dir = Temp::new();
    LearningHistory::new().save_local(dir.file()).unwrap();
    let mut h = app(dir.file(), Language::En);
    h.set_size(egui::vec2(1600.0, 1800.0));
    h.run_steps(3);
    button(&mut h, "Prepare learner validation");
    button(&mut h, "Deliberately start initial session");
    button(&mut h, "Unexpected additional aid seen – stop session");
    let view = h.state().focus_view().unwrap();
    assert!(view.feedback.is_none());
    assert!(view.entry.assisted);
    assert!(matches!(view.entry.action, FocusAction::Stopped));
    let persisted = LearningHistory::open_local(dir.file()).unwrap();
    assert!(
        persisted
            .focus_entries()
            .last()
            .unwrap()
            .validation
            .as_ref()
            .unwrap()
            .external_aid
    );
}

#[test]
fn prepared_validation_launcher_opens_confirmation_without_a_task_or_new_series() {
    let dir = Temp::new();
    LearningHistory::new().save_local(dir.file()).unwrap();
    let mut h = app(dir.file(), Language::En);
    h.state_mut().exit_training();
    let live = h.state().live_session().view();
    h.state_mut().prepare_validation();
    h.run_steps(3);
    assert!(h.state().training_active());
    assert!(h.state().focus_view().is_none());
    assert!(!h.state().new_series_open());
    h.get_by_label("Deliberately start initial session");
    assert_eq!(h.state().live_session().view(), live);
    assert!(
        LearningHistory::open_local(dir.file())
            .unwrap()
            .focus_entries()
            .is_empty()
    );
}
