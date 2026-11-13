// SPDX-License-Identifier: EUPL-1.2

//! M7 replay comments (D-67) through real widgets in both languages: reopen
//! a completed recording, select an earlier own decision, request the
//! historical comment, then the labelled hindsight comment; navigation,
//! close/reopen and practice start drop running work; the live series and
//! the file stay untouched.

use eframe::egui::{self, Role};
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{Budget, GameRecording, Level, Opponents, SeriesLength, SessionConfig};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

const HERZ: &str = include_str!("../../skat-app/tests/fixtures/recordings/herz.json");

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "skat-gui-replay-coach-{}-{}",
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

fn wait(h: &mut Harness<'_, GrandCoachApp>) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while h.state().replay_computing() {
        assert!(Instant::now() < deadline, "replay comment did not finish");
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(2));
    }
    h.run_steps(4);
}

fn has(h: &Harness<'_, GrandCoachApp>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

/// Picks `option` in the decision selector labelled `combo`.
fn select(h: &mut Harness<'_, GrandCoachApp>, combo: &str, option: &str) {
    h.get_by_role_and_label(Role::ComboBox, combo).click();
    h.run_steps(4);
    h.get_by_label(option).click();
    h.run_steps(4);
    h.get_by_value(option);
}

#[test]
fn reopen_select_historical_then_hindsight_navigate_close_and_practice_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let i18n = I18n::new(language);
        let temp = Temp::new();
        let file = temp.0.join("herz.json");
        fs::write(&file, HERZ).unwrap();
        let bytes = fs::read(&file).unwrap();
        let recording = GameRecording::open_local(&file).unwrap();
        let mut h = Harness::builder()
            .with_size([1600.0, 1200.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 7,
                        series: SeriesLength::Fixed(3),
                        kontra_re: true,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                    },
                );
                app.set_language(language);
                app.opponent_delay = 0.0;
                app.set_coach_budget(Budget {
                    samples: 4,
                    nodes_per_task: 2_000_000,
                });
                app
            });
        for _ in 0..500 {
            if !h.state().session().opponent_to_move() {
                break;
            }
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(1));
        }
        let live = h.state().session().view();
        let live_record = h
            .state()
            .session()
            .export_current_recording_privileged()
            .unwrap();
        assert!(!live.legal_actions.is_empty());

        let open = |h: &mut Harness<'_, GrandCoachApp>| {
            button(h, t("Spiel", "Game"));
            button(h, t("Aufnahme öffnen …", "Open recording …"));
            path(h, t("Dateipfad:", "File path:"), file.to_str().unwrap());
            button(h, t("Datei öffnen", "Open file"));
        };
        open(&mut h);
        h.get_by_label(t(
            "Frühere Entscheidung kommentieren",
            "Comment on an earlier decision",
        ));
        let none = t("Keine Entscheidung gewählt", "No decision selected");
        let combo = t("Entscheidung", "Decision");
        h.get_by_value(none);
        let coach = skat_app::ReplayCoach::open(&recording, Budget::default()).unwrap();
        let label = |prefix: usize| {
            let d = coach
                .decisions()
                .iter()
                .find(|d| d.prefix == prefix)
                .unwrap();
            coach::recorded_decision(&i18n, d)
        };
        let historical = t(
            "Sicht: damaliger Wissensstand",
            "Perspective: what you knew then",
        );
        let hindsight = t(
            "RÜCKBLICK: alle Karten bekannt",
            "HINDSIGHT: all cards known",
        );

        // Select trick 4 (the human trumps a Pik trick) and see the table
        // before the move, still uncomputed.
        select(&mut h, combo, &label(17));
        assert!(has(&h, historical) && !has(&h, hindsight));
        h.get_by_label(t(
            "Der Tisch vor deinem aufgenommenen Zug",
            "The table before your recorded move",
        ));
        assert!(has(&h, t("Noch nicht berechnet", "Not computed yet")));

        // Revealing hands for display does not switch the perspective.
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Alle verdeckten Karten und den Skat aufdecken",
                "Reveal all concealed cards and the Skat",
            ),
        )
        .click();
        h.run_steps(4);
        assert!(has(&h, historical) && !has(&h, hindsight));
        assert!(!h.state().replay_computing());

        button(&mut h, t("Kommentar anfordern", "Request comment"));
        wait(&mut h);
        assert!(has(&h, historical) && !has(&h, hindsight));
        assert!(has(&h, t("Verglichen mit", "Compared with")));
        assert!(has(&h, t("Du hast", "You chose")));

        // The deliberate hindsight control: persistently labelled.
        button(
            &mut h,
            t(
                "Rückblick: mit allen Karten vergleichen",
                "Hindsight: compare with all cards",
            ),
        );
        wait(&mut h);
        assert!(has(&h, hindsight) && !has(&h, historical));
        assert!(has(&h, t("Verglichen mit", "Compared with")));
        assert!(has(&h, t("kein Beweis", "no proof")));

        // Another decision: back to the historical perspective, nothing
        // computed until asked.
        select(&mut h, combo, &label(14));
        assert!(has(&h, historical) && !has(&h, hindsight));
        assert!(has(&h, t("Noch nicht berechnet", "Not computed yet")));

        // Rapid changes while computing never show a stale comment.
        button(&mut h, t("Kommentar anfordern", "Request comment"));
        select(&mut h, combo, &label(9));
        assert!(!h.state().replay_computing());
        assert!(has(&h, t("Noch nicht berechnet", "Not computed yet")));
        h.run_steps(20);
        assert!(!has(&h, t("Verglichen mit", "Compared with")));

        // Cancel, then retry.
        button(&mut h, t("Kommentar anfordern", "Request comment"));
        if h.state().replay_computing() {
            button(&mut h, t("Berechnung abbrechen", "Cancel computation"));
            assert!(has(&h, t("abgebrochen", "cancelled")));
        }
        button(&mut h, t("Erneut berechnen", "Compute again"));
        wait(&mut h);
        assert!(has(&h, t("Verglichen mit", "Compared with")));

        // Close while computing; reopening starts unselected and historical.
        button(&mut h, t("Erneut berechnen", "Compute again"));
        button(&mut h, t("Aufnahme schließen", "Close recording"));
        assert!(!h.state().replay_computing());
        assert_eq!(h.state().session().view(), live);
        open(&mut h);
        h.get_by_value(none);
        assert!(!has(&h, hindsight));

        // Practice start while computing drops the comment; return restores
        // the untouched live series.
        select(&mut h, combo, &label(17));
        button(&mut h, t("Kommentar anfordern", "Request comment"));
        button(&mut h, t("Dieses Blatt üben", "Practise this deal"));
        assert!(h.state().practicing());
        assert!(!h.state().replay_computing());
        button(&mut h, t("Zurück zur Serie", "Return to series"));
        assert!(!h.state().practicing());
        assert_eq!(h.state().session().view(), live);
        assert_eq!(
            h.state()
                .session()
                .export_current_recording_privileged()
                .unwrap(),
            live_record
        );
        assert_eq!(fs::read(&file).unwrap(), bytes);
    }
}
