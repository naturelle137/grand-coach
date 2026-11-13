// SPDX-License-Identifier: EUPL-1.2

//! The learning history through the real window in both languages (D-70):
//! counting questions answered and skipped at the table are recorded,
//! survive a restart, are shown with their limited sample, can be reset
//! only after confirmation, and a damaged file is kept and set aside.

use eframe::egui::Role;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language};
use skat_app::{
    Action, AssistSettings, AssistanceLevel, Card, LearningHistory, Level, Observations, Opponents,
    SeriesLength, SessionConfig, TableView,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "skat-gui-learning-{}-{}",
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

fn app<'a>(language: Language, file: &Path) -> Harness<'a, GrandCoachApp> {
    let file = file.to_path_buf();
    Harness::builder()
        .with_size([1600.0, 1000.0])
        .build_eframe(move |cc| {
            let mut app = GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed: 11,
                    series: SeriesLength::Fixed(8),
                    kontra_re: false,
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    ..SessionConfig::default()
                },
            );
            app.set_language(language);
            app.opponent_delay = 0.0;
            app.use_learning_file(file.clone());
            app.set_assist(AssistSettings {
                counting_aid: false,
                quiz: true,
                retrieval: true,
                assistance: AssistanceLevel::AnswerBeforeReveal,
            });
            app
        })
}

fn button(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}

fn view(h: &Harness<'_, GrandCoachApp>) -> TableView {
    h.state().session().view()
}

fn settle(h: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..2000 {
        if !view(h).opponent_to_move {
            h.run_steps(2);
            return;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not finish");
}

fn card_label(card: Card, en: bool) -> String {
    let s = usize::from(card.suit().index());
    let r = usize::from(card.rank().index());
    if en {
        let suit = ["Clubs", "Spades", "Hearts", "Diamonds"][s];
        let rank = [
            "Seven", "Eight", "Nine", "Ten", "Jack", "Queen", "King", "Ace",
        ][r];
        format!("{rank} of {suit}")
    } else {
        let suit = ["Kreuz", "Pik", "Herz", "Karo"][s];
        let rank = [
            "Sieben", "Acht", "Neun", "Zehn", "Bube", "Dame", "König", "Ass",
        ][r];
        format!("{suit}-{rank}")
    }
}

/// Plays defensively through the widgets until `questions` counting
/// questions were settled: the first answered, the others skipped.
fn play_questions(h: &mut Harness<'_, GrandCoachApp>, en: bool, questions: usize) {
    let t = |de, english| if en { english } else { de };
    let mut settled = 0;
    for _ in 0..3000 {
        settle(h);
        let v = view(h);
        if v.assist.quiz.question.is_some() {
            let label = if settled == 0 {
                t("Antworten", "Answer")
            } else {
                t("Überspringen", "Skip")
            };
            button(h, label);
            settled += 1;
            if settled == questions {
                return;
            }
            continue;
        }
        if v.acting.is_none() {
            assert!(!v.series_finished, "series ended with {settled} questions");
            button(h, t("Nächstes Spiel", "Next game"));
            continue;
        }
        let legal = v.legal_actions;
        if legal.contains(&Action::Pass) {
            button(h, t("Passe", "Pass"));
        } else {
            let Action::Play(card) = legal[0] else {
                panic!("unscripted {:?}", legal[0]);
            };
            button(h, &card_label(card, en));
        }
    }
    panic!("not enough questions");
}

fn open_window(h: &mut Harness<'_, GrandCoachApp>, en: bool) {
    let t = |de, english| if en { english } else { de };
    button(h, t("Spiel", "Game"));
    button(h, t("Lernverlauf …", "Learning history …"));
    assert!(h.state().learning_open());
}

#[test]
fn history_is_recorded_shown_retained_and_reset_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let temp = Temp::new();
        let file = temp.0.join("data").join("learning-history.json");
        let mut h = app(language, &file);
        open_window(&mut h, en);
        h.get_by_label_contains(t(
            "Noch keine Zählfragen beobachtet",
            "No counting questions observed yet",
        ));
        assert!(
            h.get_by_role_and_label(
                Role::Button,
                t("Lernverlauf zurücksetzen …", "Reset learning history …")
            )
            .accesskit_node()
            .is_disabled()
        );
        button(&mut h, t("Schließen", "Close"));
        assert!(!h.state().learning_open());

        play_questions(&mut h, en, 2);
        let history = h.state().learning_history().clone();
        let summary = history.summary();
        assert_eq!(summary.totals.opportunities, 2);
        assert_eq!(summary.totals.answers, 1);
        assert_eq!(summary.totals.unaided_answers, 1);
        assert_eq!(summary.totals.skips, 1);
        assert_eq!(summary.observations, Observations::Limited);
        // Saved after each event batch.
        assert_eq!(LearningHistory::open_local(&file).unwrap(), history);
        drop(h);

        // Restart: a new window reads the same file.
        let mut h = app(language, &file);
        assert_eq!(h.state().learning_history(), &history);
        open_window(&mut h, en);
        h.get_by_label_contains(t(
            "Bisher erst 2 Frage(n) beobachtet",
            "Only 2 question(s) observed so far",
        ));
        h.get_by_label_contains(t("kein Maß für Können", "not a measure of skill"));
        // One grid row per observed topic, in the series context.
        let rows = history.summary().rows;
        assert!(!rows.is_empty());
        assert_eq!(
            h.query_all_by_label(t("Serie", "Series")).count(),
            rows.len()
        );
        h.get_by_label_contains(file.to_str().unwrap());

        // Cancel keeps everything; confirming deletes it and the file.
        let reset = t("Lernverlauf zurücksetzen …", "Reset learning history …");
        button(&mut h, reset);
        h.get_by_label_contains(t("endgültig löschen", "permanently"));
        button(&mut h, t("Abbrechen", "Cancel"));
        assert_eq!(h.state().learning_history(), &history);
        button(&mut h, reset);
        button(&mut h, t("Verlauf löschen", "Delete history"));
        assert!(h.state().learning_history().is_empty());
        assert!(LearningHistory::open_local(&file).unwrap().is_empty());
        h.get_by_label_contains(t(
            "Noch keine Zählfragen beobachtet",
            "No counting questions observed yet",
        ));

        // Recording continues after the reset with a fresh attempt.
        button(&mut h, t("Schließen", "Close"));
        play_questions(&mut h, en, 1);
        let after = LearningHistory::open_local(&file).unwrap();
        assert_eq!(after.summary().totals.opportunities, 1);
        assert!(after.attempts()[0].id > history.attempts()[0].id);
    }
}

#[test]
fn a_damaged_file_is_kept_until_the_learner_sets_it_aside() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let temp = Temp::new();
        let file = temp.0.join("learning-history.json");
        let damaged = b"{\"format\": 1, \"events\": broken".to_vec();
        fs::write(&file, &damaged).unwrap();
        let mut h = app(language, &file);
        // Events while damaged are kept in memory only; the file is untouched.
        play_questions(&mut h, en, 1);
        assert_eq!(h.state().learning_history().summary().totals.skips, 0);
        assert_eq!(fs::read(&file).unwrap(), damaged);
        open_window(&mut h, en);
        h.get_by_label_contains(t(
            "ist beschädigt oder widersprüchlich",
            "is damaged or inconsistent",
        ));
        h.get_by_label_contains(t("bleibt unverändert", "was left unchanged"));
        button(
            &mut h,
            t(
                "Beschädigte Datei umbenennen und neuen Verlauf beginnen",
                "Rename damaged file and start a new history",
            ),
        );
        let aside = temp.0.join("learning-history.damaged-1.json");
        assert_eq!(fs::read(&aside).unwrap(), damaged);
        h.get_by_label_contains(aside.to_str().unwrap());
        // The in-memory events of this run are now saved.
        let saved = LearningHistory::open_local(&file).unwrap();
        assert_eq!(&saved, h.state().learning_history());
        assert_eq!(saved.summary().totals.opportunities, 1);

        // Unsupported (newer) format: reported as such, also kept.
        let newer = b"{\"format\": 3}".to_vec();
        fs::write(&file, &newer).unwrap();
        let mut h = app(language, &file);
        open_window(&mut h, en);
        h.get_by_label_contains(t(
            "nicht unterstützte Formatversion",
            "unsupported format version",
        ));
        button(&mut h, t("Schließen", "Close"));
        play_questions(&mut h, en, 1);
        assert_eq!(fs::read(&file).unwrap(), newer);
    }
}
