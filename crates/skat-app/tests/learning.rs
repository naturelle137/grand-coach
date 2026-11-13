// SPDX-License-Identifier: EUPL-1.2

//! The local learning history (M8 foundation, D-70): actual retrieval
//! events of real sessions are recorded per attempt, persisted, reopened
//! and summarised; duplicates, retries and practice stay distinct; damaged
//! or unsupported files are reported and never overwritten.

use skat_app::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn config(seed: u64) -> SessionConfig {
    SessionConfig {
        seed,
        series: SeriesLength::Fixed(4),
        kontra_re: false,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    }
}

fn focused(assistance: AssistanceLevel) -> AssistSettings {
    AssistSettings {
        counting_aid: false,
        quiz: true,
        retrieval: true,
        assistance,
    }
}

/// How the test answers a question.
#[derive(Clone, Copy)]
enum Respond {
    /// Restore help, then answer exactly from it.
    HelpThenExact,
    /// Answer zero without help.
    Zero,
    /// Skip.
    Skip,
    /// Answer exactly from the visible aid (VisibleAid only).
    VisibleExact,
}

fn retrieval(events: &[AppEvent]) -> Vec<RetrievalEvent> {
    events
        .iter()
        .filter_map(|e| match e {
            AppEvent::Retrieval(r) => Some(*r),
            _ => None,
        })
        .collect()
}

/// Plays the current game to its end with the first legal action,
/// answering every question with `respond`; returns all retrieval events.
fn play_game(s: &mut Session, respond: Respond) -> Vec<RetrievalEvent> {
    let mut out = Vec::new();
    loop {
        out.extend(retrieval(&s.run_opponents()));
        let view = s.view();
        if view.assist.quiz.question.is_some() {
            let events = match respond {
                Respond::HelpThenExact => {
                    out.extend(retrieval(&s.execute(Command::RestoreQuizHelp).unwrap()));
                    let exact = s.view().assist.quiz.help.unwrap().expected();
                    s.execute(Command::AnswerQuiz(exact)).unwrap()
                }
                Respond::Zero => s.execute(Command::AnswerQuiz(0)).unwrap(),
                Respond::Skip => s.execute(Command::SkipQuiz).unwrap(),
                Respond::VisibleExact => {
                    let exact = view.assist.quiz.help.unwrap().expected();
                    s.execute(Command::AnswerQuiz(exact)).unwrap()
                }
            };
            out.extend(retrieval(&events));
            continue;
        }
        let Some(&action) = view.legal_actions.first() else {
            return out;
        };
        out.extend(retrieval(&s.execute(Command::Act(action)).unwrap()));
    }
}

/// A live series with focused retrieval, played until one game produced
/// questions; returns the session, that game's events and its recording.
fn series_with_questions(
    respond: Respond,
    assistance: AssistanceLevel,
) -> (Session, Vec<RetrievalEvent>, GameRecording) {
    for seed in 1..40 {
        let mut s = Session::new(config(seed));
        s.execute(Command::SetAssist(focused(assistance))).unwrap();
        let events = play_game(&mut s, respond);
        if events
            .iter()
            .filter(|e| e.action == RetrievalAction::Opportunity)
            .count()
            >= 2
        {
            let recording = s.completed_recordings_privileged()[0].clone();
            return (s, events, recording);
        }
    }
    panic!("no seed with two questions in game 1");
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "skat-learning-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}

fn opportunities(events: &[RetrievalEvent]) -> u32 {
    events
        .iter()
        .filter(|e| e.action == RetrievalAction::Opportunity)
        .count() as u32
}

#[test]
fn actual_session_events_persist_across_restart_and_are_summarised() {
    let (_, events, _) = series_with_questions(Respond::Zero, AssistanceLevel::AnswerBeforeReveal);
    let n = opportunities(&events);
    let mut history = LearningHistory::new();
    let attempt = history.begin_attempt(AttemptKind::Series).unwrap();
    let recorded = history.record(attempt, &events).unwrap();
    assert_eq!(recorded.added, events.len());
    let dir = temp_dir();
    let path = dir.join("grand-coach").join("learning-history.json");
    history.save_local(&path).unwrap();
    // "Restart": a fresh history read from the file only.
    let reopened = LearningHistory::open_local(&path).unwrap();
    assert_eq!(reopened, history);
    let summary = reopened.summary();
    assert_eq!(summary.attempts, 1);
    assert_eq!(summary.totals.opportunities, n);
    assert_eq!(summary.totals.answers, n);
    assert_eq!(summary.totals.unaided_answers, n);
    assert_eq!(summary.totals.help + summary.totals.skips, 0);
    assert_eq!(summary.observations, Observations::Limited);
    assert!(
        summary
            .rows
            .iter()
            .all(|r| r.context == LearningContext::Series
                && matches!(
                    r.topic,
                    LearningTopic::TrumpfOutside | LearningTopic::ProvenVoids
                ))
    );
    // The file holds events and attempt identities, not cards or deals.
    let text = fs::read_to_string(&path).unwrap();
    for word in ["deal", "hand", "skat", "cards", "seed", "actions"] {
        assert!(!text.to_lowercase().contains(word), "{word} in {text}");
    }
    assert!(!dir.join("grand-coach/learning-history.json.tmp").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn replaying_or_reloading_the_same_events_never_counts_twice() {
    let (_, events, _) = series_with_questions(Respond::Skip, AssistanceLevel::RevealOnRequest);
    let mut history = LearningHistory::new();
    let attempt = history.begin_attempt(AttemptKind::Series).unwrap();
    history.record(attempt, &events).unwrap();
    let once = history.summary();
    // The same batch again, and a batch with internal repeats.
    let again = history.record(attempt, &events).unwrap();
    assert_eq!(again.added, 0);
    assert_eq!(again.duplicates, events.len());
    let doubled: Vec<_> = events.iter().chain(&events).copied().collect();
    assert_eq!(history.record(attempt, &doubled).unwrap().added, 0);
    // After a reload.
    let dir = temp_dir();
    let path = dir.join("learning-history.json");
    history.save_local(&path).unwrap();
    let mut reloaded = LearningHistory::open_local(&path).unwrap();
    assert_eq!(reloaded.record(attempt, &events).unwrap().added, 0);
    assert_eq!(reloaded.summary(), once);
    assert_eq!(once.totals.skips, opportunities(&events));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn series_practice_and_intentional_retry_stay_distinct() {
    let (live, events, recording) =
        series_with_questions(Respond::Zero, AssistanceLevel::AnswerBeforeReveal);
    let mut history = LearningHistory::new();
    let series = history.begin_attempt(AttemptKind::Series).unwrap();
    history.record(series, &events).unwrap();
    // Two practice attempts of the same deal, played identically: their
    // question IDs coincide (same practice stream), their attempts do not.
    let mut practice_events = Vec::new();
    for attempt in 1..=2 {
        let mut practice = live.start_practice(&recording).unwrap();
        let e = play_game(&mut practice, Respond::Zero);
        assert!(opportunities(&e) > 0);
        let id = history
            .begin_attempt(AttemptKind::Practice { attempt })
            .unwrap();
        assert_eq!(history.record(id, &e).unwrap().added, e.len());
        practice_events.push(e);
    }
    assert_eq!(
        practice_events[0].iter().map(|e| e.id).collect::<Vec<_>>(),
        practice_events[1].iter().map(|e| e.id).collect::<Vec<_>>()
    );
    // Practice IDs never collide with the live series' IDs.
    assert!(
        practice_events[0]
            .iter()
            .all(|p| events.iter().all(|e| e.id != p.id))
    );
    let summary = history.summary();
    assert_eq!(summary.attempts, 3);
    let total = |context| {
        summary
            .rows
            .iter()
            .filter(|r| r.context == context)
            .map(|r| r.counts.opportunities)
            .sum::<u32>()
    };
    assert_eq!(total(LearningContext::Series), opportunities(&events));
    assert_eq!(
        total(LearningContext::Practice),
        opportunities(&practice_events[0])
    );
    assert_eq!(
        total(LearningContext::PracticeRetry),
        opportunities(&practice_events[1])
    );
    let ids: Vec<_> = history.attempts().iter().map(|a| a.id).collect();
    assert_eq!(ids, [AttemptId(1), AttemptId(2), AttemptId(3)]);
}

#[test]
fn help_and_visible_aid_are_never_counted_as_unaided() {
    let mut history = LearningHistory::new();
    for (respond, level) in [
        (Respond::HelpThenExact, AssistanceLevel::RevealOnRequest),
        (Respond::VisibleExact, AssistanceLevel::VisibleAid),
        (Respond::Zero, AssistanceLevel::AnswerBeforeReveal),
    ] {
        let (_, events, _) = series_with_questions(respond, level);
        let attempt = history.begin_attempt(AttemptKind::Series).unwrap();
        history.record(attempt, &events).unwrap();
    }
    let t = history.summary().totals;
    assert_eq!(t.answers, t.opportunities);
    // Explicit help only for RevealOnRequest; VisibleAid starts assisted.
    assert!(t.help > 0 && t.help < t.answers);
    // Only the unassisted series' answers are unaided.
    assert!(t.unaided_answers > 0 && t.unaided_answers < t.answers);
    assert!(t.correct >= t.answers - t.unaided_answers);
    for e in history.events() {
        if let RetrievalAction::Answer { .. } = e.event.action {
            assert_eq!(
                e.event.assisted,
                e.event.assistance != AssistanceLevel::AnswerBeforeReveal
            );
        }
    }
}

#[test]
fn empty_history_states_no_observations_and_reset_is_persisted() {
    let empty = LearningHistory::new().summary();
    assert_eq!(empty.observations, Observations::None);
    assert!(empty.rows.is_empty() && empty.attempts == 0 && !empty.full);

    let (_, events, _) = series_with_questions(Respond::Zero, AssistanceLevel::VisibleAid);
    let mut history = LearningHistory::new();
    let first = history.begin_attempt(AttemptKind::Series).unwrap();
    history.record(first, &events).unwrap();
    let dir = temp_dir();
    let path = dir.join("learning-history.json");
    history.save_local(&path).unwrap();
    history.reset();
    history.save_local(&path).unwrap();
    let mut reopened = LearningHistory::open_local(&path).unwrap();
    assert!(reopened.is_empty());
    assert_eq!(reopened.summary().observations, Observations::None);
    // Identities issued before the reset are never issued again.
    assert_eq!(
        reopened.record(first, &events),
        Err(LearningError::UnknownAttempt)
    );
    assert_eq!(
        reopened.begin_attempt(AttemptKind::Series),
        Ok(AttemptId(2))
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn damaged_unsupported_and_oversized_files_are_reported_and_kept() {
    let (_, events, _) = series_with_questions(Respond::Zero, AssistanceLevel::VisibleAid);
    let mut history = LearningHistory::new();
    let a = history.begin_attempt(AttemptKind::Series).unwrap();
    history.record(a, &events).unwrap();
    let good: serde_json::Value = serde_json::to_value(&history).unwrap();
    let edit = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = good.clone();
        f(&mut v);
        serde_json::to_vec(&v).unwrap()
    };
    let cases: Vec<(Vec<u8>, LearningFileError)> = vec![
        (b"{ not json".to_vec(), LearningFileError::InvalidJson),
        (b"[]".to_vec(), LearningFileError::InvalidJson),
        (
            edit(&|v| {
                v.as_object_mut().unwrap().remove("format");
            }),
            LearningFileError::InvalidJson,
        ),
        (
            edit(&|v| v["format"] = 3.into()),
            LearningFileError::UnsupportedFormat,
        ),
        (
            edit(&|v| v["format"] = 0.into()),
            LearningFileError::UnsupportedFormat,
        ),
        (
            edit(&|v| v["events"] = "x".into()),
            LearningFileError::InvalidJson,
        ),
        (
            edit(&|v| v["next_attempt"] = 1.into()),
            LearningFileError::Invalid(LearningError::InvalidAttempt),
        ),
        (
            edit(&|v| v["attempts"][0]["id"] = 0.into()),
            LearningFileError::Invalid(LearningError::InvalidAttempt),
        ),
        (
            edit(&|v| {
                let a = v["attempts"][0].clone();
                v["attempts"].as_array_mut().unwrap().push(a);
            }),
            LearningFileError::Invalid(LearningError::InvalidAttempt),
        ),
        (
            edit(&|v| v["events"][0]["attempt"] = 7.into()),
            LearningFileError::Invalid(LearningError::UnknownAttempt),
        ),
        (
            edit(&|v| {
                let e = v["events"][0].clone();
                v["events"].as_array_mut().unwrap().push(e);
            }),
            LearningFileError::Invalid(LearningError::DuplicateEvent),
        ),
        (
            edit(&|v| {
                let e = v["events"][0].clone();
                v["events"] = vec![e; MAX_LEARNING_EVENTS + 1].into();
            }),
            LearningFileError::TooLarge,
        ),
        (
            vec![b' '; MAX_LEARNING_BYTES + 1],
            LearningFileError::TooLarge,
        ),
    ];
    let dir = temp_dir();
    for (n, (bytes, error)) in cases.into_iter().enumerate() {
        let path = dir.join(format!("case-{n}.json"));
        fs::write(&path, &bytes).unwrap();
        assert_eq!(LearningHistory::open_local(&path), Err(error), "case {n}");
        // Reading never changes the file.
        assert_eq!(fs::read(&path).unwrap(), bytes);
        // The recovery path keeps the content under a new name.
        let aside = LearningHistory::set_aside(&path).unwrap();
        assert!(!path.exists());
        assert_eq!(fs::read(&aside).unwrap(), bytes);
        assert_eq!(
            aside.file_name().unwrap().to_string_lossy(),
            format!("case-{n}.damaged-1.json")
        );
    }
    // A second damaged file never replaces the first set-aside one.
    let path = dir.join("learning-history.json");
    for content in ["first", "second"] {
        fs::write(&path, content).unwrap();
        LearningHistory::set_aside(&path).unwrap();
    }
    assert_eq!(
        fs::read_to_string(dir.join("learning-history.damaged-1.json")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(dir.join("learning-history.damaged-2.json")).unwrap(),
        "second"
    );
    assert_eq!(
        LearningHistory::open_local(&path),
        Err(LearningFileError::Missing)
    );
    assert_eq!(
        LearningHistory::open_local(&dir),
        Err(LearningFileError::Io)
    );
    // Saving refuses to replace a directory.
    let sub = dir.join("sub");
    fs::create_dir(&sub).unwrap();
    assert_eq!(history.save_local(&sub), Err(LearningFileError::Io));
    fs::remove_dir_all(dir).unwrap();
}
