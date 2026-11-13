// SPDX-License-Identifier: EUPL-1.2

//! M7 recording review (D-69) through real widgets in both languages:
//! reopen a completed recording, request the review, cancel and continue,
//! see the moments and their scope, navigate to a moment's decision (the
//! historical comment appears without computing), keep explicit hindsight
//! separately labelled, and leave the live series, the practice flow and
//! the file untouched. A saved in-progress game reports the review as
//! unavailable.

use eframe::egui::{self, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{
    Budget, GameRecording, Level, Opponents, ReplayCoach, ReviewStatus, SeriesLength, SessionConfig,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

const HERZ: &str = include_str!("../../skat-app/tests/fixtures/recordings/herz.json");

const SMALL: Budget = Budget {
    samples: 2,
    nodes_per_task: 1_000_000,
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "skat-gui-replay-review-{}-{}",
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
    // AccessKit's click: independent of where the scroll area put it.
    h.get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    h.run_steps(4);
}

fn disabled(h: &Harness<'_, GrandCoachApp>, label: &str) -> bool {
    h.get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .is_disabled()
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
    let deadline = Instant::now() + Duration::from_secs(300);
    while h.state().review_running() || h.state().replay_computing() {
        assert!(Instant::now() < deadline, "analysis did not finish");
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(2));
    }
    h.run_steps(4);
}

fn has(h: &Harness<'_, GrandCoachApp>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

fn count(h: &Harness<'_, GrandCoachApp>, text: &str) -> usize {
    h.query_all_by_label_contains(text).count()
}

#[test]
fn review_cancel_continue_navigate_hindsight_close_and_practice_in_both_languages() {
    let temp = Temp::new();
    let fixture = temp.0.join("fixture.json");
    fs::write(&fixture, HERZ).unwrap();
    let recording = GameRecording::open_local(&fixture).unwrap();
    // Independent expectation from the application interface.
    let mut expected = ReplayCoach::open(&recording, SMALL).unwrap();
    assert!(expected.request_review());
    while let Some(job) = expected.review_job() {
        expected.finish_review(job.run(None)).unwrap();
    }
    let ReviewStatus::Done(summary) = expected.review_status() else {
        panic!("review not done");
    };
    assert!(!summary.moments.is_empty());
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let i18n = I18n::new(language);
        let temp = Temp::new();
        let file = temp.0.join("herz.json");
        fs::write(&file, HERZ).unwrap();
        let bytes = fs::read(&file).unwrap();
        let mut h = Harness::builder()
            .with_size([1600.0, 1400.0])
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
                app.set_coach_budget(SMALL);
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

        let open = |h: &mut Harness<'_, GrandCoachApp>, file: &str| {
            button(h, t("Spiel", "Game"));
            button(h, t("Aufnahme öffnen …", "Open recording …"));
            path(h, t("Dateipfad:", "File path:"), file);
            button(h, t("Datei öffnen", "Open file"));
        };
        open(&mut h, file.to_str().unwrap());
        h.get_by_label(t(
            "Spielauswertung: lehrreiche Momente",
            "Game review: teachable moments",
        ));
        assert!(has(
            &h,
            t("13 Entscheidungen mit Wahl", "13 decisions with a choice")
        ));
        let request = t("Meine Entscheidungen auswerten", "Review my decisions");
        let single = t("Kommentar anfordern", "Request comment");
        let show = t(
            "Am Tisch vor diesem Zug zeigen",
            "Show the table before this move",
        );
        assert!(
            !h.state().review_running(),
            "nothing runs before the request"
        );

        // Select a decision, then request the review: the single comment
        // and hindsight controls wait while it runs.
        let combo = t("Entscheidung", "Decision");
        let first = &summary.moments[0];
        let label = coach::recorded_decision(&i18n, &expected.decisions()[first.decision]);
        h.get_by_role_and_label(Role::ComboBox, combo).click();
        h.run_steps(4);
        h.get_by_label(&label).click();
        h.run_steps(4);
        button(&mut h, request);
        assert!(h.state().review_running());
        assert!(has(&h, t("Werte Entscheidung", "Evaluating decision")));
        assert!(disabled(&h, single));
        assert!(disabled(
            &h,
            t(
                "Rückblick: mit allen Karten vergleichen",
                "Hindsight: compare with all cards"
            )
        ));

        // Cancel: partial and labelled as such, never as the whole game.
        button(&mut h, t("Auswertung abbrechen", "Cancel review"));
        assert!(!h.state().review_running());
        assert!(has(&h, t("Abgebrochen:", "Cancelled:")));
        assert!(has(
            &h,
            t("nicht aus dem ganzen Spiel", "not from the whole game")
        ));
        // Continue to the end.
        button(&mut h, t("Auswertung fortsetzen", "Continue review"));
        wait(&mut h);
        assert!(has(
            &h,
            t(
                "Alle 13 Entscheidungen mit Wahl ausgewertet (1 ohne Alternative übersprungen)",
                "All 13 decisions with a choice evaluated (1 without an alternative skipped)"
            )
        ));
        assert!(has(
            &h,
            t(
                "Damaliger Wissensstand: Auswahl",
                "What you knew then: selection"
            )
        ));
        assert_eq!(count(&h, show), summary.moments.len());
        assert!(has(&h, t("Verglichen mit", "Compared with")));
        assert!(!disabled(&h, single));

        // Navigate to the last moment: its decision is selected and the
        // historical comment shows at once, without computing.
        let last = summary.moments.last().unwrap();
        let label = coach::recorded_decision(&i18n, &expected.decisions()[last.decision]);
        h.query_all_by_label(show).last().unwrap().click_accesskit();
        h.run_steps(4);
        h.get_by_value(&label);
        assert!(!h.state().replay_computing());
        assert!(has(
            &h,
            t(
                "Kommentar aus damaliger Sicht",
                "Comment from your view then"
            )
        ));
        assert!(!has(
            &h,
            t(
                "RÜCKBLICK: alle Karten bekannt",
                "HINDSIGHT: all cards known"
            )
        ));

        // Explicit hindsight stays separate and labelled; the review stays.
        button(
            &mut h,
            t(
                "Rückblick: mit allen Karten vergleichen",
                "Hindsight: compare with all cards",
            ),
        );
        wait(&mut h);
        assert!(has(
            &h,
            t(
                "RÜCKBLICK: alle Karten bekannt",
                "HINDSIGHT: all cards known"
            )
        ));
        assert!(has(&h, t("Rückblick-Kommentar", "Hindsight comment")));
        assert_eq!(count(&h, show), summary.moments.len());

        // Close while the review runs; reopening starts idle.
        button(&mut h, t("Erneut auswerten", "Review again"));
        button(&mut h, t("Aufnahme schließen", "Close recording"));
        assert!(!h.state().review_running());
        assert_eq!(h.state().session().view(), live);
        open(&mut h, file.to_str().unwrap());
        assert!(!disabled(&h, request));
        assert_eq!(count(&h, show), 0);

        // Practice start while the review runs drops it; returning restores
        // the untouched live series.
        button(&mut h, request);
        assert!(h.state().review_running());
        button(&mut h, t("Dieses Blatt üben", "Practise this deal"));
        assert!(h.state().practicing());
        assert!(!h.state().review_running());
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

        // Save the running game and reopen it: no whole-game review.
        let saved = temp.0.join("running.json");
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
        h.run_steps(3);
        button(&mut h, t("Datei speichern", "Save file"));
        open(&mut h, saved.to_str().unwrap());
        assert!(has(
            &h,
            t(
                "Eine Auswertung des ganzen Spiels ist nicht verfügbar",
                "A review of the whole game is not available"
            )
        ));
        assert!(h.query_by_label(request).is_none());
        assert_eq!(h.state().session().view(), live);
    }
}
