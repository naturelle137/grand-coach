// SPDX-License-Identifier: EUPL-1.2

//! Provenance, response categories and earlier/recent comparisons of the
//! learning history (format 3, D-70). Every expected number below is
//! written by hand from the fixture events, not computed by the production
//! aggregation: exact, too high, too low, unrecorded legacy mismatch and
//! skips, with and without help, per context, and the window boundaries at
//! 19, 20 and 21 settled responses. Legacy format 1 and 2 events stay
//! unversioned; inconsistent or missing provenance is refused.

use skat_app::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const QUIZ: LessonRevision = LessonFamily::CountingQuiz.current();
const FOCUSED: LessonRevision = LessonFamily::FocusedRetrieval.current();
const TRAINING: LessonRevision = LessonFamily::CountingTraining.current();
const VOIDS: QuestionKind = QuestionKind::ProvenVoids {
    class: skat_core::Follow::Farbe(skat_core::Suit::Herz),
};

fn temp_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "skat-learning-responses-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}

fn game(series: u64, trick: u8) -> OpportunityId {
    QuestionId {
        series,
        game: 1,
        trick,
    }
    .into()
}

fn ev(
    id: OpportunityId,
    question: QuestionKind,
    action: RetrievalAction,
    assisted: bool,
    lesson: LessonRevision,
) -> RetrievalEvent {
    RetrievalEvent {
        id,
        question,
        action,
        assistance: if assisted {
            AssistanceLevel::RevealOnRequest
        } else {
            AssistanceLevel::AnswerBeforeReveal
        },
        assisted,
        lesson: Some(lesson),
    }
}

fn answer(given: u8, expected: u8) -> RetrievalAction {
    RetrievalAction::Answer {
        given,
        correct: given == expected,
        expected: Some(expected),
    }
}

const OPP: RetrievalAction = RetrievalAction::Opportunity;
const HELP: RetrievalAction = RetrievalAction::Help;
const SKIP: RetrievalAction = RetrievalAction::Skip { disabled: false };

fn counts(
    exact: u32,
    too_high: u32,
    too_low: u32,
    unrecorded: u32,
    skipped: u32,
) -> CategoryCounts {
    CategoryCounts {
        exact,
        too_high,
        too_low,
        mismatch_unrecorded: unrecorded,
        skipped,
    }
}

fn row(summary: &LearningSummary, topic: LearningTopic, context: LearningContext) -> LearningRow {
    *summary
        .rows
        .iter()
        .find(|r| r.topic == topic && r.context == context)
        .unwrap_or_else(|| panic!("no row {topic:?} {context:?}"))
}

/// The hand-authored mixed history: a series with focused retrieval, one
/// full-quiz question, and a training presentation.
fn mixed() -> LearningHistory {
    let mut h = LearningHistory::new();
    let series = h.begin_attempt(AttemptKind::Series).unwrap();
    let t = QuestionKind::TrumpfOutside;
    h.record(
        series,
        &[
            // Trumpf at trick 3: exact, unaided.
            ev(game(5, 3), t, OPP, false, FOCUSED),
            ev(game(5, 3), t, answer(3, 3), false, FOCUSED),
            // Voids at trick 6: help, then 0 for 2 (too low, assisted).
            ev(game(5, 6), VOIDS, OPP, false, FOCUSED),
            ev(game(5, 6), VOIDS, HELP, true, FOCUSED),
            ev(game(5, 6), VOIDS, answer(0, 2), true, FOCUSED),
        ],
    )
    .unwrap();
    // A second batch of the same attempt: 5 for 3 (too high), a skip and
    // an opportunity that is still open.
    h.record(
        series,
        &[
            ev(game(6, 3), t, OPP, false, QUIZ),
            ev(game(6, 3), t, answer(5, 3), false, QUIZ),
            ev(game(6, 6), VOIDS, OPP, false, FOCUSED),
            ev(game(6, 6), VOIDS, SKIP, false, FOCUSED),
            ev(game(7, 3), t, OPP, false, FOCUSED),
        ],
    )
    .unwrap();
    let training = h
        .begin_attempt(AttemptKind::Training { attempt: 1 })
        .unwrap();
    let task = OpportunityId::Training(TrainingFixture::TrumpfDeclarerFarbspiel.task());
    h.record(
        training,
        &[
            ev(task, t, OPP, false, TRAINING),
            ev(task, t, answer(1, 2), false, TRAINING),
        ],
    )
    .unwrap();
    h
}

#[test]
fn response_categories_are_counted_per_topic_context_and_support() {
    let h = mixed();
    let s = h.summary();
    let trumpf = row(&s, LearningTopic::TrumpfOutside, LearningContext::Series);
    assert_eq!(trumpf.counts.opportunities, 3);
    assert_eq!(trumpf.counts.answers, 2);
    assert_eq!(trumpf.responses.unaided, counts(1, 1, 0, 0, 0));
    assert_eq!(trumpf.responses.assisted, counts(0, 0, 0, 0, 0));
    let voids = row(&s, LearningTopic::ProvenVoids, LearningContext::Series);
    assert_eq!(
        (voids.counts.opportunities, voids.counts.help),
        (2, 1),
        "help is its own count"
    );
    assert_eq!(voids.responses.unaided, counts(0, 0, 0, 0, 1));
    assert_eq!(voids.responses.assisted, counts(0, 0, 1, 0, 0));
    let training = row(&s, LearningTopic::TrumpfOutside, LearningContext::Training);
    assert_eq!(training.responses.unaided, counts(0, 0, 1, 0, 0));
    assert_eq!(s.rows.len(), 3);
    // Totals: 6 opportunities, 5 settled, one still open.
    assert_eq!(s.totals.opportunities, 6);
    assert_eq!(s.responses.settled(), 5);
    assert_eq!(s.responses.unaided, counts(1, 1, 1, 0, 1));
    assert_eq!(s.responses.assisted, counts(0, 0, 1, 0, 0));
    // Lesson revisions per opportunity, in family order.
    assert_eq!(
        s.provenance,
        [
            ProvenanceCount {
                lesson: Some(QUIZ),
                opportunities: 1
            },
            ProvenanceCount {
                lesson: Some(FOCUSED),
                opportunities: 4
            },
            ProvenanceCount {
                lesson: Some(TRAINING),
                opportunities: 1
            },
        ]
    );
    // Sparse: every topic has fewer than 20 settled responses.
    assert_eq!(
        s.comparisons
            .iter()
            .map(|c| (c.topic, c.settled, c.windows.is_some()))
            .collect::<Vec<_>>(),
        [
            (LearningTopic::TrumpfOutside, 3, false),
            (LearningTopic::ProvenVoids, 2, false)
        ]
    );
}

#[test]
fn the_category_of_each_action_is_only_the_observed_response() {
    use ResponseCategory::*;
    let legacy_wrong = RetrievalAction::Answer {
        given: 4,
        correct: false,
        expected: None,
    };
    let legacy_exact = RetrievalAction::Answer {
        given: 4,
        correct: true,
        expected: None,
    };
    for (action, category) in [
        (OPP, None),
        (HELP, None),
        (answer(2, 2), Some(Exact)),
        (answer(0, 0), Some(Exact)),
        (answer(3, 2), Some(TooHigh)),
        (answer(1, 2), Some(TooLow)),
        (answer(0, 11), Some(TooLow)),
        (legacy_wrong, Some(MismatchUnrecorded)),
        (legacy_exact, Some(Exact)),
        (SKIP, Some(Skipped)),
        (RetrievalAction::Skip { disabled: true }, Some(Skipped)),
    ] {
        assert_eq!(ResponseCategory::of(action), category, "{action:?}");
    }
}

#[test]
fn empty_history_has_no_categories_provenance_or_comparisons() {
    let s = LearningHistory::new().summary();
    assert_eq!(s.responses, ResponseCounts::default());
    assert!(s.provenance.is_empty() && s.comparisons.is_empty());
    // Opportunities and help alone settle nothing.
    let mut h = LearningHistory::new();
    let a = h.begin_attempt(AttemptKind::Series).unwrap();
    h.record(
        a,
        &[
            ev(game(1, 3), VOIDS, OPP, false, FOCUSED),
            ev(game(1, 3), VOIDS, HELP, true, FOCUSED),
        ],
    )
    .unwrap();
    let s = h.summary();
    assert_eq!(s.responses.settled(), 0);
    assert!(s.comparisons.is_empty());
    assert_eq!(s.provenance.len(), 1);
}

/// `n` settled Trumpf responses: numbers 1 to 10 are 4 for 3 (too high)
/// in a series attempt, 11 to 20 exact in a practice attempt, 21 a skip in
/// a second series attempt. Proven-void questions are interleaved; they
/// belong to their own topic.
fn trumpf_sequence(n: u8) -> LearningHistory {
    let mut h = LearningHistory::new();
    let first = h.begin_attempt(AttemptKind::Series).unwrap();
    let practice = h
        .begin_attempt(AttemptKind::Practice { attempt: 1 })
        .unwrap();
    let last = h.begin_attempt(AttemptKind::Series).unwrap();
    let t = QuestionKind::TrumpfOutside;
    for i in 1..=n {
        let (attempt, action) = match i {
            1..=10 => (first, answer(4, 3)),
            11..=20 => (practice, answer(3, 3)),
            _ => (last, SKIP),
        };
        h.record(
            attempt,
            &[
                ev(game(u64::from(i), 3), t, OPP, false, FOCUSED),
                ev(game(u64::from(i), 3), t, action, false, FOCUSED),
                ev(game(u64::from(i), 6), VOIDS, OPP, false, FOCUSED),
                ev(game(u64::from(i), 6), VOIDS, SKIP, false, FOCUSED),
            ],
        )
        .unwrap();
    }
    h
}

fn trumpf_comparison(h: &LearningHistory) -> TopicComparison {
    h.summary()
        .comparisons
        .into_iter()
        .find(|c| c.topic == LearningTopic::TrumpfOutside)
        .unwrap()
}

#[test]
fn comparison_windows_have_deterministic_boundaries() {
    assert_eq!(COMPARISON_WINDOW, 10);
    let c = trumpf_comparison(&trumpf_sequence(19));
    assert_eq!((c.settled, c.windows), (19, None));

    let c = trumpf_comparison(&trumpf_sequence(20));
    assert_eq!(c.settled, 20);
    let [earlier, recent] = c.windows.unwrap();
    assert_eq!((earlier.first, earlier.last), (1, 10));
    assert_eq!(
        (earlier.lowest_attempt, earlier.highest_attempt),
        (AttemptId(1), AttemptId(1))
    );
    assert_eq!(earlier.responses.unaided, counts(0, 10, 0, 0, 0));
    assert_eq!(
        earlier.contexts,
        [ContextCount {
            context: LearningContext::Series,
            settled: 10
        }]
    );
    assert_eq!((recent.first, recent.last), (11, 20));
    assert_eq!(
        (recent.lowest_attempt, recent.highest_attempt),
        (AttemptId(2), AttemptId(2))
    );
    assert_eq!(recent.responses.unaided, counts(10, 0, 0, 0, 0));
    assert_eq!(
        recent.contexts,
        [ContextCount {
            context: LearningContext::Practice,
            settled: 10
        }]
    );

    // One more response moves both windows by one.
    let c = trumpf_comparison(&trumpf_sequence(21));
    assert_eq!(c.settled, 21);
    let [earlier, recent] = c.windows.unwrap();
    assert_eq!((earlier.first, earlier.last), (2, 11));
    assert_eq!(earlier.responses.unaided, counts(1, 9, 0, 0, 0));
    assert_eq!(
        (earlier.lowest_attempt, earlier.highest_attempt),
        (AttemptId(1), AttemptId(2))
    );
    assert_eq!(
        earlier.contexts,
        [
            ContextCount {
                context: LearningContext::Series,
                settled: 9
            },
            ContextCount {
                context: LearningContext::Practice,
                settled: 1
            }
        ]
    );
    assert_eq!((recent.first, recent.last), (12, 21));
    assert_eq!(recent.responses.unaided, counts(9, 0, 0, 0, 1));
    assert_eq!(
        (recent.lowest_attempt, recent.highest_attempt),
        (AttemptId(2), AttemptId(3))
    );
    assert_eq!(recent.responses.assisted, CategoryCounts::default());
    // The interleaved void questions are their own topic: 21 skips.
    let voids = h_voids(&trumpf_sequence(21));
    assert_eq!(voids.settled, 21);
    let [e, r] = voids.windows.unwrap();
    assert_eq!(e.responses.unaided, counts(0, 0, 0, 0, 10));
    assert_eq!(r.responses.unaided, counts(0, 0, 0, 0, 10));
}

fn h_voids(h: &LearningHistory) -> TopicComparison {
    h.summary()
        .comparisons
        .into_iter()
        .find(|c| c.topic == LearningTopic::ProvenVoids)
        .unwrap()
}

#[test]
fn windows_follow_persisted_order_even_when_attempt_ids_interleave() {
    // Attempt 1 (series) gets 10 answers, attempt 2 (training) one, then
    // attempt 1 again 9: the recent window spans attempts 1 to 2 although
    // its last response belongs to attempt 1.
    let mut h = LearningHistory::new();
    let series = h.begin_attempt(AttemptKind::Series).unwrap();
    let training = h
        .begin_attempt(AttemptKind::Training { attempt: 2 })
        .unwrap();
    let t = QuestionKind::TrumpfOutside;
    let task = OpportunityId::Training(TrainingFixture::TrumpfDefenderGrand.task());
    for i in 1..=20u8 {
        if i == 11 {
            h.record(
                training,
                &[
                    ev(task, t, OPP, true, TRAINING),
                    ev(task, t, answer(2, 2), true, TRAINING),
                ],
            )
            .unwrap();
            continue;
        }
        h.record(
            series,
            &[
                ev(game(1, i), t, OPP, false, FOCUSED),
                ev(game(1, i), t, answer(1, 2), false, FOCUSED),
            ],
        )
        .unwrap();
    }
    let [earlier, recent] = trumpf_comparison(&h).windows.unwrap();
    assert_eq!(
        (earlier.lowest_attempt, earlier.highest_attempt),
        (AttemptId(1), AttemptId(1))
    );
    assert_eq!(earlier.responses.unaided, counts(0, 0, 10, 0, 0));
    assert_eq!(
        (recent.lowest_attempt, recent.highest_attempt),
        (AttemptId(1), AttemptId(2))
    );
    assert_eq!(recent.responses.unaided, counts(0, 0, 9, 0, 0));
    assert_eq!(recent.responses.assisted, counts(1, 0, 0, 0, 0));
    assert_eq!(
        recent.contexts,
        [
            ContextCount {
                context: LearningContext::Series,
                settled: 9
            },
            ContextCount {
                context: LearningContext::TrainingRepeat,
                settled: 1
            }
        ]
    );
}

#[test]
fn duplicates_and_reloads_never_multiply_categories_or_windows() {
    let mut h = trumpf_sequence(21);
    let once = h.summary();
    let events: Vec<_> = h.events().iter().map(|e| (e.attempt, e.event)).collect();
    for (attempt, event) in &events {
        assert_eq!(h.record(*attempt, [event]).unwrap().added, 0);
    }
    assert_eq!(h.summary(), once);
    let dir = temp_dir();
    let path = dir.join("learning-history.json");
    h.save_local(&path).unwrap();
    let mut reloaded = LearningHistory::open_local(&path).unwrap();
    assert_eq!(reloaded, h);
    for (attempt, event) in &events {
        assert_eq!(reloaded.record(*attempt, [event]).unwrap().added, 0);
    }
    assert_eq!(reloaded.summary(), once);
    // Reset forgets categories and windows with the events.
    reloaded.reset();
    assert!(reloaded.summary().comparisons.is_empty());
    assert_eq!(reloaded.summary().responses, ResponseCounts::default());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn new_events_without_or_with_inconsistent_provenance_are_refused() {
    let mut h = LearningHistory::new();
    let a = h.begin_attempt(AttemptKind::Series).unwrap();
    let t = QuestionKind::TrumpfOutside;
    let unversioned = RetrievalEvent {
        lesson: None,
        ..ev(game(1, 3), t, OPP, false, FOCUSED)
    };
    let no_expected = ev(
        game(1, 3),
        t,
        RetrievalAction::Answer {
            given: 2,
            correct: false,
            expected: None,
        },
        false,
        FOCUSED,
    );
    let contradicting = ev(
        game(1, 3),
        t,
        RetrievalAction::Answer {
            given: 2,
            correct: true,
            expected: Some(3),
        },
        false,
        FOCUSED,
    );
    let wrong_family = ev(game(1, 3), t, OPP, false, TRAINING);
    let version_zero = ev(
        game(1, 3),
        t,
        OPP,
        false,
        LessonRevision {
            family: LessonFamily::FocusedRetrieval,
            version: 0,
        },
    );
    for bad in [
        vec![unversioned],
        vec![no_expected],
        vec![contradicting],
        vec![wrong_family],
        vec![version_zero],
        // The lesson cannot change within one opportunity, in one batch...
        vec![
            ev(game(1, 3), t, OPP, false, FOCUSED),
            ev(game(1, 3), t, answer(2, 2), false, QUIZ),
        ],
    ] {
        assert_eq!(
            h.record(a, &bad),
            Err(LearningError::InvalidProvenance),
            "{bad:?}"
        );
        assert!(h.is_empty(), "nothing recorded");
    }
    // ...or across batches.
    h.record(a, &[ev(game(1, 3), t, OPP, false, FOCUSED)])
        .unwrap();
    assert_eq!(
        h.record(a, &[ev(game(1, 3), t, answer(2, 2), false, QUIZ)]),
        Err(LearningError::InvalidProvenance)
    );
    // The same question ID in another attempt is another opportunity.
    let b = h
        .begin_attempt(AttemptKind::Practice { attempt: 1 })
        .unwrap();
    h.record(b, &[ev(game(1, 3), t, OPP, false, QUIZ)]).unwrap();
    assert_eq!(h.events().len(), 2);
}

/// A format 2 file as the PR #137 build wrote it: a series with a wrong
/// and an exact answer, a training task with a skip after help.
const FORMAT_2: &str = r#"{"format":2,"next_attempt":3,"attempts":[{"id":1,"kind":"Series"},{"id":2,"kind":{"Training":{"attempt":1}}}],"events":[{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":{"Answer":{"given":4,"correct":false}},"assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":6},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":7,"game":1,"trick":6},"question":"TrumpfOutside","action":{"Answer":{"given":1,"correct":true}},"assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":"Opportunity","assistance":"RevealOnRequest","assisted":false}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":"Help","assistance":"RevealOnRequest","assisted":true}},{"attempt":2,"event":{"id":{"fixture":"VoidsDeclarerGrand","version":1},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":{"Skip":{"disabled":false}},"assistance":"RevealOnRequest","assisted":true}}]}"#;

/// A format 1 file as the PR #133 build wrote it: one wrong answer.
const FORMAT_1: &str = r#"{"format":1,"next_attempt":2,"attempts":[{"id":1,"kind":"Series"}],"events":[{"attempt":1,"event":{"id":{"series":3,"game":2,"trick":3},"question":{"ProvenVoids":{"class":{"Farbe":"Pik"}}},"action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":1,"event":{"id":{"series":3,"game":2,"trick":3},"question":{"ProvenVoids":{"class":{"Farbe":"Pik"}}},"action":{"Answer":{"given":0,"correct":false}},"assistance":"AnswerBeforeReveal","assisted":false}}]}"#;

#[test]
fn format_1_and_2_events_migrate_unversioned_with_unknown_deviation() {
    let dir = temp_dir();
    for (n, (text, opportunities, unaided)) in [
        (FORMAT_1, 1, counts(0, 0, 0, 1, 0)),
        (FORMAT_2, 3, counts(1, 0, 0, 1, 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let path = dir.join(format!("history-{n}.json"));
        fs::write(&path, text).unwrap();
        let mut h = LearningHistory::open_local(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), text, "read only");
        assert!(h.events().iter().all(|e| e.event.lesson.is_none()));
        let s = h.summary();
        assert_eq!(
            s.provenance,
            [ProvenanceCount {
                lesson: None,
                opportunities
            }]
        );
        // The wrong legacy answer is a mismatch without a known deviation.
        assert_eq!(s.responses.unaided, unaided);
        // Order, attempts and counter are kept; saving writes format 3
        // with the legacy events byte-identical in shape.
        let old: serde_json::Value = serde_json::from_str(text).unwrap();
        h.save_local(&path).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["format"], 3);
        assert_eq!(saved["events"], old["events"]);
        assert_eq!(saved["attempts"], old["attempts"]);
        assert_eq!(saved["next_attempt"], old["next_attempt"]);
        let reread = LearningHistory::open_local(&path).unwrap();
        assert_eq!(reread, h);
        // New events beside them are versioned; legacy ones stay unknown.
        let next = h.begin_attempt(AttemptKind::Series).unwrap();
        assert_eq!(next.0, old["next_attempt"].as_u64().unwrap());
        h.record(
            next,
            &[
                ev(game(9, 3), VOIDS, OPP, false, FOCUSED),
                ev(game(9, 3), VOIDS, answer(2, 1), false, FOCUSED),
            ],
        )
        .unwrap();
        let s = h.summary();
        assert_eq!(
            s.provenance,
            [
                ProvenanceCount {
                    lesson: None,
                    opportunities
                },
                ProvenanceCount {
                    lesson: Some(FOCUSED),
                    opportunities: 1
                }
            ]
        );
        assert_eq!(s.responses.unaided.too_high, 1);
        assert_eq!(s.responses.unaided.mismatch_unrecorded, 1);
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn files_with_impossible_provenance_are_reported_and_kept() {
    let mut format_2_lesson: serde_json::Value = serde_json::from_str(FORMAT_2).unwrap();
    format_2_lesson["events"][0]["event"]["lesson"] = serde_json::to_value(FOCUSED).unwrap();
    let mut format_2_expected: serde_json::Value = serde_json::from_str(FORMAT_2).unwrap();
    format_2_expected["events"][1]["event"]["action"]["Answer"]["expected"] = 3.into();
    let mut format_1_lesson: serde_json::Value = serde_json::from_str(FORMAT_1).unwrap();
    format_1_lesson["events"][0]["event"]["lesson"] = serde_json::to_value(QUIZ).unwrap();

    let h = mixed();
    let good = serde_json::to_value(&h).unwrap();
    assert_eq!(good["format"], 3);
    let mut contradicting = good.clone();
    contradicting["events"][1]["event"]["action"]["Answer"]["expected"] = 4.into();
    let mut changed = good.clone();
    changed["events"][1]["event"]["lesson"] = serde_json::to_value(QUIZ).unwrap();
    let mut wrong_family = good.clone();
    wrong_family["events"][0]["event"]["lesson"] = serde_json::to_value(TRAINING).unwrap();
    wrong_family["events"][1]["event"]["lesson"] = serde_json::to_value(TRAINING).unwrap();
    let mut newer = good.clone();
    newer["format"] = 4.into();
    // A format 3 file may hold unversioned legacy events.
    let mut legacy = good.clone();
    legacy["events"][0]["event"]
        .as_object_mut()
        .unwrap()
        .remove("lesson");
    legacy["events"][1]["event"]
        .as_object_mut()
        .unwrap()
        .remove("lesson");
    legacy["events"][1]["event"]["action"]["Answer"]
        .as_object_mut()
        .unwrap()
        .remove("expected");

    let dir = temp_dir();
    let invalid = LearningFileError::Invalid(LearningError::InvalidProvenance);
    for (n, (value, result)) in [
        (format_2_lesson, Err(invalid)),
        (format_2_expected, Err(invalid)),
        (format_1_lesson, Err(invalid)),
        (contradicting, Err(invalid)),
        (changed, Err(invalid)),
        (wrong_family, Err(invalid)),
        (newer, Err(LearningFileError::UnsupportedFormat)),
        (legacy, Ok(())),
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = serde_json::to_vec(&value).unwrap();
        let path = dir.join(format!("case-{n}.json"));
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            LearningHistory::open_local(&path).map(|_| ()),
            result,
            "case {n}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes, "never rewritten");
    }
    fs::remove_dir_all(dir).unwrap();
}
