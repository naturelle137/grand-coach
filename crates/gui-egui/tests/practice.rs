// SPDX-License-Identifier: EUPL-1.2

//! Same-deal practice through the real windows in both languages: open a
//! saved game, start a fresh attempt, play other legal cards, finish or
//! leave early and return to the untouched live series and file.

use eframe::egui::{self, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language};
use skat_app::{
    Action, Card, GameRecording, Level, Opponents, Phase, PlayerId, SeriesLength, SessionConfig,
    TableView,
};
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
            "skat-gui-practice-{}-{}",
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

/// One defensive human click: pass, no Kontra, else `card` or the first card.
fn act(h: &mut Harness<'_, GrandCoachApp>, en: bool, card: Option<Card>) {
    let legal = view(h).legal_actions;
    let t = |de, english| if en { english } else { de };
    if legal.contains(&Action::Pass) {
        button(h, t("Passe", "Pass"));
    } else if legal.contains(&Action::KeinKontra) {
        button(h, t("Kein Kontra", "No Kontra"));
    } else {
        let card = card.unwrap_or_else(|| match legal[0] {
            Action::Play(c) => c,
            other => panic!("unscripted {other:?}"),
        });
        button(h, &card_label(card, en));
    }
}

/// The human's card in each trick of the original game.
fn original_cards(recording: &GameRecording) -> Vec<Card> {
    let end = recording
        .reconstruct_privileged(recording.actions.len())
        .unwrap();
    let human = PlayerId::HUMAN.seat(recording.context.dealer);
    end.state.seats[0]
        .tricks
        .iter()
        .map(|t| t.cards[(0..3).find(|&i| t.seat_of(i) == human).unwrap()])
        .collect()
}

#[test]
fn practice_starts_from_the_viewer_plays_alternatives_and_returns_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let temp = Temp::new();
        let saved = temp.0.join("game1.json");
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
                    },
                );
                app.set_language(language);
                app.opponent_delay = 0.0;
                app
            });
        // Live game 1 to its end, saved through the menu.
        loop {
            settle(&mut h);
            if view(&h).acting.is_none() {
                break;
            }
            act(&mut h, en, None);
        }
        let recording = h
            .state()
            .session()
            .export_current_recording_privileged()
            .unwrap();
        let originals = original_cards(&recording);
        assert!(originals.len() >= 2, "game 1 has card play");
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
        let bytes = fs::read(&saved).unwrap();
        // Live game 2 with an outstanding human decision.
        button(&mut h, t("Nächstes Spiel", "Next game"));
        settle(&mut h);
        let live = view(&h);
        assert_eq!(live.game_number, 2);
        assert_eq!(live.sheet.records.len(), 1);
        let live_current = h.state().session().export_current_recording_privileged();
        let live_completed = h
            .state()
            .session()
            .completed_recordings_privileged()
            .to_vec();

        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Aufnahme öffnen …", "Open recording …"));
        path(
            &mut h,
            t("Dateipfad:", "File path:"),
            saved.to_str().unwrap(),
        );
        button(&mut h, t("Datei öffnen", "Open file"));
        button(&mut h, t("Dieses Blatt üben", "Practise this deal"));
        assert!(h.state().practicing());
        h.get_by_label_contains(t("ÜBUNG · Spiel 1", "PRACTICE · Game 1"));
        h.get_by_label_contains(t("Übung: kein Eintrag", "Practice: no score sheet entry"));
        let start = view(&h);
        assert!(start.practice && start.sheet.records.is_empty());
        assert_eq!((start.game_number, start.dealer), (1, PlayerId(2)));
        assert_eq!(start.phase, Phase::Reizen);
        // Nothing can be saved, opened or started from the practice table.
        button(&mut h, t("Spiel", "Game"));
        for label in [
            t("Spiel speichern …", "Save game …"),
            t("Aufnahme öffnen …", "Open recording …"),
            t("Neue Serie …", "New series …"),
        ] {
            assert!(
                h.get_by_role_and_label(Role::Button, label)
                    .accesskit_node()
                    .is_disabled(),
                "{label}"
            );
        }
        h.key_press(egui::Key::Escape);
        h.run_steps(2);

        // Different legal cards from the original game.
        let mut deviated = false;
        loop {
            settle(&mut h);
            let v = view(&h);
            if v.acting.is_none() {
                break;
            }
            let plays: Vec<Card> = v
                .legal_actions
                .iter()
                .filter_map(|a| match a {
                    Action::Play(c) => Some(*c),
                    _ => None,
                })
                .collect();
            let other = (!deviated)
                .then(|| originals.get(usize::from(v.tricks_played)))
                .flatten()
                .and_then(|o| plays.iter().copied().find(|c| c != o));
            if let Some(card) = other {
                deviated = true;
                act(&mut h, en, Some(card));
                assert!(view(&h).practice);
                if en {
                    // Leave early, mid-game.
                    break;
                }
            } else {
                act(&mut h, en, None);
            }
        }
        assert!(deviated, "an alternative legal card was played");
        if en {
            assert!(view(&h).acting.is_some(), "left before the end");
        } else {
            h.get_by_label_contains("Übungsspiel beendet.");
            assert!(
                h.query_by_label("Nächstes Spiel").is_none(),
                "practice never deals another game"
            );
            assert!(view(&h).sheet.records.is_empty());
        }
        button(&mut h, t("Zurück zur Serie", "Return to series"));
        assert!(!h.state().practicing());
        assert_eq!(view(&h), live);
        assert_eq!(
            h.state().session().export_current_recording_privileged(),
            live_current
        );
        assert_eq!(
            h.state().session().completed_recordings_privileged(),
            live_completed
        );
        assert_eq!(fs::read(&saved).unwrap(), bytes, "file unchanged");
        assert!(h.query_by_label_contains(t("ÜBUNG", "PRACTICE")).is_none());
        // The live decision is still playable.
        act(&mut h, en, None);
        assert_ne!(view(&h), live);
    }
}
