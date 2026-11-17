// SPDX-License-Identifier: EUPL-1.2

use super::*;
use crate::{
    AttemptKind, FocusAnswer as A, LearningHistory, LiveAbstichAction as L, LiveAbstichId,
    MechanismResponse as R,
};
#[path = "../../skat-analysis/tests/common/mod.rs"]
mod common;
use common::*;
use skat_core::{Announcement, Deal, GameType, Suit};

// Independent expectations and exact legal setups fixed by the commissioning
// brief and skat-analysis/tests/mechanisms.rs:185-312, not the classifier.
fn fixture(which: u8, swapped: bool) -> Session {
    let low = if swapped {
        deal(
            "CJ SJ HJ DJ CA CT CK CQ SA SK",
            "C9 C8 C7 HA HT HK DA DK DQ D7",
            "ST SQ S9 S8 S7 H8 H7 DT D9 D8",
            "HQ H9",
        )
    } else {
        deal(
            "CJ SJ HJ DJ CA CT CK CQ SA SK",
            "C9 HA HT HK HQ H9 DA DK DQ D7",
            "ST SQ S9 S8 S7 H8 H7 DT D9 D8",
            "C8 C7",
        )
    };
    let (deal, setup, plays) = match which {
        1 => (
            deal(
                "CJ SJ HA HT HK HQ H9 SA ST CA",
                "DJ H8 H7 SK SQ S9 CK CQ C9 DA",
                "HJ S8 S7 CT C8 C7 DT DK DQ D9",
                "D8 D7",
            ),
            vec![
                Action::Hand,
                Action::Announce(Announcement::plain(GameType::Farbe(Suit::Herz))),
            ],
            vec!["CJ", "H7", "HJ", "HA", "H8", "S7"],
        ),
        2 => (
            deal(
                "CJ SJ CA CT CK SA SK HA HT DA",
                "HJ DJ CQ C9 HK HQ H9 DK DQ D7",
                "ST SQ S9 S8 S7 H8 H7 DT D9 D8",
                "C8 C7",
            ),
            vec![
                Action::TakeSkat,
                Action::Discard(cards("HT DA")),
                Action::Announce(Announcement::plain(GameType::Farbe(Suit::Kreuz))),
            ],
            vec!["CJ", "C9", "D8", "SK", "D7", "S7"],
        ),
        _ => (
            low,
            if which == 4 {
                vec![
                    Action::TakeSkat,
                    Action::Discard(cards("C8 C7")),
                    Action::Announce(Announcement::plain(GameType::Farbe(Suit::Kreuz))),
                ]
            } else {
                vec![
                    Action::Hand,
                    Action::Announce(Announcement::plain(GameType::Farbe(Suit::Kreuz))),
                ]
            },
            vec!["CJ", "C9", "D8", "SK", "D7", "S7"],
        ),
    };
    at_position(deal, setup, plays)
}
fn at_position(deal: Deal, setup: Vec<Action>, plays: Vec<&str>) -> Session {
    let mut s = Session::new(SessionConfig {
        kontra_re: false,
        ..SessionConfig::default()
    });
    s.game = Game::from_deal(deal, RuleConfig { kontra_re: false });
    s.recording.deal = deal;
    let actions = [Action::Bid(18), Action::Hold, Action::Pass, Action::Pass]
        .into_iter()
        .chain(setup)
        .chain(plays.into_iter().map(|s| Action::Play(c(s))));
    for a in actions {
        if s.game.current_seat() == Some(Seat::Vorhand) {
            s.execute(Command::Act(a)).unwrap();
        } else {
            s.apply(a).unwrap();
        }
    }
    assert_eq!(s.human_decision().unwrap().number, 1);
    s
}
fn select(s: &mut Session, card: &str) -> LiveAbstichId {
    s.execute(Command::StartLiveAbstich).unwrap();
    s.execute(Command::SelectLiveAbstich(
        s.live_abstich().map_or((0, 0, 0), |v| v.boundary),
        c(card),
    ))
    .unwrap();
    s.live_abstich().unwrap().id.unwrap()
}
fn answer(s: &mut Session, id: LiveAbstichId, given: A) -> R {
    s.execute(Command::LiveAbstich(id, L::Answer(given)))
        .unwrap();
    s.live_abstich().unwrap().response.unwrap()
}
fn persist(h: &mut LearningHistory, attempt: crate::AttemptId, s: &Session) {
    h.record_live_abstich(attempt, s.live_abstich_observations())
        .unwrap();
}
fn reload(h: &LearningHistory) -> LearningHistory {
    LearningHistory::read_local(serde_json::to_vec(h).unwrap().as_slice()).unwrap()
}

#[test]
fn live_fixed_evidence_classifies_all_explicit_answers_and_never_plays() {
    for (which, card, expected) in [
        (
            1,
            "CA",
            [
                (A::Certain, R::OverstatedCertainty),
                (A::Absent, R::DeniedPossibility),
                (A::Possible, R::Correct),
            ],
        ),
        (
            2,
            "SA",
            [
                (A::Possible, R::UnderstatedCertainty),
                (A::Certain, R::Correct),
                (A::Absent, R::DeniedPossibility),
            ],
        ),
        (
            3,
            "SA",
            [
                (A::Certain, R::OverstatedCertainty),
                (A::Absent, R::DeniedPossibility),
                (A::Possible, R::Correct),
            ],
        ),
    ] {
        for (given, response) in expected {
            let mut s = fixture(which, false);
            let before = s.export_current_recording_privileged().unwrap();
            let sheet = s.view().sheet;
            let legal = s.view().legal_actions;
            let id = select(&mut s, card);
            assert_eq!(id.opponent, Some(Seat::Mittelhand));
            assert!(s.live_abstich().unwrap().evidence.is_none());
            assert!(s.live_abstich().unwrap().response.is_none());
            assert_eq!(
                s.execute(Command::RequestAdvice),
                Err(AppError::QuestionPending)
            );
            assert_eq!(answer(&mut s, id, given), response);
            assert!(s.live_abstich().unwrap().evidence.is_some());
            s.execute(Command::CloseLiveAbstich).unwrap();
            assert_eq!(s.export_current_recording_privileged().unwrap(), before);
            assert_eq!(s.view().sheet, sheet);
            assert_eq!(s.view().legal_actions, legal);
            for action in legal {
                s.clone().execute(Command::Act(action)).unwrap();
            }
        }
    }
}

#[test]
fn live_help_skip_unanswered_duplicates_and_roundtrip_remain_separate() {
    let mut s = fixture(1, false);
    let mut h = LearningHistory::new();
    let a = h.begin_attempt(AttemptKind::Series).unwrap();
    let id = select(&mut s, "CA");
    persist(&mut h, a, &s);
    let first = h.clone();
    s.execute(Command::CloseLiveAbstich).unwrap();
    assert_eq!(select(&mut s, "CA"), id);
    persist(&mut h, a, &s);
    assert_eq!(h, first);
    s.execute(Command::LiveAbstich(id, L::Help)).unwrap();
    s.execute(Command::LiveAbstich(id, L::Help)).unwrap();
    persist(&mut h, a, &s);
    s.execute(Command::CloseLiveAbstich).unwrap();
    let id = select(&mut s, "CA");
    assert!(s.live_abstich().unwrap().assisted);
    assert_eq!(answer(&mut s, id, A::Possible), R::Correct);
    persist(&mut h, a, &s);
    s.execute(Command::LiveAbstich(id, L::Answer(A::Absent)))
        .unwrap();
    persist(&mut h, a, &s);
    h = reload(&h);
    let rows = h.live_abstich_summary();
    assert_eq!(rows[1].1.presented, 1);
    assert_eq!(rows[1].1.answered, 1);
    assert_eq!(rows[1].1.correct, 1);
    assert_eq!(rows[0].1.presented, 0);
    s.execute(Command::SelectLiveAbstich(
        s.live_abstich().unwrap().boundary,
        c("SA"),
    ))
    .unwrap();
    let other = s.live_abstich().unwrap().id.unwrap();
    assert_ne!(other, id);
    s.execute(Command::LiveAbstich(other, L::Skip)).unwrap();
    assert!(s.live_abstich().unwrap().evidence.is_none());
    persist(&mut h, a, &s);
    s.execute(Command::SelectLiveAbstich(
        s.live_abstich().unwrap().boundary,
        c("ST"),
    ))
    .unwrap();
    persist(&mut h, a, &s);
    let rows = reload(&h).live_abstich_summary();
    assert_eq!(rows[0].1.skipped, 1);
    assert_eq!(rows[0].1.unanswered, 1);
    assert_eq!(rows[0].1.answered, 0);
    assert_eq!(h.events().len(), 0);
    assert_eq!(h.focus_entries().len(), 0);
    assert!(!h.is_empty());
    let practice = h
        .begin_attempt(AttemptKind::Practice { attempt: 1 })
        .unwrap();
    assert_eq!(
        h.record_live_abstich(practice, s.live_abstich_observations()),
        Err(crate::LearningError::InvalidContext)
    );
}

#[test]
fn live_empty_is_unsupported_never_absent_and_stale_commands_are_refused() {
    for (which, card) in [(4, "SA"), (2, "HA"), (1, "SJ")] {
        let mut s = fixture(which, false);
        let id = select(&mut s, card);
        assert!(s.live_abstich().unwrap().unsupported);
        assert_eq!(id.opponent, None);
        assert_eq!(
            s.execute(Command::LiveAbstich(id, L::Answer(A::Absent))),
            Err(AppError::StaleLiveAbstich)
        );
        let mut h = LearningHistory::new();
        let a = h.begin_attempt(AttemptKind::Series).unwrap();
        persist(&mut h, a, &s);
        let rows = reload(&h).live_abstich_summary();
        assert_eq!(rows[0].1.unsupported, 1);
        assert_eq!(rows[0].1.scored, 0);
        assert_eq!(rows[0].1.answered, 0);
        assert_eq!(rows[0].1.false_presence, 0);
        s.execute(Command::CloseLiveAbstich).unwrap();
        s.execute(Command::Act(s.view().legal_actions[0])).unwrap();
        assert_eq!(
            s.execute(Command::LiveAbstich(id, L::Help)),
            Err(AppError::StaleLiveAbstich)
        );
        assert_eq!(
            s.execute(Command::SelectLiveAbstich(
                s.live_abstich().map_or((0, 0, 0), |v| v.boundary),
                c(card)
            )),
            Err(AppError::StaleLiveAbstich)
        );
    }
}

#[test]
fn live_hidden_substitution_has_identical_prompt_evidence_and_category() {
    let mut x = fixture(3, false);
    let mut y = fixture(3, true);
    let xi = select(&mut x, "SA");
    let yi = select(&mut y, "SA");
    let mut xv = x.live_abstich().unwrap();
    let mut yv = y.live_abstich().unwrap();
    xv.id.as_mut().unwrap().lineage = 0;
    yv.id.as_mut().unwrap().lineage = 0;
    xv.boundary.0 = 0;
    yv.boundary.0 = 0;
    assert_eq!(xv, yv);
    assert_eq!(answer(&mut x, xi, A::Certain), R::OverstatedCertainty);
    assert_eq!(answer(&mut y, yi, A::Certain), R::OverstatedCertainty);
    assert_eq!(
        x.live_abstich().unwrap().evidence,
        y.live_abstich().unwrap().evidence
    );
}

#[test]
fn live_history_rejects_tampered_target_version_and_context_preserves_reset_counter() {
    let mut s = fixture(2, false);
    let id = select(&mut s, "SA");
    answer(&mut s, id, A::Certain);
    let mut h = LearningHistory::new();
    let a = h.begin_attempt(AttemptKind::Series).unwrap();
    persist(&mut h, a, &s);
    let value = serde_json::to_value(&h).unwrap();
    for (field, bad) in [
        ("version", serde_json::json!(9)),
        ("opponent", serde_json::json!("Hinterhand")),
    ] {
        let mut v = value.clone();
        v["live_abstich"][0]["observation"]["id"][field] = bad;
        assert!(LearningHistory::read_local(serde_json::to_vec(&v).unwrap().as_slice()).is_err());
    }
    let mut v = value.clone();
    v["live_abstich"][0]["observation"]["provenance"]["own_skat"] = serde_json::Value::Null;
    assert!(LearningHistory::read_local(serde_json::to_vec(&v).unwrap().as_slice()).is_err());
    // A stored expected answer cannot replace shared analysis.
    let mut v = value;
    v["live_abstich"][0]["observation"]["expected"] = serde_json::json!("Absent");
    assert!(LearningHistory::read_local(serde_json::to_vec(&v).unwrap().as_slice()).is_err());
    let mut h = reload(&h);
    h.reset();
    assert!(h.live_abstich_entries().is_empty());
    let b = reload(&h).begin_attempt(AttemptKind::Series).unwrap();
    assert!(b > a);
}

#[test]
fn live_excludes_open_god_practice_ouvert_null_other_phase_forced_and_final_seat() {
    for excluded in 0..7 {
        let mut s = fixture(1, false);
        match excluded {
            0 => s.config.display = TableDisplay::Open,
            1 => s.config.opponents = Opponents::GodMode,
            2 => s.practice = true,
            3 => {
                // Reach an actual legal Ouvert contract, before its first lead.
                let d = s.recording.deal;
                s = at_position(
                    d,
                    vec![
                        Action::Hand,
                        Action::Announce(Announcement {
                            game: GameType::Farbe(Suit::Herz),
                            schneider: true,
                            schwarz: true,
                            ouvert: true,
                        }),
                    ],
                    vec![],
                );
            }
            4 => {
                let d = s.recording.deal;
                s = at_position(
                    d,
                    vec![
                        Action::Hand,
                        Action::Announce(Announcement::plain(GameType::Null)),
                    ],
                    vec![],
                );
            }
            5 => s = Session::new(SessionConfig::default()),
            6 => {
                // Legal play until the human has only one legal action.
                while s.game.phase() == Phase::Spiel
                    && (s.human_decision().is_none() || s.game.legal_actions().len() != 1)
                {
                    s.apply(s.game.legal_actions()[0]).unwrap();
                }
                assert_eq!(s.game.legal_actions().len(), 1);
            }
            _ => unreachable!(),
        }
        s.execute(Command::StartLiveAbstich).unwrap();
        let v = s.live_abstich().unwrap();
        assert!(v.unsupported, "exclusion {excluded}");
        assert!(s.live_abstich_observations().is_empty());
        if let Some(&card) = v.candidates.first() {
            assert_eq!(
                s.execute(Command::SelectLiveAbstich(v.boundary, card)),
                Err(AppError::StaleLiveAbstich)
            );
        }
    }
    // A legal final-seat decision has no later target, even for a full card.
    let mut s = fixture(1, false);
    for card in ["H9", "DJ", "D9", "SK", "S8"] {
        s.apply(Action::Play(c(card))).unwrap();
    }
    assert_eq!(s.game.view(Seat::Vorhand).current_trick.len(), 2);
    s.execute(Command::StartLiveAbstich).unwrap();
    let v = s.live_abstich().unwrap();
    for card in v.candidates {
        s.execute(Command::SelectLiveAbstich(v.boundary, card))
            .unwrap();
        assert!(s.live_abstich().unwrap().unsupported);
    }
}

#[test]
fn live_stale_selection_target_attempt_and_boundary_never_collide() {
    let mut s = fixture(1, false);
    let id = select(&mut s, "CA");
    let boundary = s.live_abstich().unwrap().boundary;
    let mut wrong = id;
    wrong.opponent = Some(Seat::Hinterhand);
    assert_eq!(
        s.execute(Command::LiveAbstich(wrong, L::Help)),
        Err(AppError::StaleLiveAbstich)
    );
    assert_eq!(
        s.execute(Command::SelectLiveAbstich(boundary, c("D7"))),
        Err(AppError::StaleLiveAbstich)
    );
    let mut h = LearningHistory::new();
    let a = h.begin_attempt(AttemptKind::Series).unwrap();
    persist(&mut h, a, &s);
    let b = h.begin_attempt(AttemptKind::Series).unwrap();
    persist(&mut h, b, &s);
    assert_eq!(reload(&h).live_abstich_summary()[0].1.presented, 2);
    s.execute(Command::CloseLiveAbstich).unwrap();
    s.execute(Command::Act(Action::Play(c("CA")))).unwrap();
    s.run_opponents();
    if s.human_decision().is_some() {
        s.execute(Command::StartLiveAbstich).unwrap();
        let card = s.live_abstich().unwrap().candidates[0];
        assert_eq!(
            s.execute(Command::SelectLiveAbstich(boundary, card)),
            Err(AppError::StaleLiveAbstich)
        );
        assert_eq!(
            s.execute(Command::LiveAbstich(id, L::Help)),
            Err(AppError::StaleLiveAbstich)
        );
    }
}

#[test]
fn live_supported_history_versions_migrate_without_reordering_existing_records() {
    // Real historical event shape; formats1/2 have no provenance,3+ do.
    let legacy = serde_json::json!({"format":1,"next_attempt":2,"attempts":[{"id":1,"kind":"Series"}],"events":[
        {"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},
        {"attempt":1,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":{"Answer":{"given":4,"correct":false}},"assistance":"AnswerBeforeReveal","assisted":false}}
    ]});
    for version in 1..=5 {
        let mut original = legacy.clone();
        original["format"] = version.into();
        let mut h =
            LearningHistory::read_local(serde_json::to_vec(&original).unwrap().as_slice()).unwrap();
        let events = h.events().to_vec();
        let attempts = h.attempts().to_vec();
        let mut s = fixture(1, false);
        select(&mut s, "CA");
        let a = h.begin_attempt(AttemptKind::Series).unwrap();
        persist(&mut h, a, &s);
        let h = reload(&h);
        assert_eq!(h.events(), events);
        assert_eq!(&h.attempts()[..1], attempts);
        assert_eq!(h.live_abstich_summary()[0].1.unanswered, 1);
        assert_eq!(h.summary().totals.answers, 1);
    }
}

#[test]
fn live_late_coach_results_stay_withheld() {
    let mut s = fixture(1, false);
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 500,
    });
    s.execute(Command::RequestAdvice).unwrap();
    let job = s.coach_job().unwrap();
    let id = select(&mut s, "CA");
    // Settings remain available at any time; enabling an aid during the
    // question must not disclose it or introduce a second pending quiz.
    s.execute(Command::SetCoach(s.coach_settings())).unwrap();
    s.execute(Command::SetAssist(crate::AssistSettings {
        counting_aid: true,
        quiz: true,
        ..crate::AssistSettings::default()
    }))
    .unwrap();
    assert_eq!(s.view().assist.counting, crate::CountingState::Off);
    assert!(s.view().assist.quiz.question.is_none());
    let before = s.export_current_recording_privileged().unwrap();
    s.finish_coach(job.run(None)).unwrap();
    assert_eq!(s.view().coach, CoachView::withheld(s.coach_settings()));
    assert!(s.live_abstich().unwrap().evidence.is_none());
    assert_eq!(s.export_current_recording_privileged().unwrap(), before);
    assert_eq!(answer(&mut s, id, A::Possible), R::Correct);
}
