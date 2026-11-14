// SPDX-License-Identifier: EUPL-1.2

//! Independent response expectations; the catalogue supplies legal evidence,
//! not the expected classification or aggregation used by these tests.
use skat_app::practice_catalogue::PracticeFamily as F;
use skat_app::{
    AssistanceLevel as A, FocusAnswer as R, FocusCommand as C, LearningContext, LearningHistory,
    MechanismExposure as E, MechanismResponse as M, TrainingSession,
};

fn start(h: &mut LearningHistory, family: F) -> TrainingSession {
    TrainingSession::catalogue(family, A::AnswerBeforeReveal, h, true).unwrap()
}
fn run(h: &mut LearningHistory, s: &mut TrainingSession, c: C) {
    s.execute_focus(c, h, true).unwrap();
}
fn reopen(h: &LearningHistory) -> LearningHistory {
    LearningHistory::read_local(serde_json::to_vec(h).unwrap().as_slice()).unwrap()
}

#[test]
fn independent_complete_answer_matrix_distinguishes_uncertainty_and_absence() {
    for (expected, outcomes) in [
        (
            R::Possible,
            [M::Correct, M::OverstatedCertainty, M::DeniedPossibility],
        ),
        (
            R::Certain,
            [M::UnderstatedCertainty, M::Correct, M::DeniedPossibility],
        ),
        (R::Absent, [M::FalsePresence, M::FalsePresence, M::Correct]),
    ] {
        for (given, outcome) in [R::Possible, R::Certain, R::Absent]
            .into_iter()
            .zip(outcomes)
        {
            assert_eq!(M::classify(given, Some(expected)), outcome);
        }
    }
    for given in [R::Possible, R::Certain, R::Absent, R::Count(0)] {
        assert_eq!(M::classify(given, None), M::Unsupported);
        assert_eq!(M::classify(given, Some(R::Count(0))), M::Unsupported);
    }
    assert_eq!(M::classify(R::Count(0), Some(R::Absent)), M::Unsupported);
}

#[test]
fn legal_catalogue_positive_negative_and_uncertain_answers_have_fixed_categories() {
    for family in [F::SideCardVulnerability, F::PartnerOvertaking] {
        let mut h = LearningHistory::new();
        let mut s = start(&mut h, family);
        run(&mut h, &mut s, C::Answer(R::Certain)); // intro: possible, never certain
        run(&mut h, &mut s, C::Next);
        run(&mut h, &mut s, C::Answer(R::Absent)); // changed: possible, not absent
        run(&mut h, &mut s, C::Next);
        run(&mut h, &mut s, C::Answer(R::Possible)); // boundary: absent, not possible
        let rows = h.mechanism_responses();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].counts.overstated, 1);
        assert_eq!(rows[1].counts.denied, 1);
        assert_eq!(rows[2].counts.false_presence, 1);
        assert!(rows[1].eligible);
        for row in rows {
            assert_eq!(
                (row.counts.presented, row.counts.answered, row.counts.scored),
                (1, 1, 1)
            );
            assert_eq!(row.counts.correct, 0);
            assert_eq!(row.exposure, E::First);
        }
        assert_eq!(h.summary().responses, Default::default());
    }
}

#[test]
fn mixed_help_skip_unanswered_and_repeated_opportunities_have_separate_denominators() {
    let mut h = LearningHistory::new();
    let mut s = start(&mut h, F::SideCardVulnerability);
    run(&mut h, &mut s, C::Help);
    run(&mut h, &mut s, C::Assistance(A::AnswerBeforeReveal));
    run(&mut h, &mut s, C::Answer(R::Possible));
    let duplicate = h.focus_entries().last().unwrap().clone();
    h.record_focus(&duplicate).unwrap();
    run(&mut h, &mut s, C::Retry);
    run(&mut h, &mut s, C::Skip);
    run(&mut h, &mut s, C::Next); // unanswered changed opportunity
    let mut count = start(&mut h, F::RemainingTrumpf);
    run(&mut h, &mut count, C::Answer(R::Count(5)));
    let rows = h.mechanism_responses();
    assert_eq!(rows.len(), 3);
    let first = &rows[0];
    assert!(first.assisted);
    assert_eq!(first.counts.correct, 1);
    let repeat = &rows[1];
    assert_eq!(repeat.context, LearningContext::TrainingRepeat);
    assert_eq!(repeat.exposure, E::Repeated);
    assert_eq!(
        (
            repeat.counts.presented,
            repeat.counts.answered,
            repeat.counts.scored,
            repeat.counts.skipped,
            repeat.counts.unknown
        ),
        (1, 0, 0, 1, 0)
    );
    let unanswered = &rows[2];
    assert!(!unanswered.assisted);
    assert_eq!(
        (
            unanswered.counts.presented,
            unanswered.counts.answered,
            unanswered.counts.scored,
            unanswered.counts.skipped,
            unanswered.counts.unknown
        ),
        (1, 0, 0, 0, 1)
    );
    assert_eq!(unanswered.counts.unanswered, 1);
    assert_eq!(reopen(&h).mechanism_responses(), rows);
    assert_eq!(h.summary().responses, Default::default());
}

#[test]
fn empty_sparse_windows_use_only_same_task_context_and_latest_twenty_settlements() {
    let mut h = LearningHistory::new();
    assert!(h.mechanism_responses().is_empty());
    let mut s = start(&mut h, F::PartnerOvertaking);
    run(&mut h, &mut s, C::Answer(R::Possible)); // first exposure never pooled
    for i in 0..21 {
        run(&mut h, &mut s, C::Retry);
        run(
            &mut h,
            &mut s,
            C::Answer(if i < 10 { R::Certain } else { R::Possible }),
        );
        // Interleaving another topic cannot fill or shift these windows.
        if i == 9 {
            let mut other = start(&mut h, F::SideCardVulnerability);
            run(&mut h, &mut other, C::Skip);
        }
        let rows = h.mechanism_responses();
        let repeat = rows.iter().find(|r| r.exposure == E::Repeated).unwrap();
        if i < 19 {
            assert!(repeat.comparison.is_none());
        } else {
            let w = repeat.comparison.unwrap();
            assert_eq!((w.earlier.presented, w.recent.presented), (10, 10));
            assert_eq!(w.recent.correct, 10);
            assert_eq!(w.earlier.overstated, if i == 19 { 10 } else { 9 });
            assert_eq!(w.earlier.correct, if i == 19 { 0 } else { 1 });
        }
    }
    // An unanswered repeat does not enter a settlement window.
    run(&mut h, &mut s, C::Retry);
    let rows = h.mechanism_responses();
    let repeat = rows.iter().find(|r| r.exposure == E::Repeated).unwrap();
    assert_eq!(repeat.counts.unknown, 1);
    assert_eq!(repeat.comparison.unwrap().recent.correct, 10);
    assert_eq!(rows, reopen(&h).mechanism_responses());
    h.reset();
    assert!(reopen(&h).mechanism_responses().is_empty());
}

#[test]
fn incomplete_history_reports_unknown_first_and_observed_repeat_without_novelty() {
    let mut h = LearningHistory::new();
    h.invalidate_exposure();
    let mut s = start(&mut h, F::SideCardVulnerability);
    run(&mut h, &mut s, C::Skip);
    run(&mut h, &mut s, C::Retry);
    let rows = h.mechanism_responses();
    assert_eq!(rows[0].exposure, E::Unknown);
    assert_eq!(rows[1].exposure, E::Repeated);
    assert!(rows.iter().all(|r| !r.eligible));
    assert_eq!(rows, reopen(&h).mechanism_responses());
}

#[test]
fn assisted_skip_windows_never_pool_with_unaided_or_other_versions() {
    let mut h = LearningHistory::new();
    let mut s = start(&mut h, F::SideCardVulnerability);
    run(&mut h, &mut s, C::Answer(R::Possible));
    for i in 0..20 {
        run(&mut h, &mut s, C::Retry);
        run(&mut h, &mut s, C::Help);
        run(&mut h, &mut s, C::Assistance(A::AnswerBeforeReveal));
        run(
            &mut h,
            &mut s,
            if i < 10 {
                C::Skip
            } else {
                C::Answer(R::Possible)
            },
        );
    }
    let rows = h.mechanism_responses();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].comparison.is_none());
    assert!(!rows[0].assisted);
    assert!(rows[1].assisted);
    let w = rows[1].comparison.unwrap();
    assert_eq!(
        (
            w.earlier.presented,
            w.earlier.skipped,
            w.earlier.answered,
            w.earlier.scored
        ),
        (10, 10, 0, 0)
    );
    assert_eq!(
        (
            w.recent.presented,
            w.recent.skipped,
            w.recent.answered,
            w.recent.scored
        ),
        (10, 0, 10, 10)
    );
    let mut invalid = h.focus_entries().last().unwrap().clone();
    invalid.task.version += 1;
    assert!(h.record_focus(&invalid).is_err());
    assert_eq!(rows, reopen(&h).mechanism_responses());
}
