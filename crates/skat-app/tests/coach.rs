// SPDX-License-Identifier: EUPL-1.2

//! The live coach through the application interface: intervention levels,
//! staged requests, the important-decision policy, the pre-move warning,
//! the note after the move and the debriefing, stale, cancelled and
//! incomplete results, settings changes, the learning focus, skipping
//! advice, and that the coach never changes the game or the opponents.

use skat_app::coach_model::{Concept, Judgement, Note, Stage, Status, hints_for};
use skat_app::{
    Action, Advice, AppError, AppEvent, Budget, Cancel, CoachEvent, CoachLevel, CoachSettings,
    Command, Debriefing, DebriefingMoment, DecisionContext, Level, Opponents, Pending, Phase,
    SeriesLength, Session, SessionConfig, TableView, Unavailable,
};

/// Cheap but complete for card play late in a game.
const LATE: Budget = Budget {
    samples: 8,
    nodes_per_task: 50_000_000,
};

/// So small that early decisions run out of budget: fast lifecycle tests.
const TINY: Budget = Budget {
    samples: 2,
    nodes_per_task: 20_000,
};

fn session(seed: u64, kontra_re: bool) -> Session {
    Session::new(SessionConfig {
        seed,
        series: SeriesLength::Fixed(4),
        kontra_re,
        opponents: Opponents::Levels([Level::Easy; 2]),
    })
}

fn settings(level: CoachLevel, warnings: bool) -> CoachSettings {
    CoachSettings {
        level,
        warnings,
        show_figures: true,
        focus: None,
    }
}

fn set(session: &mut Session, level: CoachLevel, warnings: bool) {
    session
        .execute(Command::SetCoach(settings(level, warnings)))
        .unwrap();
}

/// The game part of the view: everything except the coach.
fn game_part(view: &TableView) -> TableView {
    let mut view = view.clone();
    view.coach = session(0, false).view().coach;
    view
}

/// Runs the opponents and returns `true` once the human has a decision.
fn to_human(session: &mut Session) -> bool {
    loop {
        session.run_opponents();
        let view = session.view();
        if view.series_finished {
            return false;
        }
        if !view.legal_actions.is_empty() {
            return true;
        }
        session.execute(Command::NextGame).unwrap();
    }
}

/// A simple human: Pass in the bidding (so the opponents declare) and the
/// first legal action otherwise.
fn simple_action(view: &TableView) -> Action {
    if view.phase == Phase::Reizen && view.legal_actions.contains(&Action::Pass) {
        Action::Pass
    } else {
        view.legal_actions[0]
    }
}

/// Runs the due coach job, if any, and stores its result.
fn run_coach(session: &mut Session) -> Vec<AppEvent> {
    match session.coach_job() {
        Some(job) => session.finish_coach(job.run(None)).unwrap(),
        None => Vec::new(),
    }
}

#[test]
fn levels_and_requests() {
    let mut s = session(1, false);
    s.set_coach_budget(TINY);
    assert_eq!(s.coach_settings(), CoachSettings::default());
    // A new session hides the estimates (D-52).
    assert!(!s.coach_settings().show_figures);
    assert!(!s.coach_settings().warnings, "warning off by default");
    assert!(to_human(&mut s));

    // Off: nothing runs, no help action.
    set(&mut s, CoachLevel::Off, false);
    assert!(s.coach_job().is_none());
    let coach = s.view().coach;
    assert!(!coach.help_available && !coach.analysing);
    assert_eq!(coach.advice, Advice::None);
    assert_eq!(s.execute(Command::RequestAdvice), Err(AppError::CoachOff));

    // On request: shown only after asking, though analysed anyway for
    // the note after the move.
    set(&mut s, CoachLevel::OnRequest, false);
    assert!(s.coach_job().is_some_and(|j| !j.is_review()));
    let coach = s.view().coach;
    assert!(coach.help_available && !coach.analysing);
    assert_eq!(coach.advice, Advice::None);
    assert_eq!(coach.next_stage, Some(Stage::Considerations));
    s.execute(Command::RequestAdvice).unwrap();
    assert_eq!(s.view().coach.advice, Advice::Computing);
    assert!(s.view().coach.analysing);
    run_coach(&mut s);
    assert!(s.coach_job().is_none(), "the result is reused");
    assert!(matches!(
        s.view().coach.advice,
        Advice::Ready(_) | Advice::Provisional(_)
    ));
    // The next decision needs a new request.
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    if to_human(&mut s) {
        assert_eq!(s.view().coach.advice, Advice::None);
        assert_eq!(s.view().coach.next_stage, Some(Stage::Considerations));
    }

    // Every move: on its own at every decision with a supported lesson, a
    // compact status at the others.
    set(&mut s, CoachLevel::EveryMove, false);
    for _ in 0..4 {
        if !to_human(&mut s) {
            break;
        }
        assert_eq!(s.view().coach.advice, Advice::Computing);
        run_coach(&mut s);
        let teachable = s
            .coach_result()
            .and_then(|r| r.explanation())
            .is_some_and(|e| e.teachable(None));
        match s.view().coach.advice {
            Advice::Ready(h) | Advice::Provisional(h) => {
                assert!(teachable && h.teachable());
                assert_eq!(h.stage, Stage::Considerations);
            }
            Advice::NoLesson => {
                assert!(!teachable);
                // Asking still shows the first stage.
                let mut asked = s.clone();
                asked.execute(Command::RequestAdvice).unwrap();
                assert!(matches!(
                    asked.view().coach.advice,
                    Advice::Ready(_) | Advice::Provisional(_)
                ));
            }
            other => panic!("{other:?}"),
        }
        assert!(s.view().coach.help_available);
        let action = simple_action(&s.view());
        s.execute(Command::Act(action)).unwrap();
    }

    // No decision of the human: no help, requests rejected.
    let mut s = session(1, false);
    s.execute(Command::Act(Action::Pass)).ok();
    if s.view().opponent_to_move {
        assert!(!s.view().coach.help_available);
        assert_eq!(s.execute(Command::RequestAdvice), Err(AppError::NoDecision));
    }
}

#[test]
fn important_level_shows_exactly_the_important_teachable_decisions() {
    // Automatic teaching needs both the engine's important mark and a
    // supported lesson (pedagogical eligibility); either alone shows
    // nothing on its own.
    let mut s = session(2, true);
    s.set_coach_budget(LATE);
    set(&mut s, CoachLevel::Important, false);
    let (mut shown, mut hidden, mut important_only) = (0, 0, 0);
    let mut decisions = 0;
    while decisions < 120 && to_human(&mut s) {
        let view = s.view();
        // Before card play the analysis is slow; decide quickly there.
        if view.phase == Phase::Spiel {
            let job = s.coach_job().expect("due at every decision");
            let result = job.run(None);
            let explanation = result.explanation().cloned();
            s.finish_coach(result).unwrap();
            let explanation = explanation.expect("analysed");
            let important = explanation.status == Status::Complete && explanation.important;
            let teachable = explanation.teachable(None);
            match s.view().coach.advice {
                Advice::Ready(e) => {
                    assert!(important && teachable);
                    // The situation shows on its own, the figures only on
                    // request, and never the ranking.
                    assert_eq!(*e, hints_for(&explanation, Stage::Situation, None));
                    shown += 1;
                }
                Advice::None => {
                    assert!(!(important && teachable));
                    if important {
                        important_only += 1;
                    }
                    hidden += 1;
                    // The help action still explains it.
                    s.execute(Command::RequestAdvice).unwrap();
                    assert!(matches!(
                        s.view().coach.advice,
                        Advice::Ready(_) | Advice::Provisional(_)
                    ));
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        let action = simple_action(&s.view());
        s.execute(Command::Act(action)).unwrap();
        decisions += 1;
    }
    assert!(
        shown > 0 && hidden > 0,
        "shown {shown}, hidden {hidden}, important without lesson {important_only}"
    );
}

/// A session at a human card-play decision with a weak option, its
/// complete explanation's weak action and a not weak one.
fn weak_position() -> (Session, Action, Action) {
    for seed in 0..60 {
        let mut s = session(seed, false);
        s.set_coach_budget(LATE);
        set(&mut s, CoachLevel::Off, true);
        while to_human(&mut s) {
            let view = s.view();
            if view.phase == Phase::Spiel
                && view.tricks_played >= 5
                && view.legal_actions.len() >= 2
            {
                let job = s.coach_job().expect("warnings need the analysis");
                let result = job.run(None);
                let e = result.explanation().cloned().unwrap();
                if e.status == Status::Complete
                    && let Some(weak) = e.options.iter().find(|o| o.judgement == Judgement::Weak)
                {
                    s.finish_coach(result).unwrap();
                    let fine = e.options[0].action;
                    return (s, weak.action, fine);
                }
            }
            let action = simple_action(&view);
            // Without a finished analysis the action is held; let it pass.
            let events = s.execute(Command::Act(action)).unwrap();
            if events.iter().any(|e| matches!(e, AppEvent::Coach(_))) {
                s.execute(Command::ProceedAnyway).unwrap();
            }
        }
    }
    panic!("no weak option found");
}

#[test]
fn warning_holds_a_weak_action_until_proceed_or_reconsider() {
    let (mut s, weak, fine) = weak_position();
    let before = game_part(&s.view());

    // Warned: nothing happens to the game.
    let events = s.execute(Command::Act(weak)).unwrap();
    assert_eq!(
        events,
        vec![AppEvent::Coach(CoachEvent::Warned { action: weak })]
    );
    assert_eq!(game_part(&s.view()), before);
    match &s.view().coach.pending {
        Some(Pending::Warning { action, warning }) => {
            assert_eq!(*action, weak);
            assert_eq!(warning.chosen.action, weak);
            assert_eq!(warning.chosen.judgement, Judgement::Weak);
        }
        other => panic!("{other:?}"),
    }
    // Another action must wait for the answer.
    assert_eq!(s.execute(Command::Act(fine)), Err(AppError::ActionPending));

    // Reconsider: back to the decision, game unchanged.
    let events = s.execute(Command::Reconsider).unwrap();
    assert_eq!(
        events,
        vec![AppEvent::Coach(CoachEvent::Withdrawn { action: weak })]
    );
    assert_eq!(game_part(&s.view()), before);
    assert_eq!(s.view().coach.pending, None);
    assert_eq!(
        s.execute(Command::Reconsider),
        Err(AppError::NothingPending)
    );

    // Choosing it again at the same position is not warned twice.
    let mut again = s.clone();
    let events = again.execute(Command::Act(weak)).unwrap();
    assert!(events.iter().all(|e| !matches!(e, AppEvent::Coach(_))));
    assert_ne!(game_part(&again.view()), before);

    // A not weak action passes at once.
    let mut other = s.clone();
    let events = other.execute(Command::Act(fine)).unwrap();
    assert!(events.iter().all(|e| !matches!(e, AppEvent::Coach(_))));

    // Proceed: the held action is played exactly once.
    let (mut s, weak, _) = weak_position();
    let hand = s.view().hand.len();
    s.execute(Command::Act(weak)).unwrap();
    let events = s.execute(Command::ProceedAnyway).unwrap();
    let played = events
        .iter()
        .filter(|e| {
            matches!(e, AppEvent::Game { event: skat_app::Event::CardPlayed { .. }, player }
                if *player == Some(skat_app::PlayerId::HUMAN))
        })
        .count();
    assert_eq!(played, 1);
    assert_eq!(s.view().hand.len(), hand - 1);
    assert_eq!(
        s.execute(Command::ProceedAnyway),
        Err(AppError::NothingPending)
    );
}

#[test]
fn equivalent_or_uncertain_options_are_not_warned() {
    // With warnings on, an option that is equivalent to the best, or worse
    // only within the noise or by less than the weak threshold, passes at
    // once. Early decisions run out of the small budget: no warning either.
    let mut checked = 0;
    for seed in 0..8 {
        let mut s = session(seed, false);
        s.set_coach_budget(LATE);
        set(&mut s, CoachLevel::Off, true);
        while to_human(&mut s) {
            let view = s.view();
            let mut action = simple_action(&view);
            if view.phase == Phase::Spiel && view.tricks_played >= 5 {
                let result = s.coach_job().expect("due").run(None);
                let e = result.explanation().cloned().unwrap();
                s.finish_coach(result).unwrap();
                if let Some(o) = e
                    .options
                    .iter()
                    .skip(1)
                    .find(|o| matches!(o.judgement, Judgement::Equivalent | Judgement::Worse))
                {
                    action = o.action;
                    checked += 1;
                    let events = s.execute(Command::Act(action)).unwrap();
                    assert!(
                        events.iter().all(|e| !matches!(e, AppEvent::Coach(_))),
                        "{:?} judged {:?}",
                        action,
                        o.judgement
                    );
                    continue;
                }
            }
            let events = s.execute(Command::Act(action)).unwrap();
            if events.iter().any(|e| matches!(e, AppEvent::Coach(_))) {
                s.execute(Command::ProceedAnyway).unwrap();
            }
        }
    }
    assert!(checked > 0);
}

#[test]
fn checking_waits_for_the_analysis() {
    let mut s = session(3, false);
    s.set_coach_budget(TINY);
    set(&mut s, CoachLevel::Off, true);
    assert!(to_human(&mut s));
    let before = game_part(&s.view());
    let action = simple_action(&s.view());
    let job = s.coach_job().expect("warnings need the analysis");
    let events = s.execute(Command::Act(action)).unwrap();
    assert_eq!(
        events,
        vec![AppEvent::Coach(CoachEvent::Checking { action })]
    );
    assert_eq!(game_part(&s.view()), before);
    assert_eq!(s.view().coach.pending, Some(Pending::Checking { action }));
    // TINY runs out of budget: no finished analysis to judge it, so the
    // action is played as chosen, exactly once.
    let result = job.run(None);
    assert_ne!(
        result.explanation().map(|e| e.status),
        Some(Status::Complete)
    );
    let events = s.finish_coach(result).unwrap();
    assert!(events.iter().any(|e| matches!(e, AppEvent::Game { .. })));
    assert_ne!(game_part(&s.view()), before);
    assert_eq!(s.view().coach.pending, None);

    // An illegal action is rejected, not held.
    let mut s = session(3, false);
    set(&mut s, CoachLevel::Off, true);
    assert!(to_human(&mut s));
    assert!(matches!(
        s.execute(Command::Act(Action::Re)),
        Err(AppError::Rule(_))
    ));
    assert_eq!(s.view().coach.pending, None);
}

#[test]
fn stale_cancelled_and_incomplete_results() {
    let mut s = session(4, false);
    s.set_coach_budget(TINY);
    set(&mut s, CoachLevel::EveryMove, false);
    assert!(to_human(&mut s));

    // Played before the result arrived: it still backs the note.
    let job = s.coach_job().unwrap();
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    let after = game_part(&s.view());
    let result = job.run(None);
    assert_eq!(s.finish_coach(result.clone()), Ok(Vec::new()));
    assert_eq!(game_part(&s.view()), after);
    // A second time it belongs to no decision any more.
    assert_eq!(s.finish_coach(result), Err(AppError::StaleCoachResult));

    // Stale: computed with another budget.
    assert!(to_human(&mut s));
    let job = s.coach_job().unwrap();
    s.set_coach_budget(Budget { samples: 3, ..TINY });
    assert_eq!(
        s.finish_coach(job.run(None)),
        Err(AppError::StaleCoachResult)
    );
    assert!(
        s.coach_job().is_some(),
        "computed again with the new budget"
    );

    // Cancelled: never shown as advice; asking again recomputes.
    let job = s.coach_job().unwrap();
    let cancel = Cancel::new();
    cancel.cancel();
    let before = game_part(&s.view());
    s.finish_coach(job.run(Some(&cancel))).unwrap();
    assert_eq!(
        s.view().coach.advice,
        Advice::Unavailable(Unavailable::Cancelled)
    );
    assert_eq!(game_part(&s.view()), before);
    assert!(s.coach_job().is_none());
    s.execute(Command::RequestAdvice).unwrap();
    assert!(s.coach_job().is_some());

    // Incomplete: shown as provisional, never as ready.
    run_coach(&mut s);
    match s.view().coach.advice {
        Advice::Provisional(e) => assert_eq!(e.status, Status::BudgetExhausted),
        Advice::Ready(e) => assert_eq!(e.status, Status::Complete),
        other => panic!("{other:?}"),
    }
    assert_eq!(game_part(&s.view()), before);
}

#[test]
fn settings_changes_keep_the_game() {
    let mut s = session(5, false);
    s.set_coach_budget(TINY);
    assert!(to_human(&mut s));
    let before = game_part(&s.view());
    for level in CoachLevel::ALL {
        for warnings in [false, true] {
            for show_figures in [false, true] {
                let settings = CoachSettings {
                    level,
                    warnings,
                    show_figures,
                    focus: None,
                };
                s.execute(Command::SetCoach(settings)).unwrap();
                assert_eq!(s.view().coach.settings, settings);
                assert_eq!(game_part(&s.view()), before);
            }
        }
        for focus in std::iter::once(None).chain(Concept::ALL.into_iter().map(Some)) {
            let settings = CoachSettings {
                level,
                focus,
                ..CoachSettings::default()
            };
            s.execute(Command::SetCoach(settings)).unwrap();
            assert_eq!(s.view().coach.settings, settings);
            assert_eq!(game_part(&s.view()), before);
        }
    }
    // Turning warnings off withdraws a held action without playing it.
    set(&mut s, CoachLevel::Off, true);
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    assert!(s.view().coach.pending.is_some());
    set(&mut s, CoachLevel::Off, false);
    assert_eq!(s.view().coach.pending, None);
    assert_eq!(game_part(&s.view()), before);
    // Off without warnings is ordinary, unassisted play.
    let events = s.execute(Command::Act(action)).unwrap();
    assert!(events.iter().all(|e| !matches!(e, AppEvent::Coach(_))));
}

/// Plays a series with the simple human; with `coached` the coach analyses
/// every decision and warns, and the human proceeds after every warning.
fn series_events(seed: u64, coached: bool) -> (Vec<AppEvent>, TableView) {
    let mut s = session(seed, true);
    s.set_coach_budget(TINY);
    if coached {
        set(&mut s, CoachLevel::EveryMove, true);
    } else {
        set(&mut s, CoachLevel::Off, false);
    }
    let mut events = Vec::new();
    loop {
        events.extend(s.run_opponents());
        let view = s.view();
        if view.series_finished {
            break;
        }
        if view.legal_actions.is_empty() {
            events.extend(s.execute(Command::NextGame).unwrap());
            continue;
        }
        if coached {
            run_coach(&mut s);
        }
        let mut out = s.execute(Command::Act(simple_action(&view))).unwrap();
        if out.iter().any(|e| matches!(e, AppEvent::Coach(_))) {
            out = s.execute(Command::ProceedAnyway).unwrap();
        }
        events.extend(out);
    }
    events.retain(|e| !matches!(e, AppEvent::Coach(_)));
    let view = game_part(&s.view());
    (events, view)
}

#[test]
fn the_coach_never_changes_the_game_or_the_opponents() {
    let (plain, plain_view) = series_events(6, false);
    let (coached, coached_view) = series_events(6, true);
    assert_eq!(plain, coached);
    assert_eq!(plain_view.sheet, coached_view.sheet);
}

#[test]
fn coach_output_is_reproducible() {
    let mut a = session(7, false);
    let mut b = session(7, false);
    for s in [&mut a, &mut b] {
        s.set_coach_budget(TINY);
        set(s, CoachLevel::EveryMove, false);
        assert!(to_human(s));
    }
    let (x, y) = (a.coach_job().unwrap(), b.coach_job().unwrap());
    assert_eq!(x.seed(), y.seed());
    assert_eq!(x.run(None), y.run(None));
    // Another series seed gives another coach seed.
    let mut c = session(8, false);
    c.set_coach_budget(TINY);
    set(&mut c, CoachLevel::EveryMove, false);
    assert!(to_human(&mut c));
    assert_ne!(c.coach_job().unwrap().seed(), x.seed());
}

#[test]
fn requests_reveal_one_stage_at_a_time() {
    let mut s = session(9, false);
    s.set_coach_budget(TINY);
    // Figures are off by default; shown explicitly here.
    set(&mut s, CoachLevel::OnRequest, false);
    assert!(to_human(&mut s));
    run_coach(&mut s);
    let stage = |s: &Session| match s.view().coach.advice {
        Advice::Ready(h) | Advice::Provisional(h) => Some(h.stage),
        _ => None,
    };
    assert_eq!(stage(&s), None);
    for expected in [Stage::Considerations, Stage::Situation, Stage::Figures] {
        assert_eq!(s.view().coach.next_stage, Some(expected));
        s.execute(Command::RequestAdvice).unwrap();
        assert_eq!(stage(&s), Some(expected));
    }
    assert_eq!(s.view().coach.next_stage, None);
    s.execute(Command::RequestAdvice).unwrap();
    assert_eq!(stage(&s), Some(Stage::Figures), "nothing more to reveal");
    match s.view().coach.advice {
        Advice::Ready(h) | Advice::Provisional(h) => {
            let figures = h.figures.expect("figures stage");
            let options = h.situation.expect("situation stage").options;
            assert!(figures.iter().map(|f| f.action).eq(options.iter().copied()));
            assert_eq!(options, s.view().legal_actions, "neutral order");
        }
        other => panic!("{other:?}"),
    }

    // Without figures the situation is the last stage.
    let mut settings = s.coach_settings();
    settings.show_figures = false;
    s.execute(Command::SetCoach(settings)).unwrap();
    assert_eq!(stage(&s), Some(Stage::Situation));
    assert_eq!(s.view().coach.next_stage, None);

    // Every move shows the considerations on its own where a lesson
    // applies, otherwise the compact status and the first stage on request.
    set(&mut s, CoachLevel::EveryMove, false);
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    if to_human(&mut s) {
        run_coach(&mut s);
        if s.view().coach.advice == Advice::NoLesson {
            assert_eq!(s.view().coach.next_stage, Some(Stage::Considerations));
        } else {
            assert_eq!(stage(&s), Some(Stage::Considerations));
            assert_eq!(s.view().coach.next_stage, Some(Stage::Situation));
        }
    }
}

#[test]
fn figures_off_stops_at_the_situation_and_toggling_keeps_the_request_flow() {
    // D-49/D-60: with figures off, repeated requests stop at stage 2 and
    // expose no estimate; switching figures on reveals nothing by itself,
    // switching them off hides the estimates at once. The exact facts and
    // the situation do not depend on the setting.
    let mut s = session(9, false);
    s.set_coach_budget(TINY);
    let mut off = settings(CoachLevel::OnRequest, false);
    off.show_figures = false;
    let on = CoachSettings {
        show_figures: true,
        ..off
    };
    s.execute(Command::SetCoach(off)).unwrap();
    assert!(to_human(&mut s));
    run_coach(&mut s);
    let game = game_part(&s.view());
    let shown = |s: &Session| match s.view().coach.advice {
        Advice::Ready(h) | Advice::Provisional(h) => Some(*h),
        _ => None,
    };
    for _ in 0..5 {
        s.execute(Command::RequestAdvice).unwrap();
    }
    let situation = shown(&s).expect("advice on request");
    assert_eq!(situation.stage, Stage::Situation, "stage 2 is the last");
    assert!(situation.figures.is_none(), "no estimate with figures off");
    assert_eq!(s.view().coach.next_stage, None);

    // Switching figures on reveals nothing until the next request.
    s.execute(Command::SetCoach(on)).unwrap();
    assert_eq!(shown(&s).as_ref(), Some(&situation));
    assert_eq!(s.view().coach.next_stage, Some(Stage::Figures));
    s.execute(Command::RequestAdvice).unwrap();
    let figures = shown(&s).expect("advice");
    assert_eq!(figures.stage, Stage::Figures);
    let options = situation
        .situation
        .as_ref()
        .expect("situation")
        .options
        .clone();
    assert!(
        figures
            .figures
            .as_ref()
            .expect("estimates on request")
            .iter()
            .map(|f| f.action)
            .eq(options.iter().copied()),
        "neutral order"
    );
    // Stage 3 only adds the figures: same facts, situation and question.
    let mut without = figures.clone();
    without.figures = None;
    without.stage = Stage::Situation;
    assert_eq!(without, situation);

    // Switching off after stage 3 hides the estimates at once and caps
    // further requests at stage 2.
    s.execute(Command::SetCoach(off)).unwrap();
    assert_eq!(shown(&s).as_ref(), Some(&situation));
    s.execute(Command::RequestAdvice).unwrap();
    assert_eq!(shown(&s).as_ref(), Some(&situation));
    assert_eq!(s.view().coach.next_stage, None);
    // Switching on again shows the stage 3 the player already asked for
    // at this decision, nothing beyond it.
    s.execute(Command::SetCoach(on)).unwrap();
    assert_eq!(shown(&s).as_ref(), Some(&figures));
    assert_eq!(s.view().coach.next_stage, None);
    // Toggles and requests changed nothing in the game and started no new
    // analysis.
    assert_eq!(game, game_part(&s.view()));
    assert!(s.coach_job().is_none());

    // At the next decision figures on still start at stage 1: enabling
    // them never skips the request flow.
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    assert!(to_human(&mut s), "precondition: a second decision");
    run_coach(&mut s);
    assert_eq!(shown(&s), None, "on request: nothing before asking");
    assert_eq!(s.view().coach.next_stage, Some(Stage::Considerations));
}

#[test]
fn every_own_move_gets_a_note_even_if_analysed_afterwards() {
    let mut s = session(10, false);
    s.set_coach_budget(TINY);
    assert!(to_human(&mut s));
    assert_eq!(s.view().coach.last_move, None);

    // Analysed before the move: the note is there at once.
    run_coach(&mut s);
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    let last = s.view().coach.last_move.expect("a note");
    assert_eq!(last.action, action);
    assert!(last.note.is_some());

    // Played before the analysis: computed afterwards, the current
    // decision first.
    assert!(to_human(&mut s));
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    let last = s.view().coach.last_move.expect("a note");
    assert_eq!((last.action, last.note), (action, None));
    if to_human(&mut s) {
        let job = s.coach_job().unwrap();
        assert!(!job.is_review(), "the current decision goes first");
        s.finish_coach(job.run(None)).unwrap();
    }
    let job = s.coach_job().expect("the review is queued");
    assert!(job.is_review());
    // A cancelled review stays queued.
    let cancel = Cancel::new();
    cancel.cancel();
    s.finish_coach(job.run(Some(&cancel))).unwrap();
    let job = s.coach_job().expect("still queued");
    assert!(job.is_review());
    s.finish_coach(job.run(None)).unwrap();
    let last = s.view().coach.last_move.unwrap();
    assert_eq!(last.action, action);
    // TINY: the early decisions are not judged reliably.
    assert!(last.note.is_some());

    // Off: no notes.
    set(&mut s, CoachLevel::Off, false);
    assert_eq!(s.view().coach.last_move, None);
}

#[test]
fn debriefing_shows_teachable_moments_after_the_game() {
    let (mut s, weak, _) = weak_position();
    let focus = Some(Concept::RemainingTrumpf);
    s.execute(Command::SetCoach(CoachSettings {
        focus,
        ..settings(CoachLevel::EveryMove, true)
    }))
    .unwrap();
    s.execute(Command::Act(weak)).unwrap();
    s.execute(Command::ProceedAnyway).unwrap();
    let last = s.view().coach.last_move.expect("a note");
    assert_eq!((last.action, last.note), (weak, Some(Note::ClearlyWeaker)));
    assert_eq!(s.view().coach.debriefing, None, "only after the game");
    let mut forced = false;
    loop {
        s.run_opponents();
        if s.view().legal_actions.is_empty() {
            break;
        }
        run_coach(&mut s);
        forced = s.view().legal_actions.len() == 1;
        let action = simple_action(&s.view());
        let events = s.execute(Command::Act(action)).unwrap();
        if events.iter().any(|e| matches!(e, AppEvent::Coach(_))) {
            s.execute(Command::ProceedAnyway).unwrap();
        }
    }
    while s.coach_job().is_some() {
        run_coach(&mut s);
    }
    // The last card of a game is forced: its note does not repeat above
    // the debriefing.
    assert!(forced, "the game ran to its last trick");
    assert_eq!(s.view().coach.last_move, None);
    let Some(Debriefing::Ready(moments)) = s.view().coach.debriefing else {
        panic!("debriefing ready");
    };
    // Zero to three moments, each with a supported lesson of its own
    // concept, never forced, in game order.
    assert!(moments.len() <= 3);
    let moments: Vec<_> = moments.into_iter().map(|m| m.moment).collect();
    let mut concepts: Vec<Concept> = moments.iter().map(|m| m.concept).collect();
    concepts.dedup();
    assert_eq!(concepts.len(), moments.len(), "one per concept");
    let tricks: Vec<_> = moments.iter().map(|m| m.review.trick).collect();
    assert!(tricks.is_sorted(), "{tricks:?}");
    for m in &moments {
        assert!(!m.review.forced);
        let taught = m.review.lesson_for(focus).expect("a supported lesson");
        assert_eq!((taught.concept, &taught.content), (m.concept, &m.lesson));
        assert_eq!(m.focused, Some(m.concept) == focus);
    }
    // Changing the focus after the game re-selects from the same reviews
    // without any new analysis or change to the game.
    let before = game_part(&s.view());
    s.execute(Command::SetCoach(CoachSettings {
        focus: None,
        ..s.coach_settings()
    }))
    .unwrap();
    assert!(s.coach_job().is_none(), "no new analysis");
    assert_eq!(game_part(&s.view()), before);
    assert!(matches!(
        s.view().coach.debriefing,
        Some(Debriefing::Ready(m)) if m.iter().all(|m| !m.moment.focused)
    ));
    // The next game starts afresh; explicit choices, the focus included,
    // stay.
    s.execute(Command::SetCoach(CoachSettings {
        focus,
        ..s.coach_settings()
    }))
    .unwrap();
    if !s.view().series_finished {
        s.execute(Command::NextGame).unwrap();
        let coach = s.view().coach;
        assert_eq!((coach.last_move, coach.debriefing), (None, None));
        assert!(coach.settings.show_figures);
        assert_eq!(coach.settings.level, CoachLevel::EveryMove);
        assert_eq!(coach.settings.focus, focus);
    }
}

/// A session at a human card-play decision late in a game whose finished
/// analysis teaches both `proven-void` and `remaining-trumpf`; no analysis
/// is due. The coach is off until then, so nothing is queued.
fn two_lessons() -> Session {
    for seed in 0..40 {
        let mut s = session(seed, false);
        s.set_coach_budget(LATE);
        set(&mut s, CoachLevel::Off, false);
        while to_human(&mut s) {
            if s.view().phase == Phase::Spiel && s.view().tricks_played >= 5 {
                set(&mut s, CoachLevel::OnRequest, false);
                run_coach(&mut s);
                let e = s.coach_result().and_then(|r| r.explanation()).cloned();
                if e.is_some_and(|e| {
                    e.teachable(Some(Concept::ProvenVoid))
                        && e.teachable(Some(Concept::RemainingTrumpf))
                }) {
                    assert!(s.coach_job().is_none());
                    return s;
                }
                set(&mut s, CoachLevel::Off, false);
            }
            let action = simple_action(&s.view());
            s.execute(Command::Act(action)).unwrap();
        }
    }
    panic!("no decision with two lessons");
}

#[test]
fn focus_changes_reuse_the_analysis_and_keep_the_game() {
    let mut s = two_lessons();
    let before = game_part(&s.view());
    let unasked = s.clone();
    s.execute(Command::RequestAdvice).unwrap();
    let concept = |s: &Session| match s.view().coach.advice {
        Advice::Ready(h) | Advice::Provisional(h) => h.concept,
        other => panic!("{other:?}"),
    };
    assert_eq!(concept(&s), Some(Concept::ProvenVoid), "catalogue order");
    for (focus, expected) in [
        (Some(Concept::RemainingTrumpf), Concept::RemainingTrumpf),
        (Some(Concept::ProvenVoid), Concept::ProvenVoid),
        (Some(Concept::GameValue), Concept::ProvenVoid),
        (None, Concept::ProvenVoid),
    ] {
        s.execute(Command::SetCoach(CoachSettings {
            focus,
            ..s.coach_settings()
        }))
        .unwrap();
        assert_eq!(concept(&s), Some(expected), "{focus:?}");
        // No new search for a focus change or a hint stage.
        assert!(s.coach_job().is_none());
        assert_eq!(game_part(&s.view()), before);
    }
    // Every move with a focus that does not apply here: compact status;
    // with an applying focus: its lesson on its own (nothing requested).
    for (focus, expected) in [
        (Concept::HandStructure, None),
        (Concept::RemainingTrumpf, Some(Concept::RemainingTrumpf)),
    ] {
        let mut t = unasked.clone();
        t.execute(Command::SetCoach(CoachSettings {
            focus: Some(focus),
            ..settings(CoachLevel::EveryMove, false)
        }))
        .unwrap();
        match (t.view().coach.advice, expected) {
            (Advice::NoLesson, None) => {}
            (Advice::Ready(h) | Advice::Provisional(h), Some(c)) => {
                assert_eq!(h.concept, Some(c));
            }
            (other, _) => panic!("{focus:?}: {other:?}"),
        }
        assert!(t.coach_job().is_none());
        assert_eq!(game_part(&t.view()), before);
    }
    // The note after the move shows the focused lesson.
    s.execute(Command::SetCoach(CoachSettings {
        focus: Some(Concept::RemainingTrumpf),
        ..s.coach_settings()
    }))
    .unwrap();
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    let last = s.view().coach.last_move.unwrap();
    assert!(matches!(
        last.lesson,
        Some(skat_app::coach_model::LessonContent::RemainingTrumpf { .. })
    ));
}

#[test]
fn skipping_advice_is_optional_and_keeps_the_game() {
    let mut s = two_lessons();
    let mut off = s.clone();
    set(&mut s, CoachLevel::EveryMove, false);
    let before = game_part(&s.view());
    assert!(matches!(s.view().coach.advice, Advice::Ready(_)));
    assert!(s.view().coach.skippable);
    let job = s.coach_job().is_some();
    s.execute(Command::SkipAdvice).unwrap();
    let coach = s.view().coach;
    assert_eq!(coach.advice, Advice::None);
    assert!(!coach.skippable && !coach.analysing && coach.help_available);
    assert_eq!(coach.next_stage, Some(Stage::Considerations));
    assert_eq!(game_part(&s.view()), before);
    assert_eq!(s.coach_job().is_some(), job, "analyses unchanged");
    assert_eq!(s.coach_settings(), settings(CoachLevel::EveryMove, false));
    // Asking brings it back; skipping a requested hint hides it again.
    s.execute(Command::RequestAdvice).unwrap();
    assert!(matches!(s.view().coach.advice, Advice::Ready(_)));
    s.execute(Command::SkipAdvice).unwrap();
    assert_eq!(s.view().coach.advice, Advice::None);
    // Playing on works as always; the note still comes.
    let action = simple_action(&s.view());
    s.execute(Command::Act(action)).unwrap();
    assert!(s.view().coach.last_move.is_some_and(|m| m.note.is_some()));
    // The next decision shows the coach again on its own.
    if to_human(&mut s) {
        assert_ne!(s.view().coach.advice, Advice::None);
    }
    // Off or without a decision: rejected, nothing changes.
    set(&mut off, CoachLevel::Off, false);
    let before = game_part(&off.view());
    assert_eq!(off.execute(Command::SkipAdvice), Err(AppError::CoachOff));
    assert_eq!(game_part(&off.view()), before);
    let mut idle = session(1, false);
    idle.execute(Command::Act(Action::Pass)).ok();
    if idle.view().opponent_to_move {
        assert_eq!(idle.execute(Command::SkipAdvice), Err(AppError::NoDecision));
    }
}

#[test]
fn warnings_keep_their_semantics_with_any_focus_and_level() {
    // Warnings stay an engine judgement: they need no lesson.
    for (level, focus) in [
        (CoachLevel::Off, None),
        (CoachLevel::Important, Some(Concept::HandStructure)),
        (CoachLevel::EveryMove, Some(Concept::PartnerOvertake)),
    ] {
        let (mut s, weak, _) = weak_position();
        s.execute(Command::SetCoach(CoachSettings {
            focus,
            ..settings(level, true)
        }))
        .unwrap();
        let before = game_part(&s.view());
        let events = s.execute(Command::Act(weak)).unwrap();
        assert_eq!(
            events,
            vec![AppEvent::Coach(CoachEvent::Warned { action: weak })]
        );
        assert_eq!(game_part(&s.view()), before);
        if level != CoachLevel::Off {
            // Skipping the advice does not release the held action.
            s.execute(Command::SkipAdvice).unwrap();
            assert!(s.view().coach.pending.is_some());
        }
        s.execute(Command::ProceedAnyway).unwrap();
        assert_ne!(game_part(&s.view()), before);
    }
}

/// What the table showed the human at one own decision, and the action.
#[derive(Clone, Debug)]
struct Seen {
    view: TableView,
    action: Action,
}

/// `true` if `c` is exactly the snapshot of `seen`: the hand, trick,
/// bidding, contract, seats and choice of that moment.
fn matches(c: &DecisionContext, seen: &Seen) -> bool {
    let v = &seen.view;
    c.action == seen.action
        && c.game_number == v.game_number
        && c.dealer == v.dealer
        && c.phase == v.phase
        && c.hand == v.hand
        && c.current_trick == v.current_trick
        && c.tricks.last() == v.last_trick.as_ref()
        && c.tricks.len() == usize::from(v.tricks_played)
        && c.bid == v.bid
        && c.bidding == v.bidding
        && c.declarer == v.declarer
        && c.hand_game == v.hand_game
        && c.contract == v.contract
        && c.kontra == v.kontra
        && c.own_skat == v.own_skat
        && c.declarer_cards == v.declarer_cards
        && v.players.iter().all(|p| {
            c.seat(p.player) == p.seat
                && c.player(p.seat) == p.player
                && c.hand_sizes[p.player.index()] == p.cards
        })
}

/// Checks that every moment carries the snapshot of its own decision,
/// in game order, consistent with its review; returns the matched
/// decisions.
fn check_moments(moments: &[DebriefingMoment], seen: &[Seen]) -> Vec<usize> {
    let mut at = Vec::new();
    for m in moments {
        let found: Vec<usize> = (0..seen.len())
            .filter(|&i| matches(&m.context, &seen[i]))
            .collect();
        assert_eq!(found.len(), 1, "exactly one decision: {:?}", m.context);
        let r = &m.moment.review;
        assert_eq!(r.chosen.action, m.context.action);
        assert_eq!((r.bid, r.hand_game), (m.context.bid, m.context.hand_game));
        assert_eq!(r.trick, m.context.trick());
        at.push(found[0]);
    }
    assert!(at.is_sorted(), "game order: {at:?}");
    at
}

/// Plays one game to its end without finishing any coach analysis: the
/// jobs for the current decisions are returned, newest last.
fn play_unanalysed(s: &mut Session) -> (Vec<Seen>, Vec<skat_app::CoachJob>) {
    let (mut seen, mut jobs) = (Vec::new(), Vec::new());
    loop {
        s.run_opponents();
        let view = s.view();
        if view.legal_actions.is_empty() {
            return (seen, jobs);
        }
        let job = s.coach_job().expect("an analysis is due");
        assert!(!job.is_review(), "the current decision first");
        jobs.push(job);
        let action = simple_action(&view);
        seen.push(Seen { view, action });
        s.execute(Command::Act(action)).unwrap();
    }
}

#[test]
fn queued_and_interrupted_reviews_keep_their_own_snapshots() {
    let (mut checked, mut partial, mut preplay) = (0, 0, 0);
    let mut s = session(2, true);
    s.set_coach_budget(TINY);
    set(&mut s, CoachLevel::EveryMove, false);
    while checked < 4 || partial == 0 || preplay == 0 {
        assert!(!s.view().series_finished, "{checked} {partial} {preplay}");
        let (seen, jobs) = play_unanalysed(&mut s);
        assert_eq!(
            s.view().coach.debriefing,
            Some(Debriefing::Computing {
                remaining: jobs.len() as u32
            })
        );
        // Interrupted: a cancelled analysis keeps its decision queued.
        let cancel = Cancel::new();
        cancel.cancel();
        for job in jobs.iter().rev() {
            assert!(s.finish_coach(job.run(Some(&cancel))).unwrap().is_empty());
        }
        assert!(matches!(
            s.view().coach.debriefing,
            Some(Debriefing::Computing { remaining }) if remaining as usize == jobs.len()
        ));
        // Finished newest first: each still meets its own decision.
        for job in jobs.iter().rev() {
            s.finish_coach(job.run(None)).unwrap();
        }
        assert!(s.coach_job().is_none(), "nothing left to analyse");
        let Some(Debriefing::Ready(moments)) = s.view().coach.debriefing else {
            panic!("debriefing ready");
        };
        for i in check_moments(&moments, &seen) {
            checked += 1;
            partial += usize::from(!seen[i].view.current_trick.is_empty());
            preplay += usize::from(seen[i].view.phase != Phase::Spiel);
        }
        // Serialisable plain data, unchanged by a round trip. The snapshots
        // hold no floats and survive JSON text exactly; the reviews' floats
        // lose their last digit in serde_json's text parser, so the whole
        // view goes through a JSON value.
        for m in &moments {
            let json = serde_json::to_string(&m.context).unwrap();
            assert_eq!(
                serde_json::from_str::<DecisionContext>(&json).unwrap(),
                m.context
            );
        }
        let view = s.view();
        let value = serde_json::to_value(&view).unwrap();
        assert_eq!(serde_json::from_value::<TableView>(value).unwrap(), view);
        // The final hand is not the hand of any moment.
        if let Some(m) = moments.iter().find(|m| !m.context.hand.is_empty()) {
            assert_ne!(m.context.hand, view.hand);
        }
        s.execute(Command::NextGame).unwrap();
    }
}

#[test]
fn reviews_never_cross_into_the_next_game() {
    let mut s = session(3, false);
    s.set_coach_budget(TINY);
    set(&mut s, CoachLevel::EveryMove, false);
    let (_, old) = play_unanalysed(&mut s);
    assert!(!old.is_empty());
    let number = s.view().game_number;
    s.execute(Command::NextGame).unwrap();
    // The new game starts without the old queue, reviews or snapshots.
    assert_eq!(s.view().coach.debriefing, None);
    assert!(s.coach_job().is_none_or(|j| !j.is_review()));
    // Stale: a late result of the old game is rejected and stored nowhere.
    for job in &old {
        assert_eq!(
            s.finish_coach(job.run(None)),
            Err(AppError::StaleCoachResult)
        );
    }
    let (seen, jobs) = play_unanalysed(&mut s);
    for job in &jobs {
        s.finish_coach(job.run(None)).unwrap();
    }
    let Some(Debriefing::Ready(moments)) = s.view().coach.debriefing else {
        panic!("debriefing ready");
    };
    check_moments(&moments, &seen);
    assert!(moments.iter().all(|m| m.context.game_number == number + 1));
}
