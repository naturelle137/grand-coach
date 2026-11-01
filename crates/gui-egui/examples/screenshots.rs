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

use eframe::egui::{Key, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use gui_egui::{Deck, GrandCoachApp, Language, Theme};
use skat_app::coach_model::{Concept, Judgement, LessonContent};
use skat_app::{
    Action, Advice, Budget, Card, CoachLevel, CoachSettings, Debriefing, Level, Opponents, Pending,
    Rank, SeriesLength, SessionConfig, TableView,
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

/// Scenarios in God Mode (D-62); all others play against two Easy
/// opponents.
const GOD_MODE: &[&str] = &[
    "god-mode",
    "god-mode-german",
    "god-mode-hidden",
    "god-mode-trick",
    "god-mode-coach",
];

const SCENARIOS: &[(&str, Scenario)] = &[
    ("launch", launch),
    ("bidding", bidding),
    ("trick", trick),
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
    ("god-mode", god_mode),
    ("god-mode-german", god_mode_german),
    ("god-mode-hidden", god_mode_hidden),
    ("god-mode-trick", god_mode_trick),
    ("god-mode-coach", god_mode_coach),
];

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
        let opponents = if GOD_MODE.contains(name) {
            Opponents::GodMode
        } else {
            Opponents::Levels([Level::Easy; 2])
        };
        let mut harness = harness(*name == "launch", seed, opponents);
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
fn harness(launch: bool, seed: u64, opponents: Opponents) -> Harness<'static, GrandCoachApp> {
    let config = SessionConfig {
        seed,
        series: SeriesLength::Fixed(2),
        kontra_re: true,
        opponents,
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
