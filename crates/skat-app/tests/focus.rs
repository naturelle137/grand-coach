// SPDX-License-Identifier: EUPL-1.2

//! Session/exposure/scoring tests. The unchanged catalogue has its own oracle
//! and hidden-substitution witnesses; these expectations test new semantics.
use skat_app::practice_catalogue::{PracticeFamily as F, PracticeFeedback, PracticeQuestion};
use skat_app::{
    AssistanceLevel as A, FocusAction, FocusAnswer as R, FocusCommand as C, FocusTask,
    LearningError, LearningFileError, LearningHistory, TrainingSession, focus_expected,
};

fn start(family: F, h: &mut LearningHistory) -> TrainingSession {
    TrainingSession::catalogue(family, A::AnswerBeforeReveal, h, true).unwrap()
}
fn run(s: &mut TrainingSession, h: &mut LearningHistory, c: C) {
    s.execute_focus(c, h, true).unwrap();
}
fn roundtrip(h: &LearningHistory) -> LearningHistory {
    LearningHistory::read_local(serde_json::to_vec(h).unwrap().as_slice()).unwrap()
}

#[test]
fn all_four_families_score_independently_specified_answers_after_attempt_only() {
    for (family, answers) in [
        (F::RemainingTrumpf, [R::Count(5), R::Count(1), R::Count(0)]),
        (F::ProvenVoid, [R::Count(1), R::Count(1), R::Count(0)]),
        (
            F::SideCardVulnerability,
            [R::Possible, R::Possible, R::Absent],
        ),
        (F::PartnerOvertaking, [R::Possible, R::Possible, R::Absent]),
    ] {
        let mut h = LearningHistory::new();
        let mut s = start(family, &mut h);
        for (i, answer) in answers.into_iter().enumerate() {
            let before = s.focus_view().unwrap();
            assert!(before.feedback.is_none());
            assert!(!before.settled);
            assert_eq!(before.number, i + 1);
            assert_eq!(before.entry.eligible, i == 1);
            assert_eq!(before.entry.action, FocusAction::Presented);
            assert!(before.position.own_skat.is_none());
            assert!(before.position.declarer_cards.is_none());
            assert!(
                before
                    .position
                    .legal_actions
                    .iter()
                    .all(|a| matches!(a, skat_app::Action::Play(_)))
            );
            assert!(s.execute_focus(C::Next, &mut h, true).is_err());
            run(&mut s, &mut h, C::Answer(answer));
            let after = s.focus_view().unwrap();
            assert_eq!(after.entry.correct(), Some(true));
            assert_eq!(after.entry.unseen_correct(), i == 1);
            assert!(after.feedback.is_some());
            assert_eq!(
                h.summary().responses,
                Default::default(),
                "mechanism responses cannot enter counting deviations"
            );
            if i != 2 {
                run(&mut s, &mut h, C::Next);
            }
        }
        let restored = roundtrip(&h);
        assert_eq!(restored.focus_entries(), h.focus_entries());
        assert_eq!(restored.attempts().len(), 3);
    }
}

#[test]
fn exposure_before_answer_survives_exit_restart_skip_and_retry() {
    let mut h = LearningHistory::new();
    let mut s = start(F::RemainingTrumpf, &mut h);
    run(&mut s, &mut h, C::Skip);
    run(&mut s, &mut h, C::Next);
    assert!(s.focus_view().unwrap().entry.eligible);
    // Exit without an answer. The presentation alone is durable exposure.
    drop(s);
    let mut h = roundtrip(&h);
    let mut s = start(F::RemainingTrumpf, &mut h);
    run(&mut s, &mut h, C::Answer(R::Count(5)));
    run(&mut s, &mut h, C::Next);
    assert!(!s.focus_view().unwrap().entry.eligible);
    run(&mut s, &mut h, C::Answer(R::Count(1)));
    run(&mut s, &mut h, C::Retry);
    assert!(!s.focus_view().unwrap().entry.eligible);
    assert!(s.focus_view().unwrap().feedback.is_none());
    let id = s.focus_view().unwrap().entry.attempt;
    assert!(matches!(
        h.attempts().iter().find(|a| a.id == id).unwrap().kind,
        skat_app::AttemptKind::Training { attempt: 3 }
    ));
    let e = h.focus_entries().last().unwrap().clone();
    let len = h.focus_entries().len();
    h.record_focus(&e).unwrap();
    assert_eq!(h.focus_entries().len(), len);
    let mut changed = e.clone();
    changed.task.version += 1;
    assert_eq!(
        h.record_focus(&changed),
        Err(LearningError::InvalidProvenance)
    );
    changed.task = FocusTask {
        id: "unknown".into(),
        version: 1,
    };
    assert!(changed.task.resolve().is_none());
    assert_eq!(
        h.record_focus(&changed),
        Err(LearningError::InvalidProvenance)
    );
}

#[test]
fn help_visible_support_and_skip_are_sticky_and_never_unseen_credit() {
    for support in [A::AnswerBeforeReveal, A::RevealOnRequest, A::VisibleAid] {
        let mut h = LearningHistory::new();
        let mut s =
            TrainingSession::catalogue(F::SideCardVulnerability, support, &mut h, true).unwrap();
        run(&mut s, &mut h, C::Skip);
        run(&mut s, &mut h, C::Next);
        run(&mut s, &mut h, C::Help);
        run(&mut s, &mut h, C::Assistance(A::AnswerBeforeReveal));
        let v = s.focus_view().unwrap();
        assert!(v.entry.assisted);
        assert!(v.feedback.is_some());
        run(&mut s, &mut h, C::Answer(R::Possible));
        let v = s.focus_view().unwrap();
        assert_eq!(v.entry.correct(), Some(true));
        assert!(!v.entry.unseen_correct());
        assert!(roundtrip(&h).focus_entries().last().unwrap().assisted);
    }
}

#[test]
fn certainty_is_not_possibility_and_mechanisms_are_not_numeric_answers() {
    let mut h = LearningHistory::new();
    let mut s = start(F::PartnerOvertaking, &mut h);
    let before = h.clone();
    assert!(
        s.execute_focus(C::Answer(R::Count(1)), &mut h, true)
            .is_err()
    );
    assert_eq!(before, h);
    run(&mut s, &mut h, C::Answer(R::Certain));
    let v = s.focus_view().unwrap();
    assert_eq!(v.entry.correct(), Some(false));
    assert!(matches!(
        v.entry.action,
        FocusAction::Answer {
            given: R::Certain,
            expected: Some(R::Possible),
            ..
        }
    ));
    assert_eq!(focus_expected(&PracticeFeedback::Count(None)), None);
    run(&mut s, &mut h, C::Next);
    run(&mut s, &mut h, C::Skip);
    run(&mut s, &mut h, C::Next);
    run(&mut s, &mut h, C::Answer(R::Absent));
    assert_eq!(s.focus_view().unwrap().entry.correct(), Some(true));
    assert!(matches!(
        s.focus_view().unwrap().entry.question,
        PracticeQuestion::Consequence { .. }
    ));
}

#[test]
fn incomplete_history_reset_and_unavailable_storage_disable_novelty() {
    for unreliable in [false, true] {
        let mut h = LearningHistory::new();
        if !unreliable {
            h.invalidate_exposure();
        }
        let mut s = TrainingSession::catalogue(
            F::RemainingTrumpf,
            A::AnswerBeforeReveal,
            &mut h,
            !unreliable,
        )
        .unwrap();
        s.execute_focus(C::Skip, &mut h, !unreliable).unwrap();
        s.execute_focus(C::Next, &mut h, !unreliable).unwrap();
        assert!(!s.focus_view().unwrap().entry.eligible);
    }
    let mut h = LearningHistory::new();
    let mut s = start(F::RemainingTrumpf, &mut h);
    run(&mut s, &mut h, C::Skip);
    run(&mut s, &mut h, C::Next);
    let last = s.focus_view().unwrap().entry.attempt;
    h.reset();
    let mut h = roundtrip(&h);
    let mut s = start(F::RemainingTrumpf, &mut h);
    run(&mut s, &mut h, C::Skip);
    run(&mut s, &mut h, C::Next);
    assert!(!s.focus_view().unwrap().entry.eligible);
    assert!(s.focus_view().unwrap().entry.attempt > last);
}

#[test]
fn formats_one_two_three_migrate_losslessly_without_inventing_exposure() {
    let legacy = r#"{"format":3,"next_attempt":2,"attempts":[{"id":1,"kind":"Series"}],"events":[{"attempt":1,"event":{"id":{"series":3,"game":1,"trick":3},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}}]}"#;
    for format in 1..=3 {
        let mut original: serde_json::Value = serde_json::from_str(legacy).unwrap();
        original["format"] = format.into();
        let mut h =
            LearningHistory::read_local(serde_json::to_vec(&original).unwrap().as_slice()).unwrap();
        let saved = serde_json::to_value(&h).unwrap();
        for key in ["next_attempt", "attempts", "events"] {
            assert_eq!(saved[key], original[key]);
        }
        let mut s = start(F::RemainingTrumpf, &mut h);
        run(&mut s, &mut h, C::Skip);
        run(&mut s, &mut h, C::Next);
        assert!(!s.focus_view().unwrap().entry.eligible);
    }
}

#[test]
fn unsupported_corrupt_provenance_and_hidden_payload_are_rejected() {
    assert_eq!(
        LearningHistory::read_local(br#"{"format":999}"#.as_slice()),
        Err(LearningFileError::UnsupportedFormat)
    );
    assert_eq!(
        LearningHistory::read_local(b"broken".as_slice()),
        Err(LearningFileError::InvalidJson)
    );
    let mut h = LearningHistory::new();
    let mut s = start(F::RemainingTrumpf, &mut h);
    run(&mut s, &mut h, C::Answer(R::Count(5)));
    let original = serde_json::to_value(&h).unwrap();
    for (field, bad) in [
        ("id", serde_json::json!("missing")),
        ("version", serde_json::json!(2)),
    ] {
        let mut data = original.clone();
        data["focus"][0]["task"][field] = bad;
        assert!(
            LearningHistory::read_local(serde_json::to_vec(&data).unwrap().as_slice()).is_err()
        );
    }
    let mut data = original.clone();
    data["focus"][1]["action"]["Answer"]["expected"] = serde_json::json!({"Count":4});
    assert!(LearningHistory::read_local(serde_json::to_vec(&data).unwrap().as_slice()).is_err());
    let mut data = original.clone();
    data["focus"][1]["assisted"] = true.into();
    data["focus"][0]["assisted"] = true.into();
    data["focus"][1]["assisted"] = false.into();
    assert!(LearningHistory::read_local(serde_json::to_vec(&data).unwrap().as_slice()).is_err());
    let task = serde_json::to_string(&s.focus_view().unwrap().position).unwrap();
    for forbidden in [
        "original_hands",
        "original_skat",
        "seed",
        "expected",
        "feedback",
    ] {
        assert!(!task.contains(forbidden));
    }
}
