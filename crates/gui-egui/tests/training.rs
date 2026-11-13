// SPDX-License-Identifier: EUPL-1.2

//! Focused counting training through the real widgets in both languages
//! (D-71): enter from the Game menu, choose a topic, read the position,
//! answer, request help, skip, retry, go to the next position and switch
//! topic; the events land in the one learning history as training
//! attempts, and returning shows the untouched live table.

use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language};
use skat_app::{
    AttemptKind, LearningContext, LearningHistory, LearningTopic, Level, Opponents, QuizOutcome,
    SeriesLength, SessionConfig, TableView, TrainingFixture,
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
            "skat-gui-training-{}-{}",
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
            app
        })
}

fn button(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}

fn live(h: &Harness<'_, GrandCoachApp>) -> TableView {
    h.state().live_session().view()
}

fn settle(h: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..2000 {
        if !live(h).opponent_to_move {
            h.run_steps(2);
            return;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not finish");
}

fn fixture(h: &Harness<'_, GrandCoachApp>) -> TrainingFixture {
    h.state().training_view().unwrap().task.fixture
}

fn outcome(h: &Harness<'_, GrandCoachApp>) -> QuizOutcome {
    h.state().training_view().unwrap().feedback.unwrap().outcome
}

/// Raises the answer from 0 with the "+ 1" button.
fn answer(h: &mut Harness<'_, GrandCoachApp>, en: bool, value: u8) {
    let t = |de, english| if en { english } else { de };
    for _ in 0..value {
        button(h, "+ 1");
    }
    h.get_by_label(&format!("{} {value}", t("Deine Antwort:", "Your answer:")));
    button(h, t("Antworten", "Answer"));
}

#[test]
fn training_is_entered_answered_retried_continued_and_left_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let temp = Temp::new();
        let file = temp.0.join("learning-history.json");
        let mut h = app(language, &file);
        settle(&mut h);
        let before = live(&h);
        let sheet = before.sheet.clone();

        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Zähltraining …", "Counting training …"));
        assert!(h.state().training_active());
        h.get_by_label_contains(t("Deine laufende Partie wartet", "your running game waits"));
        h.get_by_label_contains(t("Null hat keine Trümpfe", "Null has no Trumpf"));
        button(&mut h, t("Resttrümpfe üben", "Practise remaining Trumpf"));

        // Declarer of a Karo game: contract, role, tricks, own discard.
        assert_eq!(fixture(&h), TrainingFixture::TrumpfDeclarerFarbspiel);
        h.get_by_label_contains(t(
            "Stellung 1 von 2 · Durchgang 1",
            "Position 1 of 2 · Round 1",
        ));
        h.get_by_label(t(
            "Spiel: Karo · gereizt bis 18 · Alleinspieler: Hinterhand",
            "Game: Diamonds · bid to 18 · declarer: Rearhand",
        ));
        h.get_by_label(t(
            "Du sitzt in Hinterhand und bist Alleinspieler.",
            "You sit in Rearhand and are the declarer.",
        ));
        h.get_by_label(t(
            "Stich 3: Vorhand spielt aus, Hinterhand (du, Alleinspieler) gewinnt.",
            "Trick 3: Forehand leads, Rearhand (you, declarer) wins.",
        ));
        h.get_by_label(t(
            "Dein eigenes Drücken (dir bekannt)",
            "Your own discard (known to you)",
        ));
        // No help before a request.
        assert!(
            h.query_by_label_contains(t("in deinem Drück =", "in your discard ="))
                .is_none()
        );
        answer(&mut h, en, 2);
        assert_eq!(outcome(&h), QuizOutcome::Correct);
        h.get_by_label(t("Richtig: 2.", "Correct: 2."));

        // Retry: same task, round 2, with help requested and then skipped.
        button(&mut h, t("Noch einmal", "Try again"));
        h.get_by_label_contains(t("Durchgang 2", "Round 2"));
        button(&mut h, t("Hilfe einblenden", "Show help"));
        h.get_by_label_contains(t("in deinem Drück = 2", "in your discard = 2"));
        button(&mut h, t("Überspringen", "Skip"));
        assert_eq!(outcome(&h), QuizOutcome::Skipped);
        h.get_by_label_contains(t("Hilfe war in diesem Durchgang", "Help was visible"));

        // Next position: a Grand defender who does not know the Skat.
        button(&mut h, t("Nächste Stellung", "Next position"));
        assert_eq!(fixture(&h), TrainingFixture::TrumpfDefenderGrand);
        h.get_by_label(t("Den Skat kennst du nicht.", "You do not know the Skat."));
        h.get_by_label_contains(t(
            "gegen den Alleinspieler in Hinterhand",
            "against the declarer in Rearhand",
        ));
        answer(&mut h, en, 0);
        assert_eq!(outcome(&h), QuizOutcome::Incorrect { given: 0 });
        h.get_by_label_contains(t("richtig sind 1", "the exact count is 1"));
        h.get_by_label_contains(t(
            "kann einen Teil davon enthalten",
            "may hold some of them",
        ));

        // Other topic: proven voids, named only from public evidence.
        h.get_by_label(t("Nachgewiesene Fehlfarben", "Proven voids"))
            .click();
        h.run_steps(4);
        assert_eq!(fixture(&h), TrainingFixture::VoidsDeclarerGrand);
        button(&mut h, t("Hilfe einblenden", "Show help"));
        h.get_by_label(t(
            "Öffentlich nachgewiesen bei: Mittelhand, Hinterhand",
            "Publicly proven for: Middlehand, Rearhand",
        ));
        answer(&mut h, en, 2);
        assert_eq!(outcome(&h), QuizOutcome::Correct);
        button(&mut h, t("Nächste Stellung", "Next position"));
        assert_eq!(fixture(&h), TrainingFixture::VoidsDefenderFarbspiel);
        answer(&mut h, en, 1);
        h.get_by_label(t(
            "Öffentlich nachgewiesen bei: Mittelhand (Alleinspieler)",
            "Publicly proven for: Middlehand (declarer)",
        ));

        // The learning history shows training rows apart from the series.
        button(&mut h, t("Lernverlauf …", "Learning history …"));
        assert!(h.state().learning_open());
        assert!(
            h.query_all_by_label(t("Zähltraining", "Counting training"))
                .count()
                >= 2
        );
        // The repeat row in the counts grid and in the answer grid.
        assert_eq!(
            h.query_all_by_label(t("Zähltraining, Wiederholung", "Counting training, repeat"))
                .count(),
            2
        );
        button(&mut h, t("Schließen", "Close"));

        // Back to the game: the live table is exactly as it was.
        button(&mut h, t("Zurück zur Partie", "Back to the game"));
        assert!(!h.state().training_active());
        assert!(h.state().training_view().is_none());
        assert_eq!(live(&h), before);
        assert_eq!(live(&h).sheet, sheet);

        // Persisted: five presentations, each its own training attempt.
        let history = LearningHistory::open_local(&file).unwrap();
        assert_eq!(&history, h.state().learning_history());
        let kinds: Vec<_> = history.attempts().iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            [
                AttemptKind::Training { attempt: 1 },
                AttemptKind::Training { attempt: 2 },
                AttemptKind::Training { attempt: 1 },
                AttemptKind::Training { attempt: 1 },
                AttemptKind::Training { attempt: 1 },
            ]
        );
        let summary = history.summary();
        let row = |topic, context| {
            summary
                .rows
                .iter()
                .find(|r| r.topic == topic && r.context == context)
                .unwrap()
                .counts
        };
        let trumpf = row(LearningTopic::TrumpfOutside, LearningContext::Training);
        assert_eq!(
            (trumpf.opportunities, trumpf.answers, trumpf.correct),
            (2, 2, 1)
        );
        let retry = row(
            LearningTopic::TrumpfOutside,
            LearningContext::TrainingRepeat,
        );
        assert_eq!((retry.opportunities, retry.help, retry.skips), (1, 1, 1));
        let voids = row(LearningTopic::ProvenVoids, LearningContext::Training);
        assert_eq!(
            (
                voids.answers,
                voids.correct,
                voids.unaided_correct,
                voids.help
            ),
            (2, 2, 1, 1)
        );
        assert!(
            summary
                .rows
                .iter()
                .all(|r| r.context != LearningContext::Series)
        );

        // Entering again starts at the topic choice with round 1.
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Zähltraining …", "Counting training …"));
        button(&mut h, t("Resttrümpfe üben", "Practise remaining Trumpf"));
        h.get_by_label_contains(t("Durchgang 1", "Round 1"));
        button(&mut h, t("Zurück zur Partie", "Back to the game"));
        assert_eq!(live(&h), before);
    }
}
