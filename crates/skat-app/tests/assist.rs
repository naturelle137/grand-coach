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
    })
}

const ON: AssistSettings = AssistSettings {
    counting_aid: true,
    quiz: true,
};

/// What the test saw happen in the current game.
#[derive(Default)]
struct Witness {
    played: Vec<Card>,
    /// Augen won by each player in completed tricks.
    won: [u32; 3],
}

impl Witness {
    fn note(&mut self, events: &[AppEvent], dealer: PlayerId) {
        for event in events {
            match event {
                AppEvent::GameStarted { .. } => *self = Witness::default(),
                AppEvent::Game {
                    event: Event::CardPlayed { card, .. },
                    ..
                } => self.played.push(*card),
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
        for command in [
            Command::Act(action),
            Command::RequestAdvice,
            Command::SkipAdvice,
            Command::ProceedAnyway,
            Command::Reconsider,
        ] {
            assert_eq!(s.execute(command), Err(AppError::QuestionPending));
            assert_eq!(s.view(), view, "{command:?} changed something");
        }
        // Changing assistance or coach settings keeps the game and the
        // question as they are.
        s.execute(Command::SetAssist(AssistSettings {
            counting_aid: false,
            quiz: true,
        }))
        .unwrap();
        s.execute(Command::SetCoach(CoachSettings {
            level: CoachLevel::EveryMove,
            ..CoachSettings::default()
        }))
        .unwrap();
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
        let [AppEvent::Quiz(feedback)] = events[..] else {
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
        let [AppEvent::Quiz(f)] = events[..] else {
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
    let [AppEvent::Quiz(f)] = events[..] else {
        panic!()
    };
    // The numbers add up as documented.
    match f.derivation {
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
