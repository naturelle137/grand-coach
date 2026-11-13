// SPDX-License-Identifier: EUPL-1.2

//! M7 bounded review of a completed recording (D-69): zero to three moments
//! from the shared D-58 selection over the recorded human's historical
//! decisions. Expected decisions, seats and the forced last card follow the
//! documented deal of `fixtures/recordings/README.md`; the expected moment
//! selection is cross-checked against independent single historical
//! comments (D-67) and `skat_coach::moments`, not against the review code.

use skat_app::coach_model::{self, Status};
use skat_app::*;

fn fixture(name: &str) -> GameRecording {
    let json = match name {
        "herz" => include_str!("fixtures/recordings/herz.json"),
        "grand" => include_str!("fixtures/recordings/grand.json"),
        "null" => include_str!("fixtures/recordings/null.json"),
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(json).unwrap()
}

/// The same record as game 2: the human sits in Hinterhand and defends.
fn late(name: &str) -> GameRecording {
    let mut r = fixture(name);
    r.context.number = 2;
    r.context.dealer = PlayerId(0);
    r
}

const SMALL: Budget = Budget {
    samples: 2,
    nodes_per_task: 1_000_000,
};

/// Runs the review to its end, one job at a time.
fn run(coach: &mut ReplayCoach) -> ReviewSummary {
    assert!(coach.request_review());
    while let Some(job) = coach.review_job() {
        assert!(coach.review_running());
        assert!(coach.review_job().is_none(), "one analysis at a time");
        coach.finish_review(job.run(None)).unwrap();
    }
    done(coach)
}

fn done(coach: &ReplayCoach) -> ReviewSummary {
    match coach.review_status() {
        ReviewStatus::Done(s) => *s,
        other => panic!("not done: {other:?}"),
    }
}

/// Independent expectation: single historical comments (D-67) on every
/// decision with a choice, in game order.
fn singles(recording: &GameRecording) -> (Vec<usize>, Vec<coach_model::MoveReview>) {
    let mut coach = ReplayCoach::open(recording, SMALL).unwrap();
    let mut eligible = Vec::new();
    let mut reviews = Vec::new();
    for i in 0..coach.decisions().len() {
        if coach.decisions()[i].options == 1 {
            continue;
        }
        coach.select(Some(i));
        let job = coach.request().unwrap();
        coach.finish(job.run(None)).unwrap();
        let ReplayStatus::Ready(c) = coach.view().status else {
            panic!("single comment unavailable");
        };
        eligible.push(i);
        reviews.push(c.review);
    }
    (eligible, reviews)
}

/// The shared selection over those comments, as decision indices.
fn expected(
    singles: &(Vec<usize>, Vec<coach_model::MoveReview>),
    focus: Option<coach_model::Concept>,
) -> Vec<usize> {
    coach_model::moments(&singles.1, focus)
        .iter()
        .map(|m| singles.0[m.index])
        .collect()
}

#[test]
fn review_selects_shared_moments_in_game_order_navigable_to_the_decision() {
    let mut total = 0;
    for recording in [
        fixture("herz"),
        fixture("grand"),
        fixture("null"),
        late("herz"),
    ] {
        let mut coach = ReplayCoach::open(&recording, SMALL).unwrap();
        let decisions = coach.decisions().to_vec();
        let forced = decisions.iter().filter(|d| d.options == 1).count() as u32;
        assert_eq!(
            coach.review_status(),
            ReviewStatus::Idle {
                eligible: decisions.len() as u32 - forced
            },
            "nothing is computed before the request"
        );
        let s = run(&mut coach);
        assert_eq!(s.end, ReviewEnd::Finished);
        assert_eq!(
            (s.decisions, s.forced, s.eligible),
            (
                decisions.len() as u32,
                forced,
                decisions.len() as u32 - forced
            )
        );
        assert_eq!(s.analysed + s.unavailable, s.eligible);
        assert_eq!(s.unavailable, 0, "no limit is reached without a deadline");
        assert!(s.exhaustive());
        assert!(s.moments.len() <= coach_model::PRIMARY_MOMENTS);
        total += s.moments.len();
        let found: Vec<usize> = s.moments.iter().map(|m| m.decision).collect();
        assert_eq!(
            found,
            expected(&singles(&recording), None),
            "shared D-58 selection"
        );
        assert!(found.windows(2).all(|w| w[0] < w[1]), "game order");
        let mut concepts: Vec<_> = s.moments.iter().map(|m| m.moment.concept).collect();
        concepts.dedup();
        assert_eq!(concepts.len(), s.moments.len(), "one moment per concept");
        for m in &s.moments {
            let d = decisions[m.decision];
            assert!(d.options > 1, "forced decisions are never moments");
            assert_eq!(m.moment.review.chosen.action, d.action);
            assert!(
                m.moment.review.lesson_for(None).is_some(),
                "supported lesson"
            );
            assert!(!m.moment.review.basis.known, "never hindsight");
            // Navigating selects the exact historical table before the
            // action and shows the same comment, without computing.
            coach.select(Some(m.decision));
            let view = coach.view();
            assert_eq!(view.context.as_ref(), Some(&m.context));
            assert_eq!(view.perspective, ReplayPerspective::Historical);
            let ReplayStatus::Ready(c) = view.status else {
                panic!("cached historical comment expected");
            };
            assert_eq!(c.perspective, ReplayPerspective::Historical);
            assert_eq!(c.review, m.moment.review);
            assert_eq!(c.alternative, m.alternative);
            if let Some(alt) = &m.alternative {
                assert_ne!(alt.action, d.action);
                let human = PlayerId::HUMAN.seat(recording.context.dealer);
                let mut game = skat_core::Game::from_deal(recording.deal, recording.rules);
                for &a in &recording.actions[..d.prefix] {
                    game.apply(a).unwrap();
                }
                assert!(
                    game.view(human).legal_actions.contains(&alt.action),
                    "legal"
                );
            }
        }
        // Forced decisions are selectable but have no cached comment.
        if let Some(f) = decisions.iter().position(|d| d.options == 1) {
            coach.select(Some(f));
            assert_eq!(coach.view().status, ReplayStatus::Idle);
        }
    }
    assert!(total > 0, "the fixtures teach at least one moment");
}

#[test]
fn review_jobs_hold_only_the_historical_view_and_its_view_seed() {
    let r = late("herz");
    let human = PlayerId::HUMAN.seat(r.context.dealer);
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    assert!(coach.request_review());
    let mut seen = Vec::new();
    while let Some(job) = coach.review_job() {
        let d = coach.decisions()[job.decision()];
        // Independent core execution of the prefix.
        let mut game = skat_core::Game::from_deal(r.deal, r.rules);
        for &a in &r.actions[..d.prefix] {
            game.apply(a).unwrap();
        }
        assert_eq!(job.view(), &game.view(human));
        assert_eq!(
            job.seed(),
            coach_model::decision_seed(REPLAY_COACH_SEED, job.view())
        );
        assert_ne!(job.seed(), r.seed);
        assert_eq!(job.budget(), SMALL);
        seen.push(job.decision());
        coach.finish_review(job.run(None)).unwrap();
    }
    // Every decision with a choice exactly once, in game order; forced
    // ones are skipped without shifting indices.
    let expected: Vec<usize> = (0..coach.decisions().len())
        .filter(|&i| coach.decisions()[i].options > 1)
        .collect();
    assert_eq!(seen, expected);
}

#[test]
fn same_inputs_give_the_same_review_and_focus_only_reselects() {
    let r = fixture("herz");
    let mut a = ReplayCoach::open(&r, SMALL).unwrap();
    let mut b = ReplayCoach::open(&r, SMALL).unwrap();
    let first = run(&mut a);
    assert_eq!(first, run(&mut b), "same recording, seed and budget");
    let independent = singles(&r);
    // Every concept a reviewed decision teaches: the focus reorders the
    // selection over the same analysed decisions, nothing is recomputed.
    for concept in coach_model::Concept::ALL {
        a.set_review_focus(Some(concept));
        assert!(a.review_job().is_none(), "focus is no analysis input");
        let s = done(&a);
        assert_eq!(s.focus, Some(concept));
        assert_eq!(s.analysed, first.analysed);
        let found: Vec<usize> = s.moments.iter().map(|m| m.decision).collect();
        assert_eq!(found, expected(&independent, Some(concept)));
        for m in &s.moments {
            assert_eq!(m.moment.focused, m.moment.concept == concept);
        }
    }
    a.set_review_focus(None);
    assert_eq!(done(&a), first);
}

#[test]
fn cancel_time_limit_retry_budget_and_stale_results() {
    let r = fixture("herz");
    let mut full = ReplayCoach::open(&r, SMALL).unwrap();
    let whole = run(&mut full);

    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    assert!(coach.request_review());
    let job = coach.review_job().unwrap();
    let first = job.decision();
    assert_eq!(
        coach.review_status(),
        ReviewStatus::Running {
            attempted: 0,
            eligible: whole.eligible
        }
    );
    coach.finish_review(job.run(None)).unwrap();
    assert_eq!(
        coach.review_status(),
        ReviewStatus::Running {
            attempted: 1,
            eligible: whole.eligible
        }
    );
    // At most one replay analysis: no single comment while it runs, and
    // selecting a decision does not stop it.
    coach.select(Some(first));
    assert!(coach.request().is_none());
    coach.set_perspective(ReplayPerspective::Hindsight);
    assert!(coach.request().is_none());
    assert!(coach.review_running());
    // Cancel: partial, labelled, never exhaustive; the late result is stale.
    let late_job = coach.review_job().unwrap();
    coach.cancel_review();
    assert_eq!(
        coach.finish_review(late_job.run(None)),
        Err(ReplayStale::Stale)
    );
    let s = done(&coach);
    assert_eq!(s.end, ReviewEnd::Cancelled);
    assert_eq!((s.analysed, s.unavailable), (1, 0));
    assert_eq!(s.pending(), s.eligible - 1);
    assert!(!s.exhaustive());
    assert!(s.moments.iter().all(|m| m.decision == first));
    // Retry continues after the analysed decision.
    assert!(coach.request_review());
    let job = coach.review_job().unwrap();
    assert!(job.decision() > first, "the analysed decision is kept");
    // A deadline inside one decision: unavailable, the review goes on.
    let stop = Cancel::new();
    stop.cancel();
    let cut = job.run(Some(&stop));
    assert!(cut.cancelled());
    let cut_decision = cut.decision();
    coach.finish_review(cut).unwrap();
    assert!(coach.review_running());
    // The total limit: partial as well.
    coach.review_time_limit();
    let s = done(&coach);
    assert_eq!(s.end, ReviewEnd::TimeLimit);
    assert_eq!((s.analysed, s.unavailable), (1, 1));
    assert!(!s.exhaustive());
    // Retry tries the unavailable decision again and finishes equal to the
    // uninterrupted run.
    assert!(coach.request_review());
    assert_eq!(coach.review_job().unwrap().decision(), cut_decision);
    coach.cancel_review();
    assert_eq!(run(&mut coach), whole);

    // A budget change resets the review and its cache; old jobs are stale.
    assert!(coach.request_review());
    assert_eq!(coach.review_job().map(|j| j.decision()), None, "all cached");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    assert!(coach.request_review());
    let job = coach.review_job().unwrap();
    coach.set_budget(Budget {
        samples: 2,
        ..SMALL
    });
    assert_eq!(
        coach.review_status(),
        ReviewStatus::Idle {
            eligible: whole.eligible
        }
    );
    assert_eq!(coach.finish_review(job.run(None)), Err(ReplayStale::Stale));
    // Results of another opening of the same file are stale.
    let mut other = ReplayCoach::open(&r, SMALL).unwrap();
    assert!(other.request_review());
    assert!(coach.request_review());
    let theirs = other.review_job().unwrap().run(None);
    let _ = coach.review_job().unwrap();
    assert_eq!(coach.finish_review(theirs), Err(ReplayStale::Stale));
    // While a single comment computes, the review cannot start.
    let mut single = ReplayCoach::open(&r, SMALL).unwrap();
    single.select(Some(1));
    let pending = single.request().unwrap();
    assert!(!single.request_review());
    single.finish(pending.run(None)).unwrap();
    assert!(single.request_review());
}

#[test]
fn hindsight_stays_separate_from_the_review() {
    let r = fixture("herz");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    let before = run(&mut coach);
    let i = before.moments.first().map_or_else(
        || {
            coach
                .decisions()
                .iter()
                .position(|d| d.options > 1)
                .unwrap()
        },
        |m| m.decision,
    );
    coach.select(Some(i));
    coach.set_perspective(ReplayPerspective::Hindsight);
    assert_eq!(coach.view().status, ReplayStatus::Idle, "never cached");
    let job = coach.request().unwrap();
    assert!(job.sees_all_cards());
    coach.finish(job.run(None)).unwrap();
    let ReplayStatus::Ready(h) = coach.view().status else {
        panic!("hindsight comment");
    };
    assert!(h.review.basis.known);
    assert_eq!(h.perspective, ReplayPerspective::Hindsight);
    // The review is unchanged and its comment stays historical.
    assert_eq!(done(&coach), before);
    coach.set_perspective(ReplayPerspective::Historical);
    let ReplayStatus::Ready(c) = coach.view().status else {
        panic!("cached historical comment");
    };
    assert!(!c.review.basis.known);
    assert_eq!(c.perspective, ReplayPerspective::Historical);
}

#[test]
fn all_pass_and_incomplete_recordings_report_honest_scope() {
    // All pass: the human's one pass is a real decision; at most one moment
    // and only with a supported lesson.
    let mut all_pass = Session::new(SessionConfig::default())
        .export_current_recording_privileged()
        .unwrap();
    all_pass.actions = vec![Action::Pass; 3];
    all_pass.end = RecordingEnd::Eingepasst;
    let mut coach = ReplayCoach::open(&all_pass, SMALL).unwrap();
    let s = run(&mut coach);
    assert_eq!((s.decisions, s.forced, s.eligible), (1, 0, 1));
    assert_eq!(s.end, ReviewEnd::Finished);
    assert!(s.moments.len() <= 1);
    let found: Vec<usize> = s.moments.iter().map(|m| m.decision).collect();
    assert_eq!(found, expected(&singles(&all_pass), None));

    // In-progress files: the review is unavailable, also with recorded
    // human decisions; single comments remain.
    let mut live = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    let before_human = live.export_current_recording_privileged().unwrap();
    live.run_opponents();
    let action = live.view().legal_actions[0];
    live.execute(Command::Act(action)).unwrap();
    live.run_opponents();
    let after_human = live.export_current_recording_privileged().unwrap();
    for file in [before_human, after_human] {
        assert_eq!(file.end, RecordingEnd::InProgress);
        let mut coach = ReplayCoach::open(&file, SMALL).unwrap();
        assert_eq!(
            coach.review_status(),
            ReviewStatus::Unavailable(ReviewUnavailable::Incomplete)
        );
        assert!(!coach.request_review());
        assert!(coach.review_job().is_none());
    }
}

#[test]
fn review_leaves_the_live_session_its_pending_job_and_the_file_untouched() {
    let mut live = Session::new(SessionConfig {
        seed: 3,
        series: SeriesLength::Fixed(3),
        kontra_re: true,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    live.set_coach_budget(SMALL);
    live.run_opponents();
    live.execute(Command::RequestAdvice).unwrap();
    let pending = live.coach_job().expect("live coach job pending");
    let before = live.view();
    let recording = live.export_current_recording_privileged().unwrap();
    let herz = fixture("herz");
    let bytes = serde_json::to_vec(&herz).unwrap();
    let mut coach = ReplayCoach::open(&herz, SMALL).unwrap();
    run(&mut coach);
    assert_eq!(live.view(), before);
    assert!(live.coach_job().unwrap().same_decision(&pending));
    assert_eq!(
        live.export_current_recording_privileged().unwrap(),
        recording
    );
    assert_eq!(serde_json::to_vec(coach.recording()).unwrap(), bytes);
    live.finish_coach(pending.run(None)).unwrap();
}

/// Production budget and limits on the frozen fixtures, as the recording
/// window runs them (release: `docs/testing.md`).
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn production_budget_review_stays_within_its_limits() {
    for recording in [
        fixture("herz"),
        fixture("grand"),
        fixture("null"),
        late("herz"),
    ] {
        let mut coach = ReplayCoach::open(&recording, Budget::INTERACTIVE).unwrap();
        let start = std::time::Instant::now();
        assert!(coach.request_review());
        while let Some(job) = coach.review_job() {
            let deadline = (std::time::Instant::now() + REPLAY_REVIEW_DECISION_LIMIT)
                .min(start + REPLAY_REVIEW_TIME_LIMIT);
            let cancel = Cancel::with_deadline(deadline);
            let result = job.run(Some(&cancel));
            coach.finish_review(result).unwrap();
            if start.elapsed() >= REPLAY_REVIEW_TIME_LIMIT {
                coach.review_time_limit();
            }
        }
        let s = done(&coach);
        assert!(start.elapsed() < REPLAY_REVIEW_TIME_LIMIT);
        assert_eq!(s.end, ReviewEnd::Finished);
        assert!(s.exhaustive(), "{s:?}");
        assert!(
            s.moments
                .iter()
                .all(|m| m.moment.review.status == Status::Complete
                    || m.moment.kind == coach_model::MomentKind::Neutral)
        );
    }
}
