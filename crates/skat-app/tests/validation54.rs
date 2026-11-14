// SPDX-License-Identifier: EUPL-1.2

//! Technical evidence only: reserved views are never rendered or printed.
use skat_app::practice_catalogue::ValidationSet as S;
use skat_app::{
    AssistanceLevel as A, FocusAction, FocusAnswer as R, FocusCommand as C, LearningHistory,
    PriorExposure as E, TrainingSession, ValidationEvidence, ValidationResponse as V,
};

fn start(h: &mut LearningHistory, set: S, at: u64) -> TrainingSession {
    TrainingSession::validation(
        ValidationEvidence::new(set, "test-build-54".into(), at),
        h,
        true,
    )
    .unwrap()
}
fn submit(s: &mut TrainingSession, h: &mut LearningHistory, response: V, exposure: E, at: u64) {
    s.execute_focus(
        C::SubmitValidation {
            response,
            exposure,
            own_words: "Learner's unedited words / unklar".into(),
            clarity: "Unclear term: X".into(),
            submitted_at: at,
        },
        h,
        true,
    )
    .unwrap();
}
fn reload(h: &LearningHistory) -> LearningHistory {
    LearningHistory::read_local(&serde_json::to_vec(h).unwrap()[..]).unwrap()
}

#[test]
fn reserved_sets_use_same_history_hidden_feedback_exact_scores_and_actual_interval() {
    let mut h = LearningHistory::new();
    for (set, time, answers) in [
        (
            S::Initial,
            1_800_000_000,
            [R::Count(2), R::Count(1), R::Possible, R::Possible],
        ),
        // Intentionally outside target: preserve actual 3 days, do not assert compliance.
        (
            S::Delayed,
            1_800_259_200,
            [R::Count(7), R::Count(1), R::Absent, R::Absent],
        ),
    ] {
        let mut session = start(&mut h, set, time);
        for (i, answer) in answers.into_iter().enumerate() {
            let view = session.focus_view().unwrap();
            assert!(view.feedback.is_none());
            assert!(view.entry.eligible);
            assert_eq!(
                view.entry.validation.as_ref().unwrap().prior_exposure,
                E::Unknown
            );
            assert!(!view.entry.unseen_correct());
            assert!(
                session
                    .execute_focus(C::Answer(answer), &mut h, true)
                    .is_err()
            );
            submit(&mut session, &mut h, V::Answer(answer), E::None, time + 1);
            let view = session.focus_view().unwrap();
            assert_eq!(view.entry.correct(), Some(true));
            assert!(view.entry.unseen_correct());
            assert!(view.feedback.is_some());
            assert_eq!(
                view.entry.validation.unwrap().own_words,
                "Learner's unedited words / unklar"
            );
            if i < 3 {
                session.execute_focus(C::Next, &mut h, true).unwrap();
            }
        }
        assert!(session.execute_focus(C::Next, &mut h, true).is_err());
    }
    let restored = reload(&h);
    assert_eq!(restored.focus_entries(), h.focus_entries());
    let summary = restored.validation_summary();
    assert_eq!(summary.elapsed_seconds, Some(259200));
    assert_eq!(summary.counts.opportunities, 8);
    assert_eq!(summary.counts.scored, 8);
    assert_eq!(summary.counts.eligible_correct, 8);
    assert_eq!(summary.counts.repeats, 0);
}

#[test]
fn unknown_exposure_sticky_help_retries_unclear_skip_stop_and_no_answer_are_separate() {
    let mut h = LearningHistory::new();
    let mut s = start(&mut h, S::Initial, 100);
    s.execute_focus(C::Help, &mut h, true).unwrap();
    s.execute_focus(C::Assistance(A::AnswerBeforeReveal), &mut h, true)
        .unwrap();
    submit(&mut s, &mut h, V::Answer(R::Count(2)), E::None, 101);
    assert!(!s.focus_view().unwrap().entry.unseen_correct());
    s.execute_focus(C::Retry, &mut h, true).unwrap();
    assert!(s.focus_view().unwrap().entry.assisted);
    assert!(!s.focus_view().unwrap().entry.eligible);
    submit(&mut s, &mut h, V::Answer(R::Count(1)), E::Unknown, 102);
    s.execute_focus(C::Next, &mut h, true).unwrap();
    submit(&mut s, &mut h, V::Unclear, E::Seen, 103);
    assert!(matches!(
        s.focus_view().unwrap().entry.action,
        FocusAction::Unclear
    ));
    assert!(s.focus_view().unwrap().feedback.is_none());
    s.execute_focus(C::Next, &mut h, true).unwrap();
    submit(&mut s, &mut h, V::Skip, E::Unknown, 104);
    s.execute_focus(C::Next, &mut h, true).unwrap();
    submit(&mut s, &mut h, V::Stop, E::Unknown, 105);
    let _abandoned = start(&mut h, S::Delayed, 200);
    let summary = reload(&h).validation_summary().counts;
    assert_eq!(summary.opportunities, 6);
    assert_eq!(summary.answers, 2);
    assert_eq!(summary.scored, 2);
    assert_eq!(summary.correct, 1);
    assert_eq!(summary.wrong, 1);
    assert_eq!(summary.eligible_answers, 0);
    assert_eq!(summary.help, 1);
    assert_eq!(summary.repeats, 1);
    assert_eq!(summary.unclear, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.unanswered, 2);
    assert_eq!(summary.exposure_unknown, 4);
}

#[test]
fn reset_missing_and_format4_migration_preserve_records_but_never_grant_novelty() {
    let mut h = LearningHistory::new();
    let mut old = TrainingSession::catalogue(
        skat_app::practice_catalogue::PracticeFamily::RemainingTrumpf,
        A::AnswerBeforeReveal,
        &mut h,
        true,
    )
    .unwrap();
    old.execute_focus(C::Skip, &mut h, true).unwrap();
    let mut json = serde_json::to_value(&h).unwrap();
    json["format"] = 4.into();
    let mut migrated =
        LearningHistory::read_local(&serde_json::to_vec(&json).unwrap()[..]).unwrap();
    assert_eq!(migrated.focus_entries(), h.focus_entries());
    let mut s = start(&mut migrated, S::Initial, 100);
    assert!(!s.focus_view().unwrap().entry.eligible);
    submit(&mut s, &mut migrated, V::Answer(R::Count(2)), E::None, 101);
    assert!(!s.focus_view().unwrap().entry.unseen_correct());
    assert!(
        TrainingSession::validation(
            ValidationEvidence::new(S::Initial, "build".into(), 100),
            &mut h,
            false
        )
        .is_err()
    );
    migrated.reset();
    let s = start(&mut migrated, S::Delayed, 200);
    assert!(!s.focus_view().unwrap().entry.eligible);
}

#[test]
fn tampered_versions_metadata_scores_and_duplicate_settlements_are_rejected() {
    let mut h = LearningHistory::new();
    let mut s = start(&mut h, S::Initial, 100);
    submit(&mut s, &mut h, V::Answer(R::Count(2)), E::None, 101);
    assert!(
        s.execute_focus(
            C::SubmitValidation {
                response: V::Answer(R::Count(2)),
                exposure: E::None,
                own_words: String::new(),
                clarity: String::new(),
                submitted_at: 102
            },
            &mut h,
            true
        )
        .is_err()
    );
    let original = serde_json::to_value(&h).unwrap();
    for field in [
        "version", "set", "build", "time", "score", "missing", "format",
    ] {
        let mut value = original.clone();
        match field {
            "version" => value["focus"][1]["task"]["version"] = 2.into(),
            "set" => value["focus"][1]["validation"]["set"] = "Delayed".into(),
            "build" => value["focus"][1]["validation"]["build"] = "other".into(),
            "time" => value["focus"][1]["validation"]["submitted_at"] = 99.into(),
            "score" => {
                value["focus"][1]["action"]["Answer"]["expected"] = serde_json::json!({"Count":9})
            }
            "missing" => {
                value["focus"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("validation");
            }
            "format" => value["format"] = 4.into(),
            _ => unreachable!(),
        }
        assert!(
            LearningHistory::read_local(&serde_json::to_vec(&value).unwrap()[..]).is_err(),
            "{field}"
        );
    }
}

#[test]
fn unexpected_external_aid_stops_and_cannot_become_unaided_by_retry_or_next() {
    let mut h = LearningHistory::new();
    let mut s = start(&mut h, S::Initial, 100);
    submit(&mut s, &mut h, V::UnexpectedAid, E::None, 101);
    let entry = s.focus_view().unwrap().entry;
    assert!(entry.assisted);
    assert!(entry.validation.unwrap().external_aid);
    assert!(matches!(entry.action, FocusAction::Stopped));
    assert!(s.focus_view().unwrap().feedback.is_none());
    assert!(s.execute_focus(C::Retry, &mut h, true).is_err());
    assert!(s.execute_focus(C::Next, &mut h, true).is_err());
    assert_eq!(reload(&h).validation_summary().counts.assisted, 1);
    assert_eq!(reload(&h).validation_summary().counts.unanswered, 1);
}
