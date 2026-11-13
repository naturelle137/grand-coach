// SPDX-License-Identifier: EUPL-1.2

//! The counting aid and the in-game quiz through the application
//! interface (M6, D-64): when questions appear, that they hold the
//! decision without changing the game, answer / skip / switch-off paths,
//! reproducible selection, and that neither the aid nor the coach shows the
//! answer while a question is pending. Expected answers are counted here
//! from the table events, independently of `skat-analysis`.

use skat_app::{
    AppError, AppEvent, AssistSettings, CardSet, CoachLevel, CoachSettings, Command, CountingState,
    Derivation, Event, Level, Opponents, Phase, PlayerId, QUESTION_AFTER_TRICKS, QuestionKind,
    QuizOutcome, SeriesLength, Session, SessionConfig, TableView,
};
use skat_app::{Card, GameType};

fn session(seed: u64) -> Session {
    Session::new(SessionConfig {
        seed,
        series: SeriesLength::Fixed(12),
        kontra_re: false,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    })
}

const ON: AssistSettings = AssistSettings {
    counting_aid: true,
    quiz: true,
    retrieval: false,
    assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
};

/// What the test saw happen in the current game.
#[derive(Default)]
struct Witness {
    played: Vec<Card>,
    /// Augen won by each player in completed tricks.
    won: [u32; 3],
    game: Option<GameType>,
    voids: [Vec<skat_app::Follow>; 3],
}

impl Witness {
    fn note(&mut self, events: &[AppEvent], dealer: PlayerId) {
        for event in events {
            match event {
                AppEvent::GameStarted { .. } => *self = Witness::default(),
                AppEvent::Game {
                    event: Event::Announced { contract, .. },
                    ..
                } => self.game = Some(contract.game),
                AppEvent::Game {
                    event: Event::CardPlayed { seat, card },
                    ..
                } => {
                    if !self.played.len().is_multiple_of(3) {
                        let game = self.game.expect("announced before play");
                        let led = self.played[self.played.len() / 3 * 3];
                        let class = game.follow(led);
                        if game.follow(*card) != class {
                            self.voids[PlayerId::on_seat(*seat, dealer).index()].push(class);
                        }
                    }
                    self.played.push(*card);
                }
                AppEvent::Game {
                    event: Event::TrickWon { trick },
                    ..
                } => {
                    let winner = PlayerId::on_seat(trick.winner, dealer);
                    self.won[winner.index()] += u32::from(trick.augen());
                }
                _ => {}
            }
        }
    }
}

/// The human plays the first legal action, opponents move, until a quiz
/// question is pending. `None` if the series ends first.
fn until_question(session: &mut Session, witness: &mut Witness) -> Option<TableView> {
    for _ in 0..2000 {
        let events = session.run_opponents();
        let dealer = session.view().dealer;
        witness.note(&events, dealer);
        let view = session.view();
        if view.assist.quiz.question.is_some() {
            return Some(view);
        }
        if view.series_finished {
            return None;
        }
        let command = match view.legal_actions.first() {
            Some(&action) => Command::Act(action),
            None => Command::NextGame,
        };
        let events = session.execute(command).unwrap();
        let dealer = session.view().dealer;
        witness.note(&events, dealer);
    }
    None
}

/// The exact answer counted from the witnessed play and the permitted
/// view, with the rules of the announced game.
fn expected(view: &TableView, witness: &Witness, kind: QuestionKind) -> u8 {
    let game = view.contract.expect("announced").game;
    match kind {
        QuestionKind::ProvenVoids { class } => PlayerId::ALL
            .into_iter()
            .filter(|p| !p.is_human() && witness.voids[p.index()].contains(&class))
            .count() as u8,
        QuestionKind::TrumpfOutside => {
            let mut known: CardSet = view.hand.iter().copied().collect();
            known |= witness.played.iter().copied().collect();
            known |= view.own_skat.iter().flatten().copied().collect::<CardSet>();
            (game.trumps().iter().filter(|&c| !known.contains(c)).count()) as u8
        }
        QuestionKind::AugenWon { declarer } => {
            let d = view.declarer.expect("declarer");
            let sum: u32 = PlayerId::ALL
                .into_iter()
                .filter(|&p| (p == d) == declarer)
                .map(|p| witness.won[p.index()])
                .sum();
            sum as u8
        }
        QuestionKind::ClassPlayed { class } => witness
            .played
            .iter()
            .filter(|&&c| game.follow(c) == class)
            .count() as u8,
    }
}

#[test]
fn nothing_is_shown_by_default() {
    let mut s = session(1);
    assert_eq!(s.view().assist.settings, AssistSettings::default());
    assert_eq!(s.view().assist.counting, CountingState::Off);
    let mut w = Witness::default();
    for _ in 0..40 {
        s.run_opponents();
        let view = s.view();
        assert_eq!(view.assist.quiz.question, None);
        if let Some(&a) = view.legal_actions.first() {
            let events = s.execute(Command::Act(a)).unwrap();
            w.note(&events, view.dealer);
        }
    }
}

#[test]
fn questions_come_after_three_and_six_tricks_and_hold_the_decision() {
    let mut seen = 0;
    for seed in 1..30 {
        let mut s = session(seed);
        s.execute(Command::SetAssist(ON)).unwrap();
        let mut w = Witness::default();
        let Some(view) = until_question(&mut s, &mut w) else {
            continue;
        };
        seen += 1;
        assert_eq!(view.phase, Phase::Spiel);
        assert!(QUESTION_AFTER_TRICKS.contains(&view.tricks_played));
        assert_eq!(view.acting, Some(PlayerId::HUMAN));
        assert!(!view.legal_actions.is_empty());
        // The aid and the coach are withheld while the question is pending.
        assert_eq!(view.assist.counting, CountingState::Withheld);
        assert!(!view.coach.help_available);
        assert!(view.coach.last_move.is_none() && view.coach.debriefing.is_none());

        // The decision is held: no game action, no coach request.
        let action = view.legal_actions[0];
        let recording = s.export_current_recording_privileged().unwrap();
        for command in [
            Command::Act(action),
            Command::RequestAdvice,
            Command::SkipAdvice,
            Command::ProceedAnyway,
            Command::Reconsider,
        ] {
            assert_eq!(s.execute(command), Err(AppError::QuestionPending));
            assert_eq!(s.view(), view, "{command:?} changed something");
            assert_eq!(s.export_current_recording_privileged().unwrap(), recording);
        }
        // Changing assistance or coach settings keeps the game and the
        // question as they are.
        s.execute(Command::SetAssist(AssistSettings {
            counting_aid: false,
            quiz: true,
            retrieval: false,
            assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
        }))
        .unwrap();
        s.execute(Command::SetCoach(CoachSettings {
            level: CoachLevel::EveryMove,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert_eq!(s.export_current_recording_privileged().unwrap(), recording);
        let after = s.view();
        assert_eq!(after.assist.quiz.question, view.assist.quiz.question);
        assert_eq!(after.hand, view.hand);
        assert_eq!(after.legal_actions, view.legal_actions);
        assert_eq!(after.current_trick, view.current_trick);
        assert_eq!(after.tricks_played, view.tricks_played);
        assert!(!after.coach.help_available, "coach still withheld");
        assert_eq!(after.assist.counting, CountingState::Off);
    }
    assert!(seen >= 10, "only {seen} sessions reached a question");
}

#[test]
fn answers_are_graded_against_the_exact_count() {
    let mut kinds = std::collections::HashSet::new();
    let mut outcomes = [0; 3];
    for seed in 1..60 {
        let mut s = session(seed);
        s.execute(Command::SetAssist(ON)).unwrap();
        let mut w = Witness::default();
        let Some(view) = until_question(&mut s, &mut w) else {
            continue;
        };
        let kind = view.assist.quiz.question.unwrap();
        kinds.insert(std::mem::discriminant(&kind));
        let exact = expected(&view, &w, kind);
        // Rotate through correct, wrong and skipped answers.
        let (command, outcome) = match seed % 3 {
            0 => (Command::AnswerQuiz(exact), 0),
            1 => (Command::AnswerQuiz(exact + 1), 1),
            _ => (Command::SkipQuiz, 2),
        };
        let events = s.execute(command).unwrap();
        outcomes[outcome] += 1;
        let [AppEvent::Quiz(feedback), AppEvent::Retrieval(_)] = events[..] else {
            panic!("one quiz event, got {events:?}");
        };
        assert_eq!(feedback.kind, kind);
        assert_eq!(
            feedback.derivation.expected(),
            exact,
            "seed {seed} {kind:?}"
        );
        match (outcome, feedback.outcome) {
            (0, QuizOutcome::Correct) | (2, QuizOutcome::Skipped) => {}
            (1, QuizOutcome::Incorrect { given }) => assert_eq!(given, exact + 1),
            other => panic!("unexpected {other:?}"),
        }
        let after = s.view();
        assert_eq!(after.assist.quiz.feedback, Some(feedback));
        assert_eq!(after.assist.quiz.question, None);
        // Answering is no game action and does not cost the turn.
        assert_eq!(after.hand, view.hand);
        assert_eq!(after.acting, Some(PlayerId::HUMAN));
        assert_eq!(after.legal_actions, view.legal_actions);
        assert_eq!(after.current_trick, view.current_trick);
        assert_eq!(after.tricks_played, view.tricks_played);
        // The aid is back, and a second answer is rejected.
        assert!(matches!(after.assist.counting, CountingState::Shown(_)));
        assert_eq!(s.execute(Command::AnswerQuiz(0)), Err(AppError::NoQuestion));
        assert_eq!(s.execute(Command::SkipQuiz), Err(AppError::NoQuestion));
        // Normal play continues, once, and the feedback goes with the
        // decision. No second question in the same trick.
        let action = view.legal_actions[0];
        s.execute(Command::Act(action)).unwrap();
        assert_eq!(s.view().assist.quiz.feedback, None);
        s.run_opponents();
        assert_eq!(s.view().assist.quiz.question.is_some(), {
            QUESTION_AFTER_TRICKS.contains(&s.view().tricks_played)
                && s.view().tricks_played != view.tricks_played
        });
    }
    assert!(outcomes.iter().all(|&n| n >= 5), "{outcomes:?}");
    assert!(kinds.len() >= 3, "all kinds should occur: {}", kinds.len());
}

#[test]
fn the_aid_shows_exact_counts_that_follow_play() {
    let mut s = session(3);
    s.execute(Command::SetAssist(AssistSettings {
        counting_aid: true,
        quiz: false,
        retrieval: false,
        assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
    }))
    .unwrap();
    let mut w = Witness::default();
    let mut last = 0;
    let mut checked = 0;
    for _ in 0..400 {
        let events = s.run_opponents();
        w.note(&events, s.view().dealer);
        let view = s.view();
        if let (Some(c), CountingState::Shown(aid)) = (view.contract, &view.assist.counting) {
            assert_eq!(aid.played.len(), w.played.len());
            assert!(aid.played.len() >= last || view.tricks_played == 0);
            last = aid.played.len();
            if c.game != GameType::Null {
                let t = aid.trumpf.expect("trumpf counted");
                let played = w.played.iter().filter(|&&c2| c.game.is_trump(c2)).count() as u8;
                assert_eq!(t.played, played);
                let own = view.hand.iter().filter(|&&c2| c.game.is_trump(c2)).count() as u8;
                assert_eq!(t.own, own);
                assert_eq!(t.outside, t.total - t.own - t.played - t.discarded);
                checked += 1;
            } else {
                assert!(aid.trumpf.is_none(), "no fictitious Trumpf in Null");
            }
            // The human's own void entry is always empty.
            assert!(aid.voids[PlayerId::HUMAN.index()].is_empty());
        }
        if view.series_finished {
            break;
        }
        let command = match view.legal_actions.first() {
            Some(&a) => Command::Act(a),
            None => Command::NextGame,
        };
        let events = s.execute(command).unwrap();
        w.note(&events, s.view().dealer);
    }
    assert!(checked > 10, "{checked}");
}

#[test]
fn switching_the_quiz_off_dismisses_the_question_and_play_goes_on() {
    for seed in 1..30 {
        let mut s = session(seed);
        s.execute(Command::SetAssist(ON)).unwrap();
        let mut w = Witness::default();
        let Some(view) = until_question(&mut s, &mut w) else {
            continue;
        };
        s.execute(Command::SetAssist(AssistSettings {
            counting_aid: true,
            quiz: false,
            retrieval: false,
            assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
        }))
        .unwrap();
        let after = s.view();
        assert_eq!(after.assist.quiz.question, None);
        assert_eq!(after.assist.quiz.feedback, None);
        assert_eq!(after.legal_actions, view.legal_actions);
        // Switching it on again in the same decision asks nothing new.
        s.execute(Command::SetAssist(ON)).unwrap();
        assert_eq!(s.view().assist.quiz.question, None);
        let action = view.legal_actions[0];
        s.execute(Command::Act(action)).unwrap();
        return;
    }
    panic!("no question reached");
}

#[test]
fn enabling_the_quiz_mid_decision_asks_when_due() {
    for seed in 1..30 {
        let mut s = session(seed);
        let mut w = Witness::default();
        // Play without the quiz up to the third trick.
        for _ in 0..2000 {
            let events = s.run_opponents();
            w.note(&events, s.view().dealer);
            let view = s.view();
            if view.phase == Phase::Spiel
                && view.tricks_played == 3
                && view.acting == Some(PlayerId::HUMAN)
            {
                break;
            }
            if view.series_finished {
                break;
            }
            let command = match view.legal_actions.first() {
                Some(&a) => Command::Act(a),
                None => Command::NextGame,
            };
            let events = s.execute(command).unwrap();
            w.note(&events, s.view().dealer);
        }
        let view = s.view();
        if view.tricks_played != 3 || view.acting != Some(PlayerId::HUMAN) {
            continue;
        }
        assert_eq!(view.assist.quiz.question, None);
        s.execute(Command::SetAssist(ON)).unwrap();
        let asked = s.view();
        let kind = asked.assist.quiz.question.expect("question is due now");
        assert_eq!(asked.hand, view.hand);
        let exact = expected(&asked, &w, kind);
        let events = s.execute(Command::AnswerQuiz(exact)).unwrap();
        let [AppEvent::Quiz(f), AppEvent::Retrieval(_)] = events[..] else {
            panic!()
        };
        assert_eq!(f.outcome, QuizOutcome::Correct);
        return;
    }
    panic!("no third trick with a human decision found");
}

#[test]
fn selection_is_reproducible_and_independent_of_aid_and_coach() {
    for seed in [2u64, 5, 9] {
        let ask = |settings: AssistSettings, coach: CoachSettings| {
            let mut s = session(seed);
            s.execute(Command::SetCoach(coach)).unwrap();
            s.execute(Command::SetAssist(settings)).unwrap();
            let mut w = Witness::default();
            until_question(&mut s, &mut w).map(|v| {
                (
                    v.game_number,
                    v.tricks_played,
                    v.hand.clone(),
                    v.assist.quiz.question,
                )
            })
        };
        let a = ask(ON, CoachSettings::default());
        assert!(a.is_some(), "seed {seed}");
        assert_eq!(a, ask(ON, CoachSettings::default()));
        // The aid and the coach level do not move the question.
        let b = ask(
            AssistSettings {
                counting_aid: false,
                quiz: true,
                retrieval: false,
                assistance: skat_app::AssistanceLevel::AnswerBeforeReveal,
            },
            CoachSettings {
                level: CoachLevel::Off,
                ..CoachSettings::default()
            },
        );
        assert_eq!(a, b);
    }
}

#[test]
fn derivation_is_a_snapshot_of_what_the_player_could_know() {
    let mut s = session(7);
    s.execute(Command::SetAssist(ON)).unwrap();
    let mut w = Witness::default();
    let view = until_question(&mut s, &mut w).expect("question");
    let kind = view.assist.quiz.question.unwrap();
    let events = s.execute(Command::SkipQuiz).unwrap();
    let [AppEvent::Quiz(f), AppEvent::Retrieval(_)] = events[..] else {
        panic!()
    };
    // The numbers add up as documented.
    match f.derivation {
        Derivation::ProvenVoids { .. } => unreachable!("baseline quiz"),
        Derivation::TrumpfOutside(t) => {
            assert_eq!(t.outside, t.total - t.own - t.played - t.discarded)
        }
        Derivation::ClassPlayed {
            size,
            played,
            unplayed,
            ..
        } => assert_eq!(size, played + unplayed),
        Derivation::AugenWon { augen, .. } => assert!(augen <= 120),
    }
    assert_eq!(f.kind, kind);
}

/// Enabling the quiz while a warning check is still computing withdraws the
/// held card: a late coach result must not play it behind the question.
#[test]
fn a_late_coach_result_never_plays_behind_a_pending_question() {
    for settle in 0..9 {
        let mut s = session(1);
        for _ in 0..2000 {
            s.run_opponents();
            let v = s.view();
            if v.phase == Phase::Spiel && v.tricks_played == 3 && v.acting == Some(PlayerId::HUMAN)
            {
                break;
            }
            assert!(!v.series_finished);
            let command = match v.legal_actions.first() {
                Some(&a) => Command::Act(a),
                None => Command::NextGame,
            };
            s.execute(command).unwrap();
        }
        s.set_coach_budget(skat_app::Budget {
            samples: 1,
            nodes_per_task: 1,
        });
        s.execute(Command::SetCoach(CoachSettings {
            warnings: true,
            ..CoachSettings::default()
        }))
        .unwrap();
        let action = s.view().legal_actions[0];
        let recording = s.export_current_recording_privileged().unwrap();
        s.execute(Command::Act(action)).unwrap();
        assert!(matches!(
            s.view().coach.pending,
            Some(skat_app::Pending::Checking { .. })
        ));
        let job = s.coach_job().expect("check is computing");
        s.execute(Command::SetAssist(if settle >= 3 {
            focused(skat_app::AssistanceLevel::RevealOnRequest)
        } else {
            ON
        }))
        .unwrap();
        let before = s.view();
        assert!(before.assist.quiz.question.is_some());
        assert!(before.coach.pending.is_none(), "held card is withdrawn");
        if settle >= 3 {
            s.execute(Command::RestoreQuizHelp).unwrap();
            assert!(s.view().assist.quiz.help.is_some());
        }
        let settle_command = match settle % 3 {
            0 => Command::AnswerQuiz(0),
            1 => Command::SkipQuiz,
            _ => Command::SetAssist(AssistSettings { quiz: false, ..ON }),
        };
        // Cover completion both during the hold and after every release path.
        if settle >= 6 {
            s.execute(settle_command).unwrap();
        }
        let events = s.finish_coach(job.run(None)).unwrap();
        assert!(events.is_empty());
        let after = s.view();
        assert_eq!(after.hand, before.hand);
        assert_eq!(after.acting, before.acting);
        if settle < 6 {
            assert_eq!(after.assist.quiz.question, before.assist.quiz.question);
            s.execute(settle_command).unwrap();
        } else {
            assert!(after.assist.quiz.question.is_none());
        }
        let done = s.view();
        assert!(done.assist.quiz.question.is_none());
        assert_eq!(done.hand, before.hand);
        assert_eq!(done.acting, Some(PlayerId::HUMAN));
        assert!(done.coach.pending.is_none());
        assert_eq!(s.export_current_recording_privileged().unwrap(), recording);
        s.execute(Command::Act(done.legal_actions[0])).unwrap();
        let after_recording = s.export_current_recording_privileged().unwrap();
        assert_eq!(after_recording.actions.len(), recording.actions.len() + 1);
        assert_eq!(after_recording.actions.last(), Some(&done.legal_actions[0]));
    }
}

fn focused(level: skat_app::AssistanceLevel) -> AssistSettings {
    AssistSettings {
        retrieval: true,
        assistance: level,
        ..ON
    }
}

fn retrieval_events(events: &[AppEvent]) -> Vec<skat_app::RetrievalEvent> {
    events
        .iter()
        .filter_map(|e| {
            if let AppEvent::Retrieval(r) = e {
                Some(*r)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn focused_support_is_explicit_sticky_and_deduplicated() {
    use skat_app::{AssistanceLevel::*, RetrievalAction};
    for level in [VisibleAid, RevealOnRequest, AnswerBeforeReveal] {
        for path in 0..4 {
            let mut s = session(1);
            s.execute(Command::SetAssist(focused(level))).unwrap();
            let mut w = Witness::default();
            let before = until_question(&mut s, &mut w).unwrap();
            let id = before.assist.quiz.id.unwrap();
            let kind = before.assist.quiz.question.unwrap();
            assert!(matches!(
                kind,
                QuestionKind::TrumpfOutside | QuestionKind::ProvenVoids { .. }
            ));
            assert_eq!(before.assist.quiz.help.is_some(), level == VisibleAid);
            assert!(before.coach.last_move.is_none() && before.coach.debriefing.is_none());
            assert!(!before.coach.help_available);
            if level != VisibleAid {
                assert_eq!(before.assist.counting, CountingState::Withheld);
                for _ in 0..5 {
                    assert_eq!(s.view(), before);
                }
            }
            // Help does not play a card, change preferences, or release the hold.
            let help = retrieval_events(&s.execute(Command::RestoreQuizHelp).unwrap());
            assert_eq!(help.len(), usize::from(level != VisibleAid));
            for e in help {
                assert_eq!(e.id, id.into());
                assert_eq!(e.action, RetrievalAction::Help);
                assert!(e.assisted);
            }
            assert!(s.execute(Command::RestoreQuizHelp).unwrap().is_empty());
            let revealed = s.view();
            let exact = revealed.assist.quiz.help.unwrap().expected();
            assert_eq!(revealed.assist.settings, before.assist.settings);
            assert_eq!(revealed.hand, before.hand);
            assert_eq!(
                s.execute(Command::Act(before.legal_actions[0])),
                Err(AppError::QuestionPending)
            );
            // Hiding restored help cannot make a later answer unaided.
            s.execute(Command::SetAssist(focused(AnswerBeforeReveal)))
                .unwrap();
            s.execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                show_figures: true,
                ..CoachSettings::default()
            }))
            .unwrap();
            assert!(s.view().coach.last_move.is_none());
            let command = match path {
                0 => Command::AnswerQuiz(exact),
                1 => Command::AnswerQuiz(exact + 1),
                2 => Command::SkipQuiz,
                _ => Command::SetAssist(AssistSettings {
                    quiz: false,
                    ..focused(level)
                }),
            };
            let events = s.execute(command).unwrap();
            let attempts = retrieval_events(&events);
            assert_eq!(attempts.len(), 1);
            let e = attempts[0];
            assert_eq!(e.id, id.into());
            assert!(e.assisted);
            let expected = match path {
                0 => RetrievalAction::Answer {
                    given: exact,
                    correct: true,
                },
                1 => RetrievalAction::Answer {
                    given: exact + 1,
                    correct: false,
                },
                2 => RetrievalAction::Skip { disabled: false },
                _ => RetrievalAction::Skip { disabled: true },
            };
            assert_eq!(e.action, expected);
            assert_eq!(
                serde_json::from_str::<AppEvent>(
                    &serde_json::to_string(&AppEvent::Retrieval(e)).unwrap()
                )
                .unwrap(),
                AppEvent::Retrieval(e)
            );
            assert_eq!(s.view().hand, before.hand);
            assert_eq!(
                s.execute(Command::AnswerQuiz(exact)),
                Err(AppError::NoQuestion)
            );
            assert_eq!(s.execute(Command::SkipQuiz), Err(AppError::NoQuestion));
            s.execute(Command::Act(before.legal_actions[0])).unwrap();
            assert!(s.view().assist.quiz.feedback.is_none());
        }
    }
}

#[test]
fn focused_questions_do_not_change_series_and_events_have_unique_keys() {
    use skat_app::{AssistanceLevel::*, RetrievalAction};
    let mut signatures = Vec::new();
    for level in [VisibleAid, RevealOnRequest, AnswerBeforeReveal] {
        let mut s = session(9);
        s.execute(Command::SetAssist(focused(level))).unwrap();
        let mut ids = std::collections::HashSet::new();
        let mut questions = Vec::new();
        let mut events = Vec::new();
        for _ in 0..5000 {
            events.extend(s.run_opponents());
            let v = s.view();
            if let Some(q) = v.assist.quiz.question {
                let id = v.assist.quiz.id.unwrap();
                questions.push((id, q));
                events.extend(s.execute(Command::SkipQuiz).unwrap());
                continue;
            }
            if v.series_finished {
                break;
            }
            let cmd = v
                .legal_actions
                .first()
                .map_or(Command::NextGame, |a| Command::Act(*a));
            events.extend(s.execute(cmd).unwrap());
        }
        assert!(s.view().series_finished);
        assert!(!questions.is_empty());
        for event in retrieval_events(&events) {
            let tag = match event.action {
                RetrievalAction::Opportunity => 0,
                RetrievalAction::Help => 1,
                RetrievalAction::Answer { .. } => 2,
                RetrievalAction::Skip { .. } => 3,
            };
            assert!(ids.insert((event.id, tag)), "duplicate event {event:?}");
            assert_eq!(event.assisted, level == VisibleAid);
            let json = serde_json::to_string(&event).unwrap();
            assert_eq!(
                serde_json::from_str::<skat_app::RetrievalEvent>(&json).unwrap(),
                event
            );
        }
        signatures.push((questions, s.sheet().clone()));
    }
    assert_eq!(signatures[0], signatures[1]);
    assert_eq!(signatures[1], signatures[2]);
    let mut plain = session(9);
    for _ in 0..5000 {
        plain.run_opponents();
        let v = plain.view();
        if v.series_finished {
            break;
        }
        plain
            .execute(
                v.legal_actions
                    .first()
                    .map_or(Command::NextGame, |a| Command::Act(*a)),
            )
            .unwrap();
    }
    assert_eq!(plain.sheet(), &signatures[0].1);
}

#[test]
fn focused_unaided_delayed_responses_use_the_frozen_public_count() {
    use skat_app::{AssistanceLevel, RetrievalAction};
    let mut checked = 0;
    for seed in 1..20 {
        let mut s = session(seed);
        let settings = focused(AssistanceLevel::AnswerBeforeReveal);
        s.execute(Command::SetAssist(settings)).unwrap();
        let mut w = Witness::default();
        let Some(v) = until_question(&mut s, &mut w) else {
            continue;
        };
        let kind = v.assist.quiz.question.unwrap();
        let exact = expected(&v, &w, kind);
        let id = v.assist.quiz.id;
        // Repeated settings and unrelated coach settings do not mutate the
        // snapshot, reveal an answer, or create another opportunity.
        for _ in 0..3 {
            assert!(s.execute(Command::SetAssist(settings)).unwrap().is_empty());
            s.execute(Command::SetCoach(CoachSettings {
                level: CoachLevel::EveryMove,
                show_figures: true,
                warnings: true,
                ..CoachSettings::default()
            }))
            .unwrap();
            assert_eq!(s.view().assist.quiz.id, id);
            assert!(s.view().assist.quiz.help.is_none());
            assert!(matches!(s.view().coach.advice, skat_app::Advice::None));
            assert!(s.view().coach.last_move.is_none() && s.view().coach.debriefing.is_none());
        }
        for (cmd, error) in [
            (Command::NextGame, AppError::GameInProgress),
            (Command::AdvanceOpponent, AppError::NoOpponentToMove),
        ] {
            let held = s.view();
            assert_eq!(s.execute(cmd), Err(error));
            assert_eq!(s.view(), held);
        }
        let given = exact + (seed % 2) as u8;
        let events = s.execute(Command::AnswerQuiz(given)).unwrap();
        let r = retrieval_events(&events);
        assert_eq!(r.len(), 1);
        assert_eq!(
            r[0].action,
            RetrievalAction::Answer {
                given,
                correct: given == exact
            }
        );
        assert!(!r[0].assisted);
        assert_eq!(
            s.view().assist.quiz.feedback.unwrap().derivation.expected(),
            exact
        );
        assert_eq!(s.view().hand, v.hand);
        checked += 1;
    }
    assert!(checked >= 10);
}

#[test]
fn settings_restore_help_once_and_hiding_it_keeps_the_support_record() {
    use skat_app::{AssistanceLevel::*, RetrievalAction};
    let mut s = session(1);
    s.execute(Command::SetAssist(focused(AnswerBeforeReveal)))
        .unwrap();
    let before = until_question(&mut s, &mut Witness::default()).unwrap();
    let events = s.execute(Command::SetAssist(focused(VisibleAid))).unwrap();
    let r = retrieval_events(&events);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].action, RetrievalAction::Help);
    assert!(r[0].assisted);
    assert_eq!(r[0].assistance, VisibleAid);
    assert_eq!(Some(r[0].id), before.assist.quiz.id.map(Into::into));
    assert!(
        s.execute(Command::SetAssist(focused(VisibleAid)))
            .unwrap()
            .is_empty()
    );
    let exact = s.view().assist.quiz.help.unwrap().expected();
    assert!(
        s.execute(Command::SetAssist(focused(RevealOnRequest)))
            .unwrap()
            .is_empty()
    );
    assert!(s.view().assist.quiz.help.is_none());
    assert_eq!(s.view().assist.counting, CountingState::Withheld);
    // Re-revealing the same opportunity never duplicates a Help event.
    assert!(s.execute(Command::RestoreQuizHelp).unwrap().is_empty());
    assert_eq!(s.view().assist.quiz.help.unwrap().expected(), exact);
    let events = s.execute(Command::AnswerQuiz(exact)).unwrap();
    assert!(retrieval_events(&events)[0].assisted);
}
