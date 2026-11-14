// SPDX-License-Identifier: EUPL-1.2

//! Renders screenshots of the real egui frontend headlessly (D-45).
//!
//! ```sh
//! cargo run --locked -p gui-egui --example screenshots -- OUT_DIR [SCENARIO...]
//! ```
//!
//! Without scenario names, all scenarios are rendered. Each one writes
//! `OUT_DIR/<name>.png`. egui_kittest renders with wgpu and prefers software
//! rasterisers (lavapipe on Linux, WARP on Windows), so no display is needed.
//! The human's decisions are clicks on widgets found by their accessible
//! labels, as in `tests/gui.rs`. Add a scenario when a frontend change needs
//! its own picture.

use eframe::egui::{self, Key, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use gui_egui::{Deck, GrandCoachApp, Language, Theme, coach, i18n::I18n};
use skat_app::coach_model::{Concept, Judgement, LessonContent};
use skat_app::{
    Action, Advice, Budget, Card, CoachLevel, CoachSettings, Debriefing, Level, Opponents, Pending,
    Rank, SeriesLength, SessionConfig, TableDisplay, TableView,
};

const SIZE: [f32; 2] = [1280.0, 820.0];

/// The smallest window `main` allows; scenarios whose name ends in
/// `-narrow` shrink to it after reaching their state.
const NARROW: [f32; 2] = [960.0, 640.0];

type Scenario = fn(&mut Harness<'_, GrandCoachApp>);

/// Series seed of every scenario not in [`SEEDS`].
const SEED: u64 = 21;

/// Scenarios whose state needs another series seed: with seed 21 the
/// human never plays second to the declarer's lead in the two games.
const SEEDS: &[(&str, u64)] = &[("coach-lesson-overtake", 1)];

/// Size of the `table-*` scenarios of issue #134 by name suffix; others
/// use [`SIZE`].
fn table_size(name: &str) -> Option<[f32; 2]> {
    if !name.starts_with("table-") {
        return None;
    }
    Some(if name.ends_with("-1600") {
        [1600.0, 1000.0]
    } else if name.ends_with("-960") {
        NARROW
    } else {
        SIZE
    })
}

/// Series seed of the `table-*` scenarios: its German cast is
/// Friedrich-Wilhelm and Hildegard, its English one James and Bartholomew,
/// so the longest name of each pool is on the table (D-72); the human wins
/// the first bidding at 24 against two Easy opponents.
const LONG_NAMES: u64 = 1116;

/// Scenarios in God Mode (D-62); all others play against two Easy
/// opponents.
const GOD_MODE: &[&str] = &[
    "table-god-960",
    "god-mode",
    "god-mode-german",
    "god-mode-hidden",
    "god-mode-trick",
    "god-mode-coach",
];

/// Scenarios with an open display (D-68) against two Easy opponents.
const OPEN_DISPLAY: &[&str] = &[
    "table-open-960",
    "open-display",
    "open-display-en",
    "open-display-trick",
    "open-display-coach",
    "open-display-proven-void",
    "open-display-proven-void-en",
];

const SCENARIOS: &[(&str, Scenario)] = &[
    ("coach-jump-context", coach_jump_context),
    ("coach-jump-context-en", coach_jump_context_en),
    ("guided-orient", guided_orient),
    ("guided-act", guided_act),
    ("guided-memory", guided_memory),
    ("guided-observe", guided_observe),
    ("guided-complete", guided_complete),
    ("guided-figures", guided_figures),
    ("guided-figures-en", guided_figures_en),
    ("guided-orient-en", guided_orient_en),
    ("guided-act-en", guided_act_en),
    ("guided-observe-en", guided_observe_en),
    ("guided-complete-en", guided_complete_en),
    ("guided-orient-narrow", guided_orient),
    ("launch", launch),
    ("recording", recording),
    ("recording-final", recording_final),
    ("recording-en", recording_en),
    ("recording-error", recording_error),
    ("recording-error-en", recording_error_en),
    ("mechanism-responses", mechanism_responses),
    ("mechanism-responses-en", mechanism_responses_en),
    ("mechanism-trends", mechanism_trends),
    ("mechanism-trends-en", mechanism_trends_en),
    ("mechanism-trends-narrow", mechanism_trends_narrow),
    ("learning-empty", learning_empty),
    ("learning-empty-en", learning_empty_en),
    ("learning-history", learning_history),
    ("learning-history-en", learning_history_en),
    ("learning-reset", learning_reset),
    ("learning-categories", learning_categories),
    ("learning-categories-en", learning_categories_en),
    ("learning-categories-narrow", learning_categories),
    ("learning-compare", learning_compare),
    ("learning-compare-en", learning_compare_en),
    ("learning-damaged", learning_damaged),
    ("learning-damaged-en", learning_damaged_en),
    ("validation-entry", validation_entry),
    ("validation-entry-en", validation_entry_en),
    ("validation-form", validation_form),
    ("validation-form-en", validation_form_en),
    ("focus-count", focus_count),
    ("focus-count-en", focus_count_en),
    ("focus-changed", focus_changed),
    ("focus-changed-en", focus_changed_en),
    ("focus-help", focus_help),
    ("focus-help-en", focus_help_en),
    ("focus-mechanism", focus_mechanism),
    ("focus-mechanism-en", focus_mechanism_en),
    ("focus-feedback", focus_feedback),
    ("focus-feedback-en", focus_feedback_en),
    ("focus-boundary", focus_boundary),
    ("focus-boundary-en", focus_boundary_en),
    ("focus-practice", focus_practice),
    ("focus-practice-en", focus_practice_en),
    ("focus-void-narrow", focus_void),
    ("focus-void-en-narrow", focus_void_en),
    ("training-topics", training_topics),
    ("training-topics-en", training_topics_en),
    ("training-trumpf", training_trumpf),
    ("training-trumpf-en", training_trumpf_en),
    ("training-trumpf-narrow", training_trumpf),
    ("training-feedback", training_feedback),
    ("training-retry-help", training_retry_help),
    ("training-defender", training_defender),
    ("training-defender-en", training_defender_en),
    ("training-voids", training_voids),
    ("training-voids-en", training_voids_en),
    ("training-boundary", training_boundary),
    ("training-history", training_history),
    ("training-history-en", training_history_en),
    ("training-return", training_return),
    ("recording-narrow", recording),
    ("replay-before", replay_before),
    ("replay-historical", replay_historical),
    ("replay-historical-en", replay_historical_en),
    ("replay-hindsight", replay_hindsight),
    ("replay-hindsight-en", replay_hindsight_en),
    ("replay-none", replay_none),
    ("replay-none-en", replay_none_en),
    ("open-display-proven-void", open_display_proven_void),
    ("open-display-proven-void-en", open_display_proven_void_en),
    ("review-running", review_running),
    ("review-cancelled", review_cancelled),
    ("review-cancelled-en", review_cancelled_en),
    ("review-moments", review_moments),
    ("review-moments-en", review_moments_en),
    ("review-moments-narrow", review_moments),
    ("review-navigate", review_navigate),
    ("review-navigate-en", review_navigate_en),
    ("practice", practice),
    ("practice-play", practice_play),
    ("practice-play-en", practice_play_en),
    ("practice-finished", practice_finished),
    ("practice-return", practice_return),
    ("practice-return-en", practice_return_en),
    ("bidding", bidding),
    ("trick", trick),
    ("neural-trick", neural_trick),
    ("trick-complete", trick_complete),
    ("result", result),
    ("settings", settings),
    ("new-series", new_series),
    ("new-series-seed", new_series_seed),
    ("coach", coach),
    ("coach-situation", coach_situation),
    ("coach-figures", coach_figures),
    ("coach-no-figures", coach_no_figures),
    ("coach-note", coach_note),
    ("coach-lesson", coach_lesson),
    ("coach-lesson-review", coach_lesson_review),
    ("coach-lesson-review-en", coach_lesson_review_en),
    ("coach-lesson-trumpf", coach_lesson_trumpf),
    ("coach-lesson-abstich", coach_lesson_abstich),
    ("coach-lesson-abstich-en", coach_lesson_abstich_en),
    ("coach-lesson-overtake", coach_lesson_overtake),
    ("coach-lesson-structure", coach_lesson_structure),
    (
        "coach-lesson-structure-review",
        coach_lesson_structure_review,
    ),
    ("coach-lesson-value-review", coach_lesson_value_review),
    ("coach-lesson-value-review-en", coach_lesson_value_review_en),
    ("coach-debriefing", coach_debriefing),
    ("coach-debriefing-details", coach_debriefing_details),
    ("coach-debriefing-en", coach_debriefing_en),
    ("coach-debriefing-narrow", coach_debriefing_details),
    ("coach-debriefing-en-narrow", coach_debriefing_en),
    ("coach-debriefing-history", coach_debriefing_history),
    ("coach-debriefing-history-narrow", coach_debriefing_history),
    ("coach-debriefing-history-en", coach_debriefing_history_en),
    (
        "coach-debriefing-history-en-narrow",
        coach_debriefing_history_en,
    ),
    ("coach-warning", coach_warning),
    ("coach-focus", coach_focus),
    ("coach-no-lesson", coach_no_lesson),
    ("coach-focus-debriefing", coach_focus_debriefing),
    ("coach-focus-debriefing-en", coach_focus_debriefing_en),
    ("retrieval-question", retrieval_question),
    ("retrieval-question-en", retrieval_question_en),
    ("retrieval-question-narrow", retrieval_question),
    ("retrieval-request", retrieval_request),
    ("retrieval-help", retrieval_help),
    ("retrieval-help-en", retrieval_help_en),
    ("retrieval-visible", retrieval_visible),
    ("retrieval-void", retrieval_void),
    ("retrieval-void-en", retrieval_void_en),
    ("counting-aid", counting_aid),
    ("counting-aid-en", counting_aid_en),
    ("quiz-question", quiz_question),
    ("quiz-question-en", quiz_question_en),
    ("quiz-feedback", quiz_feedback),
    ("quiz-feedback-en", quiz_feedback_en),
    ("quiz-question-narrow", quiz_question),
    ("god-mode", god_mode),
    ("god-mode-german", god_mode_german),
    ("god-mode-hidden", god_mode_hidden),
    ("god-mode-trick", god_mode_trick),
    ("god-mode-coach", god_mode_coach),
    ("open-display-new-series", open_display_new_series),
    ("open-display-new-series-en", open_display_new_series_en),
    ("open-display", open_display),
    ("open-display-en", open_display_en),
    ("open-display-trick", open_display_trick),
    ("open-display-coach", open_display_coach),
    ("table-bidding-1600", table_bidding_1600),
    ("table-discard-1600", table_discard_1600),
    ("table-trick-1600", table_trick_1600),
    ("table-result-1600", table_result_1600),
    ("table-bidding", table_bidding),
    ("table-discard", table_discard),
    ("table-announce", table_announce),
    ("table-trick", table_trick),
    ("table-augen", table_augen),
    ("table-result", table_result),
    ("table-series-end", table_series_end),
    ("table-bidding-960", table_bidding_960),
    ("table-discard-960", table_discard_960),
    ("table-announce-960", table_announce_960),
    ("table-trick-960", table_trick_960),
    ("table-augen-960", table_augen_960),
    ("table-result-960", table_result_960),
    ("table-series-end-960", table_series_end_960),
    ("table-en-960", table_en_960),
    ("table-german-deck-960", table_german_deck_960),
    ("table-french2-en", table_french2_en),
    ("table-large-text-960", table_large_text_960),
    ("table-large-text-en-960", table_large_text_en_960),
    ("table-zoom-1280", table_zoom_1280),
    ("table-mephisto-960", table_mephisto_960),
    ("table-goldfish-960", table_goldfish_960),
    ("table-god-960", table_god_960),
    ("table-open-960", table_open_960),
    ("table-neural-960", table_neural_960),
    ("table-coach-off-960", table_coach_off_960),
];

const AID: skat_app::AssistSettings = skat_app::AssistSettings {
    counting_aid: true,
    quiz: false,
    retrieval: false,
    assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
};

const AID_AND_QUIZ: skat_app::AssistSettings = skat_app::AssistSettings {
    counting_aid: true,
    quiz: true,
    retrieval: false,
    assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
};

/// Focused question with all auxiliary answer sources withheld.
fn retrieval_question(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().set_assist(skat_app::AssistSettings {
        retrieval: true,
        ..AID_AND_QUIZ
    });
    play_until(h, |v, _| v.assist.quiz.question.is_some());
}

fn retrieval_question_en(h: &mut Harness<'_, GrandCoachApp>) {
    retrieval_question(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn retrieval_request(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().set_assist(skat_app::AssistSettings {
        retrieval: true,
        assistance: skat_app::AssistanceLevel::RevealOnRequest,
        ..AID_AND_QUIZ
    });
    play_until(h, |v, _| v.assist.quiz.question.is_some());
}

fn retrieval_help(h: &mut Harness<'_, GrandCoachApp>) {
    retrieval_question(h);
    click(h, Role::Button, "Hilfe einblenden");
}

fn retrieval_help_en(h: &mut Harness<'_, GrandCoachApp>) {
    retrieval_help(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn retrieval_visible(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().set_assist(skat_app::AssistSettings {
        retrieval: true,
        assistance: skat_app::AssistanceLevel::VisibleAid,
        ..AID_AND_QUIZ
    });
    play_until(h, |v, _| v.assist.quiz.question.is_some());
}

/// Show the new public-void question through the real seeded selection.
fn retrieval_void(h: &mut Harness<'_, GrandCoachApp>) {
    retrieval_question(h);
    for _ in 0..20 {
        if matches!(
            view(h).assist.quiz.question,
            Some(skat_app::QuestionKind::ProvenVoids { .. })
        ) {
            click(h, Role::Button, "Hilfe einblenden");
            return;
        }
        click(h, Role::Button, "Überspringen");
        play_until(h, |v, _| v.assist.quiz.question.is_some());
    }
    panic!("no public-void retrieval opportunity");
}

fn retrieval_void_en(h: &mut Harness<'_, GrandCoachApp>) {
    retrieval_void(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// A fresh learning-history file for a scenario, under the temp directory.
fn learning_file(
    h: &mut Harness<'_, GrandCoachApp>,
    name: &str,
    content: Option<&[u8]>,
) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("grand-coach-screenshots")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scenario directory");
    let file = dir.join("learning-history.json");
    if let Some(content) = content {
        std::fs::write(&file, content).expect("write scenario file");
    }
    h.state_mut().use_learning_file(file.clone());
    file
}

fn open_learning(h: &mut Harness<'_, GrandCoachApp>) {
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Lernverlauf …");
}

/// The learning history before any counting question (D-70).
fn learning_empty(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(h, "learning-empty", None);
    open_learning(h);
}

fn learning_empty_en(h: &mut Harness<'_, GrandCoachApp>) {
    learning_empty(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// Three real focused-retrieval questions at the table: answered after
/// requested help, answered unaided, skipped; then the overview.
fn learning_history(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(h, "learning-history", None);
    h.state_mut().set_assist(skat_app::AssistSettings {
        retrieval: true,
        ..AID_AND_QUIZ
    });
    for step in 0..3 {
        play_until(h, |v, _| v.assist.quiz.question.is_some());
        match step {
            0 => {
                click(h, Role::Button, "Hilfe einblenden");
                click(h, Role::Button, "Antworten");
            }
            1 => click(h, Role::Button, "Antworten"),
            _ => click(h, Role::Button, "Überspringen"),
        }
    }
    let totals = h.state().learning_history().summary().totals;
    assert_eq!((totals.opportunities, totals.help), (3, 1));
    open_learning(h);
}

fn learning_history_en(h: &mut Harness<'_, GrandCoachApp>) {
    learning_history(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// The confirmation step before deleting the history.
fn learning_reset(h: &mut Harness<'_, GrandCoachApp>) {
    learning_history(h);
    h.get_by_role_and_label(Role::Button, "Lernverlauf zurücksetzen …")
        .scroll_to_me();
    h.run_steps(4);
    click(h, Role::Button, "Lernverlauf zurücksetzen …");
    h.get_by_label("Den gesamten Lernverlauf endgültig löschen? Spiele, Aufnahmen und Punkte bleiben unberührt.");
}

/// A format 2 file as the PR #137 build wrote it: a wrong and an exact
/// Trumpf answer in a series, a training task skipped after help.
const LEGACY_FORMAT_2: &str = r#"{"format":2,"next_attempt":3,"attempts":[{"id":1,"kind":"Series"},{"id":2,"kind":{"Training":{"attempt":1}}}],"events":[{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":{"Answer":{"given":4,"correct":false}},"assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":6},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":6},"question":"TrumpfOutside","action":{"Answer":{"given":1,"correct":true}},"assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":"Opportunity","assistance":"RevealOnRequest","assisted":false}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":"Help","assistance":"RevealOnRequest","assisted":true}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":{"Skip":{"disabled":false}},"assistance":"RevealOnRequest","assisted":true}}]}"#;

/// A legacy format 2 history upgraded in place by real answers at the
/// table: three focused questions (help then the default answer, the
/// default answer unaided, skip)
/// beside the unversioned events; then the answer categories.
fn learning_categories(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(h, "learning-categories", Some(LEGACY_FORMAT_2.as_bytes()));
    h.state_mut().set_assist(skat_app::AssistSettings {
        retrieval: true,
        ..AID_AND_QUIZ
    });
    for step in 0..3 {
        play_until(h, |v, _| v.assist.quiz.question.is_some());
        match step {
            0 => {
                click(h, Role::Button, "Hilfe einblenden");
                click(h, Role::Button, "Antworten");
            }
            1 => click(h, Role::Button, "Antworten"),
            _ => click(h, Role::Button, "Überspringen"),
        }
    }
    let summary = h.state().learning_history().summary();
    assert_eq!(summary.totals.opportunities, 6);
    assert_eq!(summary.provenance.len(), 2);
    open_learning(h);
    h.get_by_label("Antworten nach beobachtetem Ergebnis")
        .scroll_to_me();
    h.run_steps(4);
}

fn learning_categories_en(h: &mut Harness<'_, GrandCoachApp>) {
    learning_categories(h);
    english(h);
}

/// The legacy file plus 20 focused Trumpf answers in a new series attempt
/// (1-10 one too high, 11-20 exact, 11 and 12 after help): the earlier
/// and most recent windows, the sparse voids topic and the provenance.
fn learning_compare(h: &mut Harness<'_, GrandCoachApp>) {
    use skat_app::{
        AssistanceLevel, AttemptKind, LearningHistory, LessonFamily, QuestionId, QuestionKind,
        RetrievalAction, RetrievalEvent,
    };
    let mut history = LearningHistory::read_local(LEGACY_FORMAT_2.as_bytes()).expect("format 2");
    let attempt = history.begin_attempt(AttemptKind::Series).expect("attempt");
    for i in 1..=20u8 {
        let assisted = matches!(i, 11 | 12);
        let e = |action, assisted| RetrievalEvent {
            id: QuestionId {
                series: 1,
                game: u32::from(i),
                trick: 3,
            }
            .into(),
            question: QuestionKind::TrumpfOutside,
            action,
            assistance: AssistanceLevel::RevealOnRequest,
            assisted,
            lesson: Some(LessonFamily::FocusedRetrieval.current()),
        };
        let given = if i <= 10 { 4 } else { 3 };
        let mut events = vec![e(RetrievalAction::Opportunity, false)];
        if assisted {
            events.push(e(RetrievalAction::Help, true));
        }
        events.push(e(
            RetrievalAction::Answer {
                given,
                correct: given == 3,
                expected: Some(3),
            },
            assisted,
        ));
        history.record(attempt, &events).expect("record");
    }
    let file = learning_file(h, "learning-compare", None);
    history.save_local(&file).expect("save scenario file");
    h.state_mut().use_learning_file(file);
    open_learning(h);
    h.get_by_label("Frühere und letzte Antworten")
        .scroll_to_me();
    h.run_steps(4);
}

fn learning_compare_en(h: &mut Harness<'_, GrandCoachApp>) {
    learning_compare(h);
    english(h);
}

/// A damaged file is reported, kept and offered to be set aside.
fn learning_damaged(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(
        h,
        "learning-damaged",
        Some(b"{\"format\": 1, \"events\": broken"),
    );
    open_learning(h);
}

fn learning_damaged_en(h: &mut Harness<'_, GrandCoachApp>) {
    learning_damaged(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn english(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn plus(h: &mut Harness<'_, GrandCoachApp>, n: u8) {
    for _ in 0..n {
        click(h, Role::Button, "+ 1");
    }
}

/// The topic choice of the focused counting training (D-71), entered
/// from the Game menu during a live game.
fn training_topics(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(h, "training", None);
    play_until(h, |v, _| v.tricks_played >= 1);
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Zähltraining …");
    assert!(h.state().training_active());
}

fn training_topics_en(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    english(h);
}

/// The Karo declarer task: hand, own discard, three tricks, the question.
fn training_trumpf(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Resttrümpfe üben");
}

fn training_trumpf_en(h: &mut Harness<'_, GrandCoachApp>) {
    training_trumpf(h);
    english(h);
}

/// The exact answer with its derivation.
fn training_feedback(h: &mut Harness<'_, GrandCoachApp>) {
    training_trumpf(h);
    plus(h, 2);
    click(h, Role::Button, "Antworten");
}

/// A deliberate retry with help requested: round 2, assisted.
fn training_retry_help(h: &mut Harness<'_, GrandCoachApp>) {
    training_feedback(h);
    click(h, Role::Button, "Noch einmal");
    click(h, Role::Button, "Hilfe einblenden");
}

/// The Grand defender who does not know the Skat, answered wrongly.
fn training_defender(h: &mut Harness<'_, GrandCoachApp>) {
    training_feedback(h);
    click(h, Role::Button, "Nächste Stellung");
    click(h, Role::Button, "Antworten");
}

fn training_defender_en(h: &mut Harness<'_, GrandCoachApp>) {
    training_defender(h);
    english(h);
}

/// Proven voids from the declarer's seat with help naming the public proof.
fn training_voids(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Nachgewiesene Fehlfarben üben");
    click(h, Role::Button, "Hilfe einblenden");
}

fn training_voids_en(h: &mut Harness<'_, GrandCoachApp>) {
    training_voids(h);
    english(h);
}

/// The defender right after the declarer failed to follow Herz.
fn training_boundary(h: &mut Harness<'_, GrandCoachApp>) {
    training_voids(h);
    plus(h, 2);
    click(h, Role::Button, "Antworten");
    click(h, Role::Button, "Nächste Stellung");
    plus(h, 1);
    click(h, Role::Button, "Antworten");
}

/// The learning history with training rows apart from the series.
fn training_history(h: &mut Harness<'_, GrandCoachApp>) {
    training_retry_help(h);
    click(h, Role::Button, "Überspringen");
    click(h, Role::Button, "Lernverlauf …");
}

fn training_history_en(h: &mut Harness<'_, GrandCoachApp>) {
    training_history(h);
    english(h);
}

/// Back at the unchanged live table after a training.
fn training_return(h: &mut Harness<'_, GrandCoachApp>) {
    learning_file(h, "training-return", None);
    play_until(h, |v, _| v.tricks_played >= 1);
    let before = h.state().live_session().view();
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Zähltraining …");
    click(h, Role::Button, "Resttrümpfe üben");
    click(h, Role::Button, "Antworten");
    click(h, Role::Button, "Zurück zur Partie");
    assert_eq!(h.state().live_session().view(), before);
}

/// The counting aid in card play, after the third trick (D-64).
fn counting_aid(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_assist(AID);
    play_until(harness, |view, _| {
        view.tricks_played >= 3 && view.acting == Some(skat_app::PlayerId::HUMAN)
    });
}

/// The same in English.
fn counting_aid_en(harness: &mut Harness<'_, GrandCoachApp>) {
    counting_aid(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// The first counting question; the aid is on but withheld meanwhile.
fn quiz_question(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_assist(AID_AND_QUIZ);
    play_until(harness, |view, _| view.assist.quiz.question.is_some());
    click(harness, Role::Button, "+ 10");
    click(harness, Role::Button, "+ 1");
}

/// The same in English.
fn quiz_question_en(harness: &mut Harness<'_, GrandCoachApp>) {
    quiz_question(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// After answering: the result with its derivation, the aid back.
fn quiz_feedback(harness: &mut Harness<'_, GrandCoachApp>) {
    quiz_question(harness);
    click(harness, Role::Button, "Antworten");
}

/// The same in English.
fn quiz_feedback_en(harness: &mut Harness<'_, GrandCoachApp>) {
    quiz_feedback(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// The window at launch: the "new series" dialog shows the series settings
/// and the opponents wait (`harness` opens it like `main`).
fn launch(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.run_steps(4);
}

/// The first decision of the series: the human bids.
fn bidding(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
}

/// The human has added a card to a trick that is not complete yet.
fn trick(harness: &mut Harness<'_, GrandCoachApp>) {
    play_until(harness, |view, played| {
        played && !view.current_trick.is_empty()
    });
}

/// The optional Neural table with the real bundled model during card play.
fn neural_trick(harness: &mut Harness<'_, GrandCoachApp>) {
    assert!(
        skat_app::neural_model_available(),
        "bundled model must load"
    );
    play_until(harness, |view, _| {
        view.tricks_played >= 1 && view.acting == Some(skat_app::PlayerId::HUMAN)
    });
}

/// The human's card completed a trick, which stays in the middle.
fn trick_complete(harness: &mut Harness<'_, GrandCoachApp>) {
    play_until(harness, |view, played| {
        played && view.current_trick.is_empty() && view.last_trick.is_some()
    });
}

/// The first game is over and its result is shown.
fn result(harness: &mut Harness<'_, GrandCoachApp>) {
    play_until(harness, |view, _| view.acting.is_none());
}

/// The settings menu with the pointer on the developer option "Seed
/// anzeigen", whose tooltip explains the seed.
fn settings(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
    click(harness, Role::Button, "Einstellungen");
    harness
        .get_by_role_and_label(Role::CheckBox, "Seed anzeigen")
        .hover();
    // Tooltips appear after a short delay.
    harness.run_steps(8);
}

/// The "new series" dialog by default: no seed.
fn new_series(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
    click(harness, Role::Button, "Spiel");
    click(harness, Role::Button, "Neue Serie …");
}

/// The "new series" dialog with "Seed anzeigen" on and an invalid seed.
fn new_series_seed(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
    click(harness, Role::Button, "Einstellungen");
    click(harness, Role::CheckBox, "Seed anzeigen");
    harness.key_press(Key::Escape);
    harness.run_steps(4);
    click(harness, Role::Button, "Spiel");
    click(harness, Role::Button, "Neue Serie …");
    let input = "Startwert (Seed):";
    harness
        .get_by_role_and_label(Role::TextInput, input)
        .focus();
    harness.run_steps(1);
    harness
        .get_by_role_and_label(Role::TextInput, input)
        .type_text("x");
    harness.run_steps(4);
}

/// The coach panel after "Was soll ich bedenken?" in the second trick:
/// objective, facts and estimates, no options yet (D-49).
fn coach(harness: &mut Harness<'_, GrandCoachApp>) {
    play_until(harness, |view, _| {
        view.tricks_played >= 1 && view.legal_actions.len() >= 2
    });
    click(harness, Role::Button, "Was soll ich bedenken?");
    await_coach(harness, |view| {
        matches!(view.coach.advice, Advice::Ready(_) | Advice::Provisional(_))
    });
}

/// The second stage: the options unjudged and a qualitative hint.
fn coach_situation(harness: &mut Harness<'_, GrandCoachApp>) {
    coach(harness);
    click(harness, Role::Button, "Wie ist die Lage?");
}

/// The third stage: every option's figures, unjudged, in neutral order.
fn coach_figures(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_situation(harness);
    click(harness, Role::Button, "Zahlen zeigen");
}

/// The third stage with the figures hidden: back to the second.
fn coach_no_figures(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_figures(harness);
    let settings = view(harness).coach.settings;
    harness.state_mut().set_coach(CoachSettings {
        show_figures: false,
        ..settings
    });
    harness.run_steps(4);
}

/// The note on the human's last card while the opponents play on.
fn coach_note(harness: &mut Harness<'_, GrandCoachApp>) {
    play_until(harness, |view, played| {
        played && view.tricks_played >= 1 && view.acting != Some(skat_app::PlayerId::HUMAN)
    });
    await_coach(harness, |view| {
        view.coach
            .last_move
            .as_ref()
            .is_some_and(|m| m.note.is_some())
    });
}

/// The first card decision whose hints carry a lesson (#43): the attention
/// question above the facts, no answer yet.
fn coach_lesson(harness: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..10 {
        play_until(harness, |view, _| {
            view.legal_actions.len() >= 2
                && matches!(view.legal_actions[0], Action::Play(_))
                && view.coach.next_stage == Some(skat_app::coach_model::Stage::Considerations)
        });
        click(harness, Role::Button, "Was soll ich bedenken?");
        await_coach(harness, |view| {
            matches!(view.coach.advice, Advice::Ready(_) | Advice::Provisional(_))
        });
        if let Advice::Ready(h) | Advice::Provisional(h) = &view(harness).coach.advice
            && h.lesson.is_some()
        {
            return;
        }
        play_until(harness, |_, played| played);
    }
    panic!("no lesson in hints");
}

/// The note on the first own card that taught a lesson: evidence with the
/// cards, exception and transfer question.
fn coach_lesson_review(harness: &mut Harness<'_, GrandCoachApp>) {
    review_with(harness, |_| true);
}

/// The note after a card that taught the remaining-Trumpf lesson.
fn coach_lesson_trumpf(harness: &mut Harness<'_, GrandCoachApp>) {
    review_with(harness, |l| {
        matches!(l, LessonContent::RemainingTrumpf { .. })
    });
}

/// The note after a card that taught the Abstich mechanism (#44): the
/// human prefers an Ass or a Zehn, so the position arises sooner.
fn coach_lesson_abstich(harness: &mut Harness<'_, GrandCoachApp>) {
    review_choosing(
        harness,
        |view| cards_of(view).find(|c| matches!(c.rank(), Rank::Ass | Rank::Zehn)),
        |l| matches!(l, LessonContent::TrumpfVulnerability { .. }),
    );
}

/// The same in English.
fn coach_lesson_abstich_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_lesson_abstich(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// The note after a card that taught the partner-overtake mechanism (#44):
/// as second player the human prefers the last (lowest) legal card.
fn coach_lesson_overtake(harness: &mut Harness<'_, GrandCoachApp>) {
    review_choosing(
        harness,
        |view| {
            (view.current_trick.len() == 1)
                .then(|| cards_of(view).last())
                .flatten()
        },
        |l| matches!(l, LessonContent::PartnerOvertake { .. }),
    );
}

/// A smaller coach budget for the bidding scenarios: analyses before card
/// play simulate whole games and are slow in a debug build.
const PRE_PLAY: Budget = Budget {
    samples: 4,
    nodes_per_task: 2_000_000,
};

/// The human's first bidding call with the hints requested (#45): the
/// hand-structure question, no answer and no call named.
fn coach_lesson_structure(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach_budget(PRE_PLAY);
    settle(harness);
    assert!(view(harness).legal_actions.contains(&Action::Pass));
    click(harness, Role::Button, "Was soll ich bedenken?");
    await_coach(harness, |view| {
        matches!(&view.coach.advice, Advice::Ready(h) | Advice::Provisional(h)
            if matches!(h.lesson, Some(LessonContent::HandStructure { .. })))
    });
}

/// The note after passing at the first call: the own Buben, Trumpf and
/// Spitzen per candidate game, with what the unknown Skat can change.
fn coach_lesson_structure_review(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_lesson_structure(harness);
    click(harness, Role::Button, "Passe");
    await_lesson(harness, |l| {
        matches!(l, LessonContent::HandStructure { .. })
    });
}

/// The note after the human's second bidding decision (or the pickup
/// decision as declarer): the game values against the value to reach.
/// The human first bids or holds the lowest value, then passes or picks
/// up the Skat.
fn coach_lesson_value_review(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach_budget(PRE_PLAY);
    settle(harness);
    let v = view(harness);
    let first = if v.legal_actions.contains(&Action::Hold) {
        format!("Ja, {} halten", v.bid)
    } else {
        let lowest = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) => Some(*value),
            _ => None,
        });
        format!("{} sagen", lowest.expect("a bid"))
    };
    click(harness, Role::Button, &first);
    settle(harness);
    let v = view(harness);
    let second = if v.legal_actions.contains(&Action::TakeSkat) {
        "Skat aufnehmen"
    } else {
        "Passe"
    };
    click(harness, Role::Button, second);
    await_lesson(harness, |l| matches!(l, LessonContent::GameValue { .. }));
}

/// The same in English.
fn coach_lesson_value_review_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_lesson_value_review(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// Runs frames until the note on the last move carries a lesson `wanted`.
fn await_lesson(harness: &mut Harness<'_, GrandCoachApp>, wanted: impl Fn(&LessonContent) -> bool) {
    await_coach(harness, |view| {
        view.coach
            .last_move
            .as_ref()
            .and_then(|m| m.lesson.as_ref())
            .is_some_and(&wanted)
    });
}

/// The human's legal cards in the view's order.
fn cards_of(view: &TableView) -> impl Iterator<Item = Card> + '_ {
    view.legal_actions.iter().filter_map(|a| match a {
        Action::Play(card) => Some(*card),
        _ => None,
    })
}

/// Plays as a defender, preferring the card `choose` picks (otherwise the
/// first legal one), until the note on an own card carries a lesson
/// `wanted`.
fn review_choosing(
    harness: &mut Harness<'_, GrandCoachApp>,
    choose: impl Fn(&TableView) -> Option<Card>,
    wanted: impl Fn(&LessonContent) -> bool,
) {
    for _ in 0..400 {
        settle(harness);
        let v = view(harness);
        assert!(!v.series_finished, "no lesson in the series");
        let legal = &v.legal_actions;
        let (label, played) = if v.acting.is_none() {
            ("Nächstes Spiel".to_owned(), false)
        } else if legal.contains(&Action::Pass) {
            ("Passe".to_owned(), false)
        } else if legal.contains(&Action::KeinKontra) {
            ("Kein Kontra".to_owned(), false)
        } else if let Some(card) = choose(&v).or_else(|| cards_of(&v).next()) {
            (card_label(card), true)
        } else {
            panic!("no scripted click for {legal:?}");
        };
        click(harness, Role::Button, &label);
        if !played {
            continue;
        }
        // After the game a forced last card gets no note.
        await_coach(harness, |view| {
            view.result.is_some()
                || view
                    .coach
                    .last_move
                    .as_ref()
                    .is_some_and(|m| m.note.is_some())
        });
        if view(harness)
            .coach
            .last_move
            .and_then(|m| m.lesson)
            .is_some_and(|l| wanted(&l))
        {
            return;
        }
    }
    panic!("no lesson after a move");
}

/// Plays on until the note on an own card carries a lesson `wanted`.
fn review_with(harness: &mut Harness<'_, GrandCoachApp>, wanted: impl Fn(&LessonContent) -> bool) {
    for _ in 0..20 {
        play_until(harness, |view, played| {
            played && view.acting != Some(skat_app::PlayerId::HUMAN)
        });
        await_coach(harness, |view| {
            view.coach
                .last_move
                .as_ref()
                .is_some_and(|m| m.note.is_some())
        });
        if view(harness)
            .coach
            .last_move
            .and_then(|m| m.lesson)
            .is_some_and(|l| wanted(&l))
        {
            return;
        }
    }
    panic!("no lesson after a move");
}

/// The same in English.
fn coach_lesson_review_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_lesson_review(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// The debriefing after the first game, coach at every move, figures off
/// by default: the moments' summaries, details collapsed.
fn coach_debriefing(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach(CoachSettings {
        level: CoachLevel::EveryMove,
        ..CoachSettings::default()
    });
    play_until(harness, |view, _| view.acting.is_none());
    await_coach(harness, |view| {
        matches!(view.coach.debriefing, Some(Debriefing::Ready(_)))
    });
}

/// The debriefing with the details of its first moment open.
fn coach_debriefing_details(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_debriefing(harness);
    open_details(harness, "Analysedetails");
}

/// The debriefing in English, details of the first moment open.
fn coach_debriefing_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_debriefing(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
    open_details(harness, "Analysis details");
}

/// The debriefing scrolled to its last moment, a card-play decision: the
/// table of that decision (hand, trick so far) and its history up to the
/// decision open (issue #47).
fn coach_debriefing_history(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_debriefing(harness);
    open_last(
        harness,
        "Verlauf bis zu dieser Entscheidung",
        "Analysedetails",
    );
}

/// The same in English.
fn coach_debriefing_history_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_debriefing(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
    open_last(harness, "History up to this decision", "Analysis details");
}

/// Opens the collapsed section `label` of the debriefing's last moment and
/// scrolls the panel down to `below`, the last line after it.
fn open_last(harness: &mut Harness<'_, GrandCoachApp>, label: &str, below: &str) {
    let last = |harness: &Harness<'_, GrandCoachApp>, label: &str| {
        harness
            .get_all_by_label(label)
            .last()
            .expect("a moment with a history")
            .scroll_to_me();
    };
    last(harness, label);
    harness.run_steps(4);
    harness.get_all_by_label(label).last().unwrap().click();
    harness.run_steps(4);
    last(harness, below);
    harness.run_steps(4);
}

/// Opens the collapsed section `label` of the debriefing's first moment.
fn open_details(harness: &mut Harness<'_, GrandCoachApp>, label: &str) {
    harness
        .get_all_by_label(label)
        .next()
        .expect("a moment with details")
        .click();
    harness.run_steps(4);
}

/// The warning before a clearly weaker card (coach at every move).
fn coach_warning(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach(CoachSettings {
        level: CoachLevel::EveryMove,
        warnings: true,
        show_figures: true,
        focus: None,
    });
    for _ in 0..400 {
        settle(harness);
        await_coach(harness, |view| {
            !view.coach.analysing && !matches!(view.coach.advice, Advice::Computing)
        });
        let v = view(harness);
        if v.series_finished {
            break;
        }
        let weak = harness
            .state()
            .session()
            .coach_result()
            .and_then(|r| r.explanation())
            .filter(|e| e.complete())
            .and_then(|e| {
                e.options
                    .iter()
                    .find(|o| o.judgement == Judgement::Weak)
                    .map(|o| o.action)
            });
        if let Some(Action::Play(card)) = weak {
            click(harness, Role::Button, &card_label(card));
            await_coach(harness, |view| {
                matches!(view.coach.pending, Some(Pending::Warning { .. }))
            });
            return;
        }
        let label = if v.acting.is_none() {
            "Nächstes Spiel".to_owned()
        } else if v.legal_actions.contains(&Action::Pass) {
            "Passe".to_owned()
        } else if v.legal_actions.contains(&Action::KeinKontra) {
            "Kein Kontra".to_owned()
        } else if let Some(&Action::Play(card)) = v.legal_actions.first() {
            card_label(card)
        } else {
            panic!("no scripted click for {:?}", v.legal_actions);
        };
        click(harness, Role::Button, &label);
        if view(harness).coach.pending.is_some() {
            click(harness, Role::Button, "Trotzdem spielen");
        }
    }
    panic!("no weak card found");
}

/// The settings menu with a learning focus chosen (#46).
fn coach_focus(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
    click(harness, Role::Button, "Einstellungen");
    click(harness, Role::RadioButton, "Wo die Trümpfe sind");
    // Choosing closes the menu; open it again to show the choice.
    click(harness, Role::Button, "Einstellungen");
}

/// Coach at every move, a card decision without a supported lesson: the
/// compact status instead of invented hints, with "Ausblenden" (#46).
fn coach_no_lesson(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach(CoachSettings {
        level: CoachLevel::EveryMove,
        ..CoachSettings::default()
    });
    let card_decision = |view: &TableView| {
        view.acting == Some(skat_app::PlayerId::HUMAN)
            && matches!(view.legal_actions.first(), Some(Action::Play(_)))
    };
    for _ in 0..40 {
        play_until(harness, |view, _| card_decision(view));
        await_coach(harness, |view| {
            !matches!(view.coach.advice, Advice::Computing)
        });
        let v = view(harness);
        if v.coach.advice == Advice::NoLesson {
            return;
        }
        let card = cards_of(&v).next().expect("a card to play");
        click(harness, Role::Button, &card_label(card));
    }
    panic!("no decision without a lesson");
}

/// The debriefing after the first game with the focus "Wo die Trümpfe
/// sind": one moment per concept, the focus marked, each with its lesson.
fn coach_focus_debriefing(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_coach(CoachSettings {
        level: CoachLevel::EveryMove,
        focus: Some(Concept::RemainingTrumpf),
        ..CoachSettings::default()
    });
    play_until(harness, |view, _| view.acting.is_none());
    await_coach(harness, |view| {
        matches!(view.coach.debriefing, Some(Debriefing::Ready(_)))
    });
    // The focused moment's lesson evidence open.
    let title = "Was sicher feststeht";
    if let Some(node) = harness.get_all_by_label(title).last() {
        node.click();
    }
    harness.run_steps(4);
}

/// The same in English.
fn coach_focus_debriefing_en(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_focus_debriefing(harness);
    harness.state_mut().set_language(Language::En);
    harness.run_steps(4);
}

/// Runs frames until the coach state satisfies `done`.
fn await_coach(harness: &mut Harness<'_, GrandCoachApp>, done: impl Fn(&TableView) -> bool) {
    for _ in 0..20_000 {
        harness.run_steps(1);
        if done(&view(harness)) {
            harness.run_steps(4);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the coach did not finish");
}

/// God Mode at the human's first bid: every card face up, the Skat
/// included, on the blue table with the God Mode faces.
fn god_mode(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
}

/// The same with the German faces.
fn god_mode_german(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_deck(Deck::German);
    settle(harness);
}

/// God Mode with "Alle Karten sehen" off: the God Mode back.
fn god_mode_hidden(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
    click(harness, Role::CheckBox, "Alle Karten sehen");
}

/// God Mode during card play.
fn god_mode_trick(harness: &mut Harness<'_, GrandCoachApp>) {
    trick(harness);
}

/// The all-seeing coach in God Mode at its second stage: options still
/// unjudged (D-49).
fn god_mode_coach(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_situation(harness);
}

/// The "new series" dialog with the open display chosen: the opponents
/// stay as selected and the note names the closed-view coach.
fn open_display_new_series(harness: &mut Harness<'_, GrandCoachApp>) {
    click(
        harness,
        Role::CheckBox,
        "Offene Anzeige: alle Hände und den Skat zeigen",
    );
}

/// The same in English.
fn open_display_new_series_en(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_language(Language::En);
    harness.run_steps(2);
    click(
        harness,
        Role::CheckBox,
        "Open display: show all hands and the Skat",
    );
}

/// Open display at the human's first bid: every hand and the Skat face up
/// on the ordinary green table, labelled apart from the closed-view coach.
fn open_display(harness: &mut Harness<'_, GrandCoachApp>) {
    settle(harness);
}

/// The same in English.
fn open_display_en(harness: &mut Harness<'_, GrandCoachApp>) {
    harness.state_mut().set_language(Language::En);
    settle(harness);
}

/// Open display during card play.
fn open_display_trick(harness: &mut Harness<'_, GrandCoachApp>) {
    trick(harness);
}

/// The coach with an open display: it names its closed-view basis.
fn open_display_coach(harness: &mut Harness<'_, GrandCoachApp>) {
    coach_situation(harness);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(out) = args.next() else {
        let names: Vec<_> = SCENARIOS.iter().map(|(name, _)| *name).collect();
        eprintln!("usage: screenshots OUT_DIR [SCENARIO...]; scenarios: {names:?}");
        std::process::exit(2);
    };
    let wanted: Vec<String> = args.collect();
    if let Some(unknown) = wanted
        .iter()
        .find(|w| !SCENARIOS.iter().any(|(name, _)| name == w))
    {
        eprintln!("unknown scenario {unknown:?}");
        std::process::exit(2);
    }
    std::fs::create_dir_all(&out).expect("create output directory");
    for (name, scenario) in SCENARIOS {
        if !wanted.is_empty() && !wanted.iter().any(|w| w == name) {
            continue;
        }
        let seed = SEEDS
            .iter()
            .find(|(n, _)| n == name)
            .map_or(SEED, |&(_, seed)| seed);
        let seed = if name.starts_with("table-") {
            LONG_NAMES
        } else {
            seed
        };
        let opponents = if GOD_MODE.contains(name) {
            Opponents::GodMode
        } else if *name == "table-mephisto-960" {
            Opponents::Mephisto
        } else if *name == "table-goldfish-960" {
            Opponents::Goldfish
        } else if *name == "neural-trick" || *name == "table-neural-960" {
            Opponents::Neural
        } else {
            Opponents::Levels([Level::Easy; 2])
        };
        let display = if OPEN_DISPLAY.contains(name) {
            TableDisplay::Open
        } else {
            TableDisplay::Closed
        };
        let launch = *name == "launch" || name.starts_with("open-display-new-series");
        let mut harness = if name.starts_with("coach-jump-context") {
            Harness::builder()
                .with_size([1440.0, 1000.0])
                .wgpu()
                .build_eframe(|cc| GrandCoachApp::new(&cc.egui_ctx, qa146::config()))
        } else {
            harness(
                launch,
                seed,
                opponents,
                display,
                name.starts_with("guided-"),
            )
        };
        if let Some(size) = table_size(name) {
            harness.set_size(size.into());
            harness.run_steps(2);
        }
        scenario(&mut harness);
        if name.ends_with("-narrow") {
            harness.set_size(NARROW.into());
        }
        harness.run_steps(4);
        let path = format!("{out}/{name}.png");
        harness
            .render()
            .expect("render with wgpu")
            .save(&path)
            .expect("write PNG");
        println!("{path}");
    }
}

/// The app with a small series; `launch` opens the "new series" dialog
/// before the first frame, as `main` does.
fn harness(
    launch: bool,
    seed: u64,
    opponents: Opponents,
    display: TableDisplay,
    guided: bool,
) -> Harness<'static, GrandCoachApp> {
    let config = SessionConfig {
        seed,
        series: if guided {
            SeriesLength::Free
        } else {
            SeriesLength::Fixed(2)
        },
        kontra_re: !guided,
        opponents,
        display,
    };
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = GrandCoachApp::new(&cc.egui_ctx, config);
            if launch {
                app.show_new_series();
            }
            app
        });
    // Simulated time (0.25 s per frame): opponents wait 2 s after the human's
    // card, so a completed trick stays visible in the middle.
    harness.state_mut().opponent_delay = 2.0;
    // A smaller coach budget than interactive keeps the debug build quick.
    harness.state_mut().set_coach_budget(Budget {
        samples: 8,
        nodes_per_task: 20_000_000,
    });
    harness.run_steps(2);
    harness
}

// ---------------------------------------------------------------------------
// The modern table of issue #134: the human declares, discards from twelve
// cards, announces, plays tricks to the result and the series end, at
// 1600 × 1000, 1280 × 820 and 960 × 640, with long names, scaled text,
// both languages and card faces and every special table mode.

/// Plays as declarer by clicks: bids while the value is at most 30 (else
/// passes), takes the Skat, discards the first two cards, announces the
/// preset Grand, declines Re and plays the first legal card, until `stop`.
/// As a defender in later games it passes and plays the first legal card.
fn declare_until(h: &mut Harness<'_, GrandCoachApp>, stop: impl Fn(&TableView) -> bool) {
    for _ in 0..400 {
        settle(h);
        let v = view(h);
        if stop(&v) {
            return;
        }
        let legal = &v.legal_actions;
        if v.acting.is_none() {
            if v.series_finished {
                panic!("series finished before the state");
            }
            click(h, Role::Button, "Nächstes Spiel");
            continue;
        }
        let bid = legal.iter().find_map(|a| match a {
            Action::Bid(b) if *b <= 30 && v.game_number == 1 => Some(*b),
            _ => None,
        });
        if let Some(b) = bid {
            click(h, Role::Button, &format!("{b} sagen"));
        } else if legal.contains(&Action::Hold) && v.bid <= 30 && v.game_number == 1 {
            click(h, Role::Button, &format!("Ja, {} halten", v.bid));
        } else if legal.contains(&Action::Pass) {
            click(h, Role::Button, "Passe");
        } else if legal.contains(&Action::TakeSkat) {
            click(h, Role::Button, "Skat aufnehmen");
        } else if legal.iter().any(|a| matches!(a, Action::Discard(_))) {
            for &card in &v.hand[..2] {
                let label = shown_card_label(h, card);
                click(h, Role::Button, &label);
            }
            click(h, Role::Button, "Drücken");
        } else if legal.iter().any(|a| matches!(a, Action::Announce(_))) {
            click(h, Role::Button, "Ansagen");
        } else if legal.contains(&Action::KeinRe) {
            click(h, Role::Button, "Kein Re");
        } else if legal.contains(&Action::KeinKontra) {
            click(h, Role::Button, "Kein Kontra");
        } else if let Some(&Action::Play(card)) = legal.first() {
            let label = shown_card_label(h, card);
            click(h, Role::Button, &label);
        } else {
            panic!("no scripted click for {legal:?}");
        }
    }
    panic!("scenario did not reach its state");
}

fn human_declares(v: &TableView) -> bool {
    v.declarer == Some(skat_app::PlayerId::HUMAN)
}

fn table_bidding(h: &mut Harness<'_, GrandCoachApp>) {
    settle(h);
}

fn table_discard(h: &mut Harness<'_, GrandCoachApp>) {
    declare_until(h, |v| {
        v.phase == skat_app::Phase::Druecken && v.hand.len() == 12
    });
    // Two cards chosen, not yet discarded.
    let v = view(h);
    for &card in &v.hand[..2] {
        let label = shown_card_label(h, card);
        click(h, Role::Button, &label);
    }
}

fn table_announce(h: &mut Harness<'_, GrandCoachApp>) {
    declare_until(h, |v| v.phase == skat_app::Phase::Ansage);
}

fn table_trick(h: &mut Harness<'_, GrandCoachApp>) {
    declare_until(h, |v| {
        human_declares(v) && v.tricks_played >= 3 && v.acting == Some(skat_app::PlayerId::HUMAN)
    });
}

/// The concealed Augen shown while the button is held down (not released
/// before the picture).
fn table_augen(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
    let pos = h
        .get_by_role_and_label(Role::Button, "Augen zeigen (gedrückt halten)")
        .rect()
        .center();
    h.input_mut().events.push(egui::Event::PointerMoved(pos));
    h.input_mut().events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(4);
    assert!(
        h.query_all_by_label_contains("Gegenpartei ")
            .next()
            .is_some(),
        "Augen revealed while held"
    );
}

fn table_result(h: &mut Harness<'_, GrandCoachApp>) {
    declare_until(h, |v| v.game_number == 1 && v.acting.is_none());
}

fn table_series_end(h: &mut Harness<'_, GrandCoachApp>) {
    declare_until(h, |v| v.series_finished);
}

fn table_bidding_1600(h: &mut Harness<'_, GrandCoachApp>) {
    table_bidding(h);
}
fn table_discard_1600(h: &mut Harness<'_, GrandCoachApp>) {
    table_discard(h);
}
fn table_trick_1600(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
}
fn table_result_1600(h: &mut Harness<'_, GrandCoachApp>) {
    table_result(h);
}
fn table_bidding_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_bidding(h);
}
fn table_discard_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_discard(h);
}
fn table_announce_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_announce(h);
}
fn table_trick_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
}
fn table_augen_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_augen(h);
}
fn table_result_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_result(h);
}
fn table_series_end_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_series_end(h);
}

/// English names and portraits of the same series after the trick state.
fn table_en_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
    h.state_mut().set_language(Language::En);
}

fn table_german_deck_960(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
    h.state_mut().set_deck(Deck::German);
}

fn table_french2_en(h: &mut Harness<'_, GrandCoachApp>) {
    table_trick(h);
    h.state_mut().set_deck(Deck::French2);
    h.state_mut().set_language(Language::En);
}

/// Every text style 30 % larger (as with a larger system font).
fn scale_text(h: &mut Harness<'_, GrandCoachApp>) {
    h.ctx.all_styles_mut(|style| {
        for font in style.text_styles.values_mut() {
            font.size *= 1.3;
        }
    });
    h.run_steps(2);
}

fn table_large_text_960(h: &mut Harness<'_, GrandCoachApp>) {
    scale_text(h);
    table_discard(h);
}

fn table_large_text_en_960(h: &mut Harness<'_, GrandCoachApp>) {
    scale_text(h);
    table_trick(h);
    h.state_mut().set_language(Language::En);
}

/// egui's own zoom (Ctrl +) at 125 %: 1280 × 820 shows like 1024 × 656.
fn table_zoom_1280(h: &mut Harness<'_, GrandCoachApp>) {
    h.ctx.set_zoom_factor(1.25);
    h.run_steps(2);
    table_discard(h);
}

fn table_mephisto_960(h: &mut Harness<'_, GrandCoachApp>) {
    play_until(h, |view, _| view.tricks_played >= 2);
}

fn table_goldfish_960(h: &mut Harness<'_, GrandCoachApp>) {
    play_until(h, |view, _| view.tricks_played >= 2);
}

fn table_god_960(h: &mut Harness<'_, GrandCoachApp>) {
    play_until(h, |view, _| view.tricks_played >= 2);
}

fn table_open_960(h: &mut Harness<'_, GrandCoachApp>) {
    play_until(h, |view, _| view.tricks_played >= 2);
}

fn table_neural_960(h: &mut Harness<'_, GrandCoachApp>) {
    play_until(h, |view, _| view.tricks_played >= 2);
}

/// Without the coach the table and hand take the room of the coach panel.
fn table_coach_off_960(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().set_coach(CoachSettings {
        level: CoachLevel::Off,
        ..CoachSettings::default()
    });
    table_trick(h);
}

fn view(harness: &Harness<'_, GrandCoachApp>) -> TableView {
    harness.state().session().view()
}

/// Lets the opponents move until the human acts or the game is over.
/// Mephisto (God Mode) thinks for seconds in a debug build.
fn settle(harness: &mut Harness<'_, GrandCoachApp>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    while std::time::Instant::now() < deadline {
        if !view(harness).opponent_to_move {
            harness.run_steps(3);
            return;
        }
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not finish their moves");
}

/// Plays the first game as a defender (the human passes, says no Kontra and
/// plays the first legal card) until `stop(view, card_just_played)` holds.
fn play_until(harness: &mut Harness<'_, GrandCoachApp>, stop: impl Fn(&TableView, bool) -> bool) {
    for _ in 0..200 {
        settle(harness);
        let before = view(harness);
        if stop(&before, false) {
            return;
        }
        let legal = &before.legal_actions;
        let (label, played) = if before.acting.is_none() {
            ("Nächstes Spiel".to_owned(), false)
        } else if legal.contains(&Action::Pass) {
            ("Passe".to_owned(), false)
        } else if legal.contains(&Action::KeinKontra) {
            ("Kein Kontra".to_owned(), false)
        } else if let Some(&Action::Play(card)) = legal.first() {
            (shown_card_label(harness, card), true)
        } else {
            panic!("no scripted click for {legal:?}");
        };
        harness.get_by_role_and_label(Role::Button, &label).click();
        harness.run_steps(4);
        if stop(&view(harness), played) {
            return;
        }
    }
    panic!("scenario did not reach its state");
}

fn click(harness: &mut Harness<'_, GrandCoachApp>, role: Role, label: &str) {
    harness.get_by_role_and_label(role, label).click();
    harness.run_steps(4);
}

/// German card label as shown to assistive technology.
/// The label of `card` on the faces shown: God Mode names Pik and Karo
/// after their symbols, with the French name in brackets (D-62).
fn shown_card_label(harness: &Harness<'_, GrandCoachApp>, card: Card) -> String {
    let label = card_label(card);
    if harness.state().theme() != Theme::God {
        return label;
    }
    for (french, symbol) in [("Pik", "Mond"), ("Karo", "Stern")] {
        if let Some(rest) = label.strip_prefix(french) {
            return format!("{symbol}{rest} ({french})");
        }
    }
    label
}

fn card_label(card: Card) -> String {
    let suit = ["Kreuz", "Pik", "Herz", "Karo"][usize::from(card.suit().index())];
    let rank = [
        "Sieben", "Acht", "Neun", "Zehn", "Bube", "Dame", "König", "Ass",
    ][usize::from(card.rank().index())];
    format!("{suit}-{rank}")
}

/// Reopen a completed game through the real Game menu and local file dialog.
fn recording(h: &mut Harness<'_, GrandCoachApp>) {
    result(h);
    let directory =
        std::env::temp_dir().join(format!("skat-screenshot-recording-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("game.json");
    // Each scenario owns its temporary file; never delete user recordings.
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Spiel speichern …");
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(
        h,
        Role::CheckBox,
        "Verdeckte Informationen bewusst in die Datei schreiben",
    );
    click(h, Role::Button, "Datei speichern");
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Aufnahme öffnen …");
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(h, Role::Button, "Datei öffnen");
    click(
        h,
        Role::CheckBox,
        "Alle verdeckten Karten und den Skat aufdecken",
    );
    h.get_by_label_contains("Vollständige Information");
    // Show a useful historical position with remaining cards and a full trick.
    click(h, Role::Button, "Anfang");
    h.get_by_label("Nach 0 Aktionen");
    click(h, Role::Button, "Weiter");
    std::fs::remove_dir_all(directory).unwrap();
}

fn set_recording_path(h: &mut Harness<'_, GrandCoachApp>, label: &str, path: &str) {
    h.get_by_role_and_label(Role::TextInput, label).focus();
    h.run_steps(1);
    h.get_by_role_and_label(Role::TextInput, label)
        .type_text(path);
    h.run_steps(4);
}

fn recording_en(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn recording_error(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Aufnahme öffnen …");
    let path = std::env::temp_dir().join(format!(
        "skat-screenshot-corrupt-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, "{").unwrap();
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(h, Role::Button, "Datei öffnen");
    std::fs::remove_file(path).unwrap();
}

fn recording_error_en(h: &mut Harness<'_, GrandCoachApp>) {
    recording_error(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn recording_final(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    click(h, Role::Button, "Ende");
}

/// From the reopened recording: a fresh practice attempt before bidding.
fn practice(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    click(h, Role::Button, "Dieses Blatt üben");
    assert!(h.state().practicing());
    settle(h);
}

/// Practice with other legal cards than the original game (the original
/// played the first legal card): the last legal card at each card decision.
fn practice_alternative(h: &mut Harness<'_, GrandCoachApp>, stop: impl Fn(&TableView) -> bool) {
    for _ in 0..200 {
        settle(h);
        let before = view(h);
        if stop(&before) {
            return;
        }
        let legal = &before.legal_actions;
        let label = if legal.contains(&Action::Pass) {
            "Passe".to_owned()
        } else if legal.contains(&Action::KeinKontra) {
            "Kein Kontra".to_owned()
        } else if let Some(&Action::Play(card)) = legal.last() {
            shown_card_label(h, card)
        } else {
            panic!("no scripted click for {legal:?}");
        };
        click(h, Role::Button, &label);
    }
    panic!("practice did not reach its state");
}

/// Mid-game in practice after alternative cards.
fn practice_play(h: &mut Harness<'_, GrandCoachApp>) {
    practice(h);
    practice_alternative(h, |v| {
        v.tricks_played >= 3 && v.acting == Some(skat_app::PlayerId::HUMAN)
    });
}

fn practice_play_en(h: &mut Harness<'_, GrandCoachApp>) {
    practice_play(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// The practice game is over: not scored, no next game.
fn practice_finished(h: &mut Harness<'_, GrandCoachApp>) {
    practice(h);
    practice_alternative(h, |v| v.acting.is_none());
    h.get_by_label_contains("Übungsspiel beendet.");
}

/// Early exit: back at the untouched live series (game 1 over, its score).
fn practice_return(h: &mut Harness<'_, GrandCoachApp>) {
    practice_play(h);
    click(h, Role::Button, "Zurück zur Serie");
    assert!(!h.state().practicing());
    assert_eq!(view(h).sheet.records.len(), 1);
}

fn practice_return_en(h: &mut Harness<'_, GrandCoachApp>) {
    practice_return(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// The reopened completed game with its first own card decision after the
/// opening lead that had a choice, selected in the replay coach: the table
/// before that move.
fn replay_before(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    // Final position with every card revealed: the reveal stays a display
    // choice and does not change the comment's perspective.
    click(h, Role::Button, "Ende");
    let label = {
        let replay = h.state_mut().replay_coach_mut().expect("recording open");
        let decision = *replay
            .decisions()
            .iter()
            .find(|d| d.options > 1 && d.trick.is_some_and(|t| t > 1))
            .expect("an own card decision with a choice");
        coach::recorded_decision(&I18n::new(Language::De), &decision)
    };
    click(h, Role::ComboBox, "Entscheidung");
    h.get_by_label(&label).click();
    h.run_steps(4);
    h.get_by_value(&label);
}

fn await_replay(h: &mut Harness<'_, GrandCoachApp>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    while h.state().replay_computing() {
        assert!(std::time::Instant::now() < deadline, "replay comment hangs");
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.run_steps(4);
}

/// The historical comment: only what the player knew then.
fn replay_historical(h: &mut Harness<'_, GrandCoachApp>) {
    replay_before(h);
    click(h, Role::Button, "Kommentar anfordern");
    await_replay(h);
    assert!(
        h.query_all_by_label_contains("Sicht: damaliger Wissensstand")
            .next()
            .is_some()
    );
}

fn replay_historical_en(h: &mut Harness<'_, GrandCoachApp>) {
    replay_historical(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// The deliberately requested hindsight comment, persistently labelled.
fn replay_hindsight(h: &mut Harness<'_, GrandCoachApp>) {
    replay_historical(h);
    click(h, Role::Button, "Rückblick: mit allen Karten vergleichen");
    await_replay(h);
    assert!(
        h.query_all_by_label_contains("RÜCKBLICK: alle Karten bekannt")
            .next()
            .is_some()
    );
}

fn replay_hindsight_en(h: &mut Harness<'_, GrandCoachApp>) {
    replay_hindsight(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// At the live open-display table: a reopened open-display recording (the
/// Herz fixture as game 2, the human defending in Hinterhand) with the
/// historical comment on the human's trick-4 card. Its proven-void lesson
/// cites only the declarer's S9 on CA in trick 3; the declarer's Pik void,
/// shown only after this decision, is absent.
fn open_display_proven_void(h: &mut Harness<'_, GrandCoachApp>) {
    let directory = std::env::temp_dir().join(format!(
        "skat-screenshot-proven-void-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("late-herz-open.json");
    // Each scenario owns its temporary file; never delete user recordings.
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    let mut recording = skat_app::GameRecording::read_local(
        include_bytes!("../../skat-app/tests/fixtures/recordings/herz.json").as_slice(),
    )
    .unwrap();
    recording.context.number = 2;
    recording.context.dealer = skat_app::PlayerId(0);
    recording.context.session.display = TableDisplay::Open;
    recording.save_local_new(&path).unwrap();
    settle(h);
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Aufnahme öffnen …");
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(h, Role::Button, "Datei öffnen");
    std::fs::remove_file(&path).unwrap();
    let label = {
        let replay = h.state_mut().replay_coach_mut().expect("recording open");
        let decision = *replay
            .decisions()
            .iter()
            .find(|d| d.prefix == 16)
            .expect("the trick-4 decision");
        coach::recorded_decision(&I18n::new(Language::De), &decision)
    };
    h.get_by_role_and_label(Role::ComboBox, "Entscheidung")
        .click_accesskit();
    h.run_steps(4);
    h.get_by_label(&label).click_accesskit();
    h.run_steps(4);
    h.key_press(Key::Escape);
    h.run_steps(4);
    click(h, Role::Button, "Kommentar anfordern");
    await_replay(h);
    h.get_by_label("Was sicher feststeht").click_accesskit();
    h.run_steps(4);
    // The declarer is the left figure of the recording's series (D-72).
    let figure = skat_app::SeriesCast::for_seed(recording.context.session.seed)
        .figure(skat_app::FigurePool::German, skat_app::PlayerId(1))
        .expect("an opponent has a figure");
    let left = I18n::new(Language::De).t(&format!("figure-{}", figure.key()));
    let lesson = format!("Stich 3: {left} hat Pik-Neun auf Kreuz-Ass gespielt");
    h.get_by_label_contains(&lesson);
    // Scroll the lesson into view.
    h.get_by_label_contains(&lesson).scroll_to_me();
    h.run_steps(4);
}

fn open_display_proven_void_en(h: &mut Harness<'_, GrandCoachApp>) {
    open_display_proven_void(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// An in-progress file saved before the player's first decision: nothing
/// to comment on, said honestly.
fn replay_none(h: &mut Harness<'_, GrandCoachApp>) {
    let directory = std::env::temp_dir().join(format!(
        "skat-screenshot-replay-none-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("early.json");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Spiel speichern …");
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(
        h,
        Role::CheckBox,
        "Verdeckte Informationen bewusst in die Datei schreiben",
    );
    click(h, Role::Button, "Datei speichern");
    click(h, Role::Button, "Spiel");
    click(h, Role::Button, "Aufnahme öffnen …");
    set_recording_path(h, "Dateipfad:", path.to_str().unwrap());
    click(h, Role::Button, "Datei öffnen");
    assert!(
        h.query_all_by_label_contains("noch keine Entscheidung von dir")
            .next()
            .is_some()
    );
    std::fs::remove_dir_all(directory).unwrap();
}

/// Clicks a button wherever its scroll area put it (AccessKit action).
fn press(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    h.run_steps(4);
}

fn await_review(h: &mut Harness<'_, GrandCoachApp>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(900);
    while h.state().review_running() || h.state().replay_computing() {
        assert!(std::time::Instant::now() < deadline, "review hangs");
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.run_steps(4);
}

/// The reopened completed game with the review just requested: progress
/// and its cancel control (D-69).
fn review_running(h: &mut Harness<'_, GrandCoachApp>) {
    recording(h);
    press(h, "Meine Entscheidungen auswerten");
    assert!(h.state().review_running());
}

/// The review cancelled after it started: partial, labelled as such.
fn review_cancelled(h: &mut Harness<'_, GrandCoachApp>) {
    review_running(h);
    // Let a few decisions finish first.
    for _ in 0..200 {
        let done = h
            .query_all_by_label_contains("Werte Entscheidung 3 von")
            .next()
            .is_some();
        if done || !h.state().review_running() {
            break;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    press(h, "Auswertung abbrechen");
    assert!(
        h.query_all_by_label_contains("Abgebrochen:")
            .next()
            .is_some()
    );
}

fn review_cancelled_en(h: &mut Harness<'_, GrandCoachApp>) {
    review_cancelled(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// The finished review: scope and up to three moments in game order.
fn review_moments(h: &mut Harness<'_, GrandCoachApp>) {
    review_running(h);
    await_review(h);
    assert!(
        h.query_all_by_label_contains("Entscheidungen mit Wahl ausgewertet")
            .next()
            .is_some()
    );
}

fn review_moments_en(h: &mut Harness<'_, GrandCoachApp>) {
    review_moments(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

/// A moment's decision selected from the review: the historical comment
/// and the table before that move, without computing again.
fn review_navigate(h: &mut Harness<'_, GrandCoachApp>) {
    review_moments(h);
    let show = "Am Tisch vor diesem Zug zeigen";
    h.query_all_by_label(show)
        .next()
        .expect("at least one moment")
        .click_accesskit();
    h.run_steps(4);
    assert!(!h.state().replay_computing());
    let node = h.get_by_label("Der Tisch vor deinem aufgenommenen Zug");
    node.scroll_to_me();
    h.run_steps(4);
}

fn review_navigate_en(h: &mut Harness<'_, GrandCoachApp>) {
    review_navigate(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
    let node = h.get_by_label("The table before your recorded move");
    node.scroll_to_me();
    h.run_steps(4);
}

fn replay_none_en(h: &mut Harness<'_, GrandCoachApp>) {
    replay_none(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

#[path = "../tests/support/guided.rs"]
mod guided_support;

fn guided_orient(h: &mut Harness<'_, GrandCoachApp>) {
    guided_support::enter(h);
}
fn guided_act(h: &mut Harness<'_, GrandCoachApp>) {
    guided_orient(h);
    guided_support::click(h, "Regelantwort bewusst aufdecken");
}
fn guided_memory(h: &mut Harness<'_, GrandCoachApp>) {
    guided_act(h);
    guided_support::click(h, "Kartenwissen aufklappen");
}
fn guided_observe(h: &mut Harness<'_, GrandCoachApp>) {
    guided_act(h);
    let v = view(h);
    let action = *v.legal_actions.last().unwrap();
    guided_support::click(h, &coach::action(&I18n::new(Language::De), &v, action));
    guided_support::settle(h);
    assert_eq!(
        h.state().session().guided().unwrap().step,
        skat_app::GuidedStep::Observe
    );
}
fn guided_complete(h: &mut Harness<'_, GrandCoachApp>) {
    guided_observe(h);
    guided_support::click(h, "Weiß ich nicht – zeige es mir");
}
fn guided_orient_en(h: &mut Harness<'_, GrandCoachApp>) {
    guided_orient(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}
fn guided_act_en(h: &mut Harness<'_, GrandCoachApp>) {
    guided_act(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}
fn guided_observe_en(h: &mut Harness<'_, GrandCoachApp>) {
    guided_observe(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}
fn guided_complete_en(h: &mut Harness<'_, GrandCoachApp>) {
    guided_complete(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

fn guided_figures(h: &mut Harness<'_, GrandCoachApp>) {
    guided_complete(h);
    h.state_mut().set_coach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    });
    h.run_steps(4);
    guided_support::click(h, "Modellvergleich separat anfordern");
}
fn guided_figures_en(h: &mut Harness<'_, GrandCoachApp>) {
    guided_figures(h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
}

// Catalogue-mode scenarios use real widgets, including settlement before next.
fn focus_count(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    let file = learning_file(h, "focus", None);
    skat_app::LearningHistory::new().save_local(&file).unwrap();
    h.state_mut().use_learning_file(file);
    click(h, Role::Button, "Fokus: Verbleibende Trümpfe");
}
fn focus_count_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_count(h);
    english(h);
}
fn focus_changed(h: &mut Harness<'_, GrandCoachApp>) {
    focus_count(h);
    click(h, Role::Button, "Überspringen");
    click(h, Role::Button, "Nächste Stellung");
}
fn focus_changed_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_changed(h);
    english(h);
}
fn focus_help(h: &mut Harness<'_, GrandCoachApp>) {
    focus_changed(h);
    click(h, Role::Button, "Hilfe einblenden");
}
fn focus_help_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_help(h);
    english(h);
}
fn focus_mechanism(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Fokus: Abstich einer vollen Nebenkarte");
}
fn focus_mechanism_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_mechanism(h);
    english(h);
}
fn focus_feedback(h: &mut Harness<'_, GrandCoachApp>) {
    focus_mechanism(h);
    click(h, Role::Button, "Möglich, aber nicht sicher");
}
fn focus_feedback_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_feedback(h);
    english(h);
}
fn focus_boundary(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Fokus: Übernehmen durch den Partner");
    for _ in 0..2 {
        click(h, Role::Button, "Überspringen");
        click(h, Role::Button, "Nächste Stellung");
    }
    click(h, Role::Button, "Dieser Mechanismus ist nicht vorhanden");
}
fn focus_boundary_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_boundary(h);
    english(h);
}
fn focus_practice(h: &mut Harness<'_, GrandCoachApp>) {
    focus_feedback(h);
    click(h, Role::Button, "Noch einmal");
}
fn focus_practice_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_practice(h);
    english(h);
}
fn focus_void(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Fokus: Nachgewiesene Fehlfarben");
}
fn focus_void_en(h: &mut Harness<'_, GrandCoachApp>) {
    focus_void(h);
    english(h);
}

/// Explicit conditional responses in the real learning overview (#13).
fn mechanism_history(h: &mut Harness<'_, GrandCoachApp>, trends: bool) {
    use skat_app::practice_catalogue::PracticeFamily as F;
    use skat_app::{
        AssistanceLevel, FocusAnswer as R, FocusCommand as C, LearningHistory, TrainingSession,
    };
    let mut history = LearningHistory::new();
    let mut s = TrainingSession::catalogue(
        F::SideCardVulnerability,
        AssistanceLevel::AnswerBeforeReveal,
        &mut history,
        true,
    )
    .unwrap();
    s.execute_focus(C::Answer(R::Certain), &mut history, true)
        .unwrap();
    if trends {
        for n in 0..20 {
            s.execute_focus(C::Retry, &mut history, true).unwrap();
            s.execute_focus(
                C::Answer(if n < 10 { R::Certain } else { R::Possible }),
                &mut history,
                true,
            )
            .unwrap();
        }
    } else {
        s.execute_focus(C::Next, &mut history, true).unwrap();
        s.execute_focus(C::Help, &mut history, true).unwrap();
        s.execute_focus(C::Answer(R::Absent), &mut history, true)
            .unwrap();
        s.execute_focus(C::Next, &mut history, true).unwrap(); // unanswered boundary
    }
    let file = learning_file(h, "mechanism-responses", None);
    history.save_local(&file).unwrap();
    h.state_mut().use_learning_file(file);
    open_learning(h);
    h.get_by_label("Mechanismus-Antworten: beobachtete Muster")
        .scroll_to_me();
    h.run_steps(4);
}
fn mechanism_responses(h: &mut Harness<'_, GrandCoachApp>) {
    mechanism_history(h, false);
}
fn mechanism_responses_en(h: &mut Harness<'_, GrandCoachApp>) {
    mechanism_responses(h);
    english(h);
}
fn mechanism_trends(h: &mut Harness<'_, GrandCoachApp>) {
    mechanism_history(h, true);
}
fn mechanism_trends_en(h: &mut Harness<'_, GrandCoachApp>) {
    mechanism_trends(h);
    english(h);
}

fn mechanism_trends_narrow(h: &mut Harness<'_, GrandCoachApp>) {
    mechanism_trends(h);
    h.get_by_label("Frühere 10 Abschlüsse dieser Aufgabe im selben Kontext")
        .scroll_to_me();
    h.run_steps(4);
}

// Only the chooser and a public practice task may be pictured. Reserved
// positions are exercised by non-rendering technical tests, never galleries.
fn validation_entry(h: &mut Harness<'_, GrandCoachApp>) {
    training_topics(h);
    click(h, Role::Button, "Lernvalidierung vorbereiten");
    assert!(h.state().focus_view().is_none());
}
fn validation_entry_en(h: &mut Harness<'_, GrandCoachApp>) {
    validation_entry(h);
    english(h);
}
fn validation_form(h: &mut Harness<'_, GrandCoachApp>) {
    focus_count(h);
    h.state_mut().preview_validation_controls();
    h.set_size(egui::vec2(1280.0, 1100.0));
    h.run_steps(4);
    assert!(h.state().focus_view().unwrap().entry.validation.is_none());
}
fn validation_form_en(h: &mut Harness<'_, GrandCoachApp>) {
    validation_form(h);
    english(h);
}

#[path = "../tests/support/qa146.rs"]
mod qa146;

fn coach_jump_context(h: &mut Harness<'_, GrandCoachApp>) {
    qa146::jump(h);
    qa146::assert_copy(h, Language::De);
}

fn coach_jump_context_en(h: &mut Harness<'_, GrandCoachApp>) {
    qa146::jump(h);
    qa146::assert_copy(h, Language::En);
}
