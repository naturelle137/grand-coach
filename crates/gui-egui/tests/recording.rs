// SPDX-License-Identifier: EUPL-1.2

//! User-facing file dialogs exercise actual files and isolated replay in both languages.

use eframe::egui::{self, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language};
use skat_app::{GameRecording, Level, Opponents, SeriesLength, SessionConfig};
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
            "skat-gui-recording-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn button(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}
fn path(h: &mut Harness<'_, GrandCoachApp>, label: &str, value: &str) {
    h.get_by_role_and_label(Role::TextInput, label).focus();
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_by_role_and_label(Role::TextInput, label)
        .type_text(value);
    h.run_steps(4);
}

#[test]
fn save_reopen_navigate_recover_and_close_preserve_live_decision_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let temp = Temp::new();
        let saved = temp.0.join("saved.json");
        let mut h = Harness::builder()
            .with_size([1600.0, 1000.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 7,
                        series: SeriesLength::Fixed(3),
                        kontra_re: true,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        ..SessionConfig::default()
                    },
                );
                app.set_language(language);
                app.opponent_delay = 0.0;
                app
            });
        for _ in 0..500 {
            if !h.state().session().opponent_to_move() {
                break;
            }
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!h.state().session().view().legal_actions.is_empty());
        let before = h.state().session().view();
        let recording = h
            .state()
            .session()
            .export_current_recording_privileged()
            .unwrap();
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Spiel speichern …", "Save game …"));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            saved.to_str().unwrap(),
        );
        assert!(
            h.get_by_role_and_label(Role::Button, t("Datei speichern", "Save file"))
                .accesskit_node()
                .is_disabled()
        );
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Verdeckte Informationen bewusst in die Datei schreiben",
                "Deliberately write concealed information to the file",
            ),
        )
        .click();
        h.run_steps(3);
        button(&mut h, t("Datei speichern", "Save file"));
        assert_eq!(GameRecording::open_local(&saved).unwrap(), recording);
        let original = fs::read(&saved).unwrap();
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Spiel speichern …", "Save game …"));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            saved.to_str().unwrap(),
        );
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Verdeckte Informationen bewusst in die Datei schreiben",
                "Deliberately write concealed information to the file",
            ),
        )
        .click();
        h.run_steps(4);
        button(&mut h, t("Datei speichern", "Save file"));
        h.get_by_label_contains(t("Der Pfad existiert bereits", "The path already exists"));
        assert_eq!(fs::read(&saved).unwrap(), original);
        button(&mut h, t("Abbrechen", "Cancel"));
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Aufnahme öffnen …", "Open recording …"));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            saved.to_str().unwrap(),
        );
        button(&mut h, t("Datei öffnen", "Open file"));
        h.get_by_label(t("Aufnahme — nur lesen", "Recording — read only"));
        assert!(
            h.get_by_role_and_label(Role::Button, t("Passe", "Pass"))
                .accesskit_node()
                .is_disabled()
        );
        assert!(
            h.query_by_label_contains(t("Vollständige Information", "Full information"))
                .is_none()
        );
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Alle verdeckten Karten und den Skat aufdecken",
                "Reveal all concealed cards and the Skat",
            ),
        )
        .click();
        h.run_steps(4);
        h.get_by_label_contains(t("Vollständige Information", "Full information"));
        for _ in 0..2 {
            button(&mut h, t("Anfang", "First"));
            button(&mut h, t("Weiter", "Next"));
            button(&mut h, t("Zurück", "Previous"));
            button(&mut h, t("Ende", "Last"));
        }
        assert_eq!(h.state().session().view(), before);
        // A failed replacement leaves the last valid replay and live session intact.
        let invalid = temp.0.join("corrupt.json");
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Aufnahme öffnen …", "Open recording …"));
        let wire = String::from_utf8(original.clone()).unwrap();
        for (bytes, de, english) in [
            ("{".to_owned(), "Ungültige JSON-Datei", "Invalid JSON file"),
            (
                wire.replace("\"format\": 1", "\"format\": 2"),
                "Diese Aufnahmeversion",
                "This recording schema version",
            ),
            (
                wire.replace("grand-coach-recording-1", "future"),
                "Diese Regel- oder Austeilversion",
                "This rules or dealing version",
            ),
            (
                wire.replace("\"InProgress\"", "\"Eingepasst\""),
                "Ungültige Aufnahme",
                "Invalid recording:",
            ),
            (
                " ".repeat(skat_app::MAX_RECORDING_BYTES + 1),
                "Aufnahme zu groß",
                "Recording too large",
            ),
        ] {
            fs::write(&invalid, bytes).unwrap();
            path(
                &mut h,
                t("Dateipfad:", "File path:"),
                invalid.to_str().unwrap(),
            );
            button(&mut h, t("Datei öffnen", "Open file"));
            h.get_by_label_contains(t(de, english));
            h.get_by_label(t("Aufnahme — nur lesen", "Recording — read only"));
            h.get_by_label_contains(t("Vollständige Information", "Full information"));
            assert_eq!(h.state().session().view(), before);
        }
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            temp.0.join("missing").to_str().unwrap(),
        );
        button(&mut h, t("Datei öffnen", "Open file"));
        h.get_by_label_contains(t(
            "Datei oder Zielordner nicht gefunden",
            "File or target folder not found",
        ));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            temp.0.to_str().unwrap(),
        );
        button(&mut h, t("Datei öffnen", "Open file"));
        h.get_by_label_contains(t(
            "Datei konnte nicht gelesen",
            "Could not read or fully write",
        ));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            saved.to_str().unwrap(),
        );
        button(&mut h, t("Datei öffnen", "Open file"));
        button(&mut h, t("Aufnahme schließen", "Close recording"));
        assert_eq!(h.state().session().view(), before);
        assert_eq!(
            h.state()
                .session()
                .export_current_recording_privileged()
                .unwrap(),
            recording
        );
        assert_eq!(fs::read(saved).unwrap(), original);
    }
}

#[test]
fn final_recording_labels_distinguish_all_pass_incomplete_and_early_null() {
    let temp = Temp::new();
    let null = GameRecording::read_local(
        include_bytes!("../../skat-app/tests/fixtures/recordings/null.json").as_slice(),
    )
    .unwrap();
    let mut all_pass = null.clone();
    all_pass.actions = vec![skat_app::Action::Pass; 3];
    all_pass.end = skat_app::RecordingEnd::Eingepasst;
    let mut incomplete = null.clone();
    incomplete.actions.clear();
    incomplete.end = skat_app::RecordingEnd::InProgress;
    assert!(
        null.reconstruct_privileged(null.actions.len())
            .unwrap()
            .state
            .seats[0]
            .tricks
            .len()
            < 10
    );
    for (n, (recording, status)) in [
        (null, "Abgeschlossen. Null kann"),
        (all_pass, "Eingepasst: kein Spielvertrag"),
        (incomplete, "Unvollständige Aufnahme"),
    ]
    .into_iter()
    .enumerate()
    {
        let file = temp.0.join(format!("{n}.json"));
        recording.save_local_new(&file).unwrap();
        let mut h = Harness::builder()
            .with_size([1600.0, 1000.0])
            .build_eframe(|cc| {
                GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 7,
                        series: SeriesLength::Fixed(0),
                        kontra_re: false,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        ..SessionConfig::default()
                    },
                )
            });
        let before = h.state().session().view();
        button(&mut h, "Spiel");
        button(&mut h, "Aufnahme öffnen …");
        path(&mut h, "Dateipfad:", file.to_str().unwrap());
        button(&mut h, "Datei öffnen");
        h.get_by_label_contains(status);
        if !recording.actions.is_empty() {
            button(&mut h, "Anfang");
            button(&mut h, "Ende");
        }
        assert_eq!(h.state().session().view(), before);
        button(&mut h, "Aufnahme schließen");
        assert_eq!(h.state().session().view(), before);
    }
}
