// SPDX-License-Identifier: EUPL-1.2

//! M7 recording review (D-69) through real widgets in both languages:
//! reopen a completed recording, request the review, cancel and continue,
//! see the moments and their scope, navigate to a moment's decision (the
//! historical comment appears without computing), keep explicit hindsight
//! separately labelled, and leave the live series, the practice flow and
//! the file untouched. A saved in-progress game reports the review as
//! unavailable.
//!
//! The combined M7 flow (issue #12) starts from an explicitly chosen open
//! display: play and save a game, reopen it, navigate to an exact recorded
//! decision, request its historical comment and the whole review while
//! frames are timed, reveal and compare in hindsight, inspect a proven-void
//! lesson in a second open recording, practise the deal and return.

use eframe::egui::{self, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{
    Action, Budget, Card, CoachBasis, GameRecording, Level, Opponents, Phase, PlayerId,
    RecordingEnd, ReplayCoach, ReplayPerspective, ReplayStatus, ReviewStatus, SeriesLength,
    SessionConfig, TableDisplay, TableView,
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
        std::thread::sleep(Duration::from_millis(1));
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

/// One defensive human click: pass, no Kontra, else `card` or the first
/// legal card.
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

/// Picks `option` in the combo box labelled `combo`, then closes its popup.
fn select(h: &mut Harness<'_, GrandCoachApp>, combo: &str, option: &str) {
    // AccessKit's clicks: the box or option may lie outside a scroll view.
    h.get_by_role_and_label(Role::ComboBox, combo)
        .click_accesskit();
    h.run_steps(4);
    h.get_by_label(option).click_accesskit();
    h.run_steps(4);
    h.key_press(egui::Key::Escape);
    h.run_steps(4);
    h.get_by_value(option);
}

/// Requests the selected decision's comment, or computes it again when the
/// review has already cached it. With the small budget the worker may
/// already have answered after these frames; the deterministic stale-result
/// rejection is pinned by the application tests.
fn compute(h: &mut Harness<'_, GrandCoachApp>, en: bool) {
    let (request, again) = if en {
        ("Request comment", "Compute again")
    } else {
        ("Kommentar anfordern", "Erneut berechnen")
    };
    let label = if h.query_by_label(request).is_some() {
        request
    } else {
        again
    };
    h.get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    h.run_steps(1);
}

fn coach_of(h: &mut Harness<'_, GrandCoachApp>) -> skat_app::ReplayCoachView {
    h.state_mut().replay_coach_mut().unwrap().view()
}

/// Runs frames while the review works and returns each frame's duration.
fn timed_review(h: &mut Harness<'_, GrandCoachApp>, deadline: Duration) -> Vec<Duration> {
    let end = Instant::now() + deadline;
    let mut frames = Vec::new();
    while h.state().review_running() {
        assert!(Instant::now() < end, "review did not finish");
        let start = Instant::now();
        h.run_steps(1);
        frames.push(start.elapsed());
        std::thread::sleep(Duration::from_millis(1));
    }
    h.run_steps(4);
    frames
}

fn report(what: &str, frames: &[Duration]) -> Duration {
    let mut sorted = frames.to_vec();
    sorted.sort();
    let slowest = *sorted.last().unwrap();
    eprintln!(
        "{what}: {} frames while the review ran, median {:?}, slowest {:?}",
        frames.len(),
        sorted[sorted.len() / 2],
        slowest
    );
    slowest
}

/// The late Herz fixture recorded as game 2 of an open-display series:
/// the human defends in Hinterhand against player 1 ("Left").
fn late_open_herz() -> GameRecording {
    let mut r = GameRecording::read_local(HERZ.as_bytes()).unwrap();
    r.context.number = 2;
    r.context.dealer = PlayerId(0);
    r.context.session.display = TableDisplay::Open;
    r
}

#[test]
fn open_game_saved_reopened_reviewed_practised_and_returned_in_both_languages() {
    for language in Language::ALL {
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let i18n = I18n::new(language);
        let temp = Temp::new();
        let saved = temp.0.join("open-game.json");
        let other = temp.0.join("late-herz-open.json");
        late_open_herz().save_local_new(&other).unwrap();
        let other_bytes = fs::read(&other).unwrap();
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
                app
            });

        // 1. The ordinary open display, explicitly chosen: not God Mode.
        button(&mut h, t("Spiel", "Game"));
        button(&mut h, t("Neue Serie …", "New series …"));
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Offene Anzeige: alle Hände und den Skat zeigen",
                "Open display: show all hands and the Skat",
            ),
        )
        .click();
        h.run_steps(4);
        button(&mut h, t("Starten", "Start"));
        let start = view(&h);
        assert_eq!(start.config.display, TableDisplay::Open);
        assert_eq!(start.config.opponents, Opponents::Levels([Level::Easy; 2]));
        assert_eq!(start.coach_basis, CoachBasis::OwnView);
        assert!(start.open_cards.is_some());
        h.state_mut().set_coach_budget(SMALL);

        // 2. Game 1 by clicks to its end, saved through the menu.
        loop {
            settle(&mut h);
            if view(&h).acting.is_none() {
                break;
            }
            act(&mut h, en, None);
        }
        let game1 = h
            .state()
            .session()
            .export_current_recording_privileged()
            .unwrap();
        assert!(matches!(game1.end, RecordingEnd::Finished(_)));
        assert_eq!(game1.context.session.display, TableDisplay::Open);
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
        assert_eq!(GameRecording::open_local(&saved).unwrap(), game1);
        let bytes = fs::read(&saved).unwrap();

        // 3. Live game 2 waits for the human; nothing below may touch it.
        button(&mut h, t("Nächstes Spiel", "Next game"));
        settle(&mut h);
        let live = view(&h);
        assert_eq!((live.game_number, live.acting), (2, Some(PlayerId::HUMAN)));
        assert!(live.open_cards.is_some());
        let live_current = h.state().session().export_current_recording_privileged();
        let live_completed = h
            .state()
            .session()
            .completed_recordings_privileged()
            .to_vec();
        let untouched = |h: &Harness<'_, GrandCoachApp>| {
            assert_eq!(view(h), live);
            assert_eq!(
                h.state().session().export_current_recording_privileged(),
                live_current
            );
            assert_eq!(
                h.state().session().completed_recordings_privileged(),
                live_completed
            );
            assert_eq!(fs::read(&saved).unwrap(), bytes);
            assert_eq!(fs::read(&other).unwrap(), other_bytes);
        };

        // Independent expectations from the application interface.
        let mut expected = ReplayCoach::open(&game1, SMALL).unwrap();
        let decisions = expected.decisions().to_vec();
        let cards: Vec<usize> = (0..decisions.len())
            .filter(|&i| decisions[i].phase == Phase::Spiel && decisions[i].options > 1)
            .collect();
        assert!(cards.len() >= 2, "the human chose cards in game 1");
        let (early, target) = (cards[0], *cards.last().unwrap());
        expected.select(Some(target));
        let job = expected.request().unwrap();
        expected.finish(job.run(None)).unwrap();
        let ReplayStatus::Ready(historical) = expected.view().status else {
            panic!("no historical comment");
        };
        let target_context = expected.view().context.unwrap();
        expected.select(None);
        assert!(expected.request_review());
        while let Some(job) = expected.review_job() {
            expected.finish_review(job.run(None)).unwrap();
        }
        let review = expected.review_status();
        let ReviewStatus::Done(summary) = &review else {
            panic!("review not done");
        };
        assert!(summary.moments.len() <= 3);

        // 4. Reopen; trick navigation backwards and forwards.
        let open = |h: &mut Harness<'_, GrandCoachApp>, file: &PathBuf| {
            button(h, t("Spiel", "Game"));
            button(h, t("Aufnahme öffnen …", "Open recording …"));
            path(h, t("Dateipfad:", "File path:"), file.to_str().unwrap());
            button(h, t("Datei öffnen", "Open file"));
        };
        open(&mut h, &saved);
        h.get_by_label(t("Aufnahme — nur lesen", "Recording — read only"));
        let boundaries = game1.trick_boundaries().unwrap();
        let last = boundaries.len() - 1;
        let at = |n: usize| {
            if en {
                format!("After {} actions", boundaries[n])
            } else {
                format!("Nach {} Aktionen", boundaries[n])
            }
        };
        h.get_by_label(&at(last));
        button(&mut h, t("Zurück", "Previous"));
        button(&mut h, t("Zurück", "Previous"));
        h.get_by_label(&at(last - 2));
        button(&mut h, t("Weiter", "Next"));
        h.get_by_label(&at(last - 1));
        untouched(&h);

        // 5. The exact pre-action decision, reached backwards and forwards.
        let combo = t("Entscheidung", "Decision");
        let label = |i: usize| coach::recorded_decision(&i18n, &decisions[i]);
        select(&mut h, combo, &label(target));
        select(&mut h, combo, &label(early));
        select(&mut h, combo, &label(target));
        let shown = coach_of(&mut h);
        assert_eq!(shown.selected, Some(target));
        assert_eq!(shown.context.as_ref(), Some(&target_context));
        assert_eq!(target_context.actions as usize, decisions[target].prefix);
        h.get_by_label(t(
            "Der Tisch vor deinem aufgenommenen Zug",
            "The table before your recorded move",
        ));

        // 6. The historical comment: chosen card and legal alternative,
        // exactly the closed-view comment of the application interface.
        button(&mut h, t("Kommentar anfordern", "Request comment"));
        wait(&mut h);
        assert!(has(&h, t("Du hast", "You chose")));
        assert!(has(&h, t("Verglichen mit", "Compared with")) == historical.alternative.is_some());
        assert!(has(
            &h,
            t(
                "Sicht: damaliger Wissensstand",
                "Perspective: what you knew then"
            )
        ));
        let ReplayStatus::Ready(shown) = coach_of(&mut h).status else {
            panic!("no comment shown");
        };
        assert_eq!(*shown, *historical);
        assert_eq!(shown.perspective, ReplayPerspective::Historical);
        assert!(!shown.review.basis.known);
        assert_eq!(shown.review.chosen.action, decisions[target].action);

        // 7. The whole review, with frames timed while it works.
        button(&mut h, request_label(en));
        assert!(h.state().review_running());
        let frames = timed_review(&mut h, Duration::from_secs(120));
        let slowest = report(&format!("{language:?} small-budget review"), &frames);
        assert!(frames.len() >= 2, "frames ran while the review worked");
        assert!(slowest < Duration::from_millis(500), "{slowest:?}");
        assert_eq!(coach_of(&mut h).review, review);
        assert_eq!(count(&h, show_label(en)), summary.moments.len());
        // The comment of the selected decision is still the same.
        assert_eq!(coach_of(&mut h).status, ReplayStatus::Ready(shown.clone()));

        // 8. Revealing every card changes only the display.
        h.get_by_role_and_label(
            Role::CheckBox,
            t(
                "Alle verdeckten Karten und den Skat aufdecken",
                "Reveal all concealed cards and the Skat",
            ),
        )
        .click();
        h.run_steps(4);
        assert!(has(&h, t("Vollständige Information", "Full information")));
        let revealed = coach_of(&mut h);
        assert_eq!(revealed.review, review);
        assert_eq!(revealed.status, ReplayStatus::Ready(shown.clone()));
        assert_eq!(revealed.perspective, ReplayPerspective::Historical);

        // 9. Hindsight only on request, labelled, leaving the review alone;
        // a mode change while computing accepts only the new mode's result.
        select(&mut h, combo, &label(early));
        compute(&mut h, en);
        button(&mut h, hindsight_label(en));
        wait(&mut h);
        let ReplayStatus::Ready(known) = coach_of(&mut h).status else {
            panic!("no hindsight comment");
        };
        assert_eq!(known.perspective, ReplayPerspective::Hindsight);
        assert!(known.review.basis.known);
        assert!(has(
            &h,
            t(
                "RÜCKBLICK: alle Karten bekannt",
                "HINDSIGHT: all cards known"
            )
        ));
        assert_eq!(coach_of(&mut h).review, review);
        select(&mut h, combo, &label(target));
        assert_eq!(coach_of(&mut h).perspective, ReplayPerspective::Historical);
        untouched(&h);

        // 10. Another open recording replaces this one while a comment is
        // computed: nothing of the old recording arrives.
        compute(&mut h, en);
        open(&mut h, &other);
        assert!(!h.state().replay_computing());
        h.run_steps(20);
        assert_eq!(coach_of(&mut h).status, ReplayStatus::NotSelected);
        assert!(matches!(coach_of(&mut h).review, ReviewStatus::Idle { .. }));

        // 11. The closed-evidence lesson in that recording: in trick 3 the
        // declarer ("Left") played S9 to CA; at the human's trick-4 card it
        // is proven, while the declarer's Pik void is not yet visible.
        let late = ReplayCoach::open(&late_open_herz(), SMALL).unwrap();
        let trick4 = late
            .decisions()
            .iter()
            .position(|d| d.prefix == 16)
            .unwrap();
        select(
            &mut h,
            combo,
            &coach::recorded_decision(&i18n, &late.decisions()[trick4]),
        );
        compute(&mut h, en);
        wait(&mut h);
        h.get_by_label(t("Was sicher feststeht", "What is certain"))
            .click();
        h.run_steps(4);
        assert!(has(
            &h,
            t(
                "Stich 3: Links hat Pik-Neun auf Kreuz-Ass gespielt, also nicht Kreuz bedient.",
                "Trick 3: Left played Nine of Spades to Ace of Clubs and so did not follow Clubs."
            )
        ));
        assert!(!has(&h, t("Stich 4:", "Trick 4:")));
        untouched(&h);

        // 12. Practise the saved open game: the dropped comment never
        // arrives, the attempt keeps the open display, other legal cards
        // are possible, and returning restores the untouched series.
        open(&mut h, &saved);
        select(&mut h, combo, &label(target));
        compute(&mut h, en);
        button(&mut h, t("Dieses Blatt üben", "Practise this deal"));
        assert!(h.state().practicing());
        assert!(!h.state().replay_computing());
        let attempt = view(&h);
        assert!(attempt.practice && attempt.sheet.records.is_empty());
        assert_eq!(attempt.config.display, TableDisplay::Open);
        assert!(attempt.open_cards.is_some());
        // The human's card in each trick of the original game.
        let human = PlayerId::HUMAN.seat(game1.context.dealer);
        let end = game1.reconstruct_privileged(game1.actions.len()).unwrap();
        let original: Vec<Card> = end.state.seats[0]
            .tricks
            .iter()
            .map(|t| t.cards[(0..3).find(|&i| t.seat_of(i) == human).unwrap()])
            .collect();
        loop {
            settle(&mut h);
            let v = view(&h);
            assert!(v.acting.is_some(), "no other legal card found");
            let other = original.get(usize::from(v.tricks_played)).and_then(|o| {
                v.legal_actions.iter().find_map(|a| match a {
                    Action::Play(c) if c != o => Some(*c),
                    _ => None,
                })
            });
            act(&mut h, en, other);
            if other.is_some() {
                break;
            }
        }
        assert!(view(&h).practice);
        button(&mut h, t("Zurück zur Serie", "Return to series"));
        assert!(!h.state().practicing());
        untouched(&h);
        // The live decision is still playable.
        act(&mut h, en, None);
        assert_ne!(view(&h), live);
    }
}

fn request_label(en: bool) -> &'static str {
    if en {
        "Review my decisions"
    } else {
        "Meine Entscheidungen auswerten"
    }
}

fn show_label(en: bool) -> &'static str {
    if en {
        "Show the table before this move"
    } else {
        "Am Tisch vor diesem Zug zeigen"
    }
}

fn hindsight_label(en: bool) -> &'static str {
    if en {
        "Hindsight: compare with all cards"
    } else {
        "Rückblick: mit allen Karten vergleichen"
    }
}

/// Frame responsiveness while the review works with the production budget
/// and limits (D-69) on a reopened open-display recording.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn frames_stay_responsive_while_the_production_budget_review_works() {
    let temp = Temp::new();
    let file = temp.0.join("late-herz-open.json");
    late_open_herz().save_local_new(&file).unwrap();
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
                    display: TableDisplay::Open,
                },
            );
            app.opponent_delay = 0.0;
            app
        });
    settle(&mut h);
    let live = view(&h);
    assert_eq!(h.state().session().coach_budget(), Budget::INTERACTIVE);
    button(&mut h, "Spiel");
    button(&mut h, "Aufnahme öffnen …");
    path(&mut h, "Dateipfad:", file.to_str().unwrap());
    button(&mut h, "Datei öffnen");
    button(&mut h, request_label(false));
    let started = Instant::now();
    let frames = timed_review(&mut h, skat_app::REPLAY_REVIEW_TIME_LIMIT * 2);
    let elapsed = started.elapsed();
    let slowest = report("production-budget review", &frames);
    let ReviewStatus::Done(summary) = coach_of(&mut h).review else {
        panic!("review not done");
    };
    eprintln!(
        "production-budget review: {elapsed:?}, {} analysed, {} forced, {} moments",
        summary.analysed,
        summary.forced,
        summary.moments.len()
    );
    assert!(elapsed < skat_app::REPLAY_REVIEW_TIME_LIMIT + Duration::from_secs(5));
    assert!(frames.len() as u32 >= summary.analysed);
    assert!(slowest < Duration::from_millis(500), "{slowest:?}");
    assert_eq!(view(&h), live);
}
