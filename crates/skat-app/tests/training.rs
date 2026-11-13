// SPDX-License-Identifier: EUPL-1.2

//! Focused counting training (M8 foundation, D-71): the four fixtures
//! replay legally through the rule engine and their expected facts are
//! frozen here independently of `skat_analysis::counting::derive`; the
//! session flow (answer, help, skip, retry, next, topic) records the right
//! events; training attempts persist apart from series and practice in the
//! one learning history; format 1 files migrate without loss.

use skat_app::*;
use skat_core::Game;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn c(suit: Suit, rank: Rank) -> Card {
    Card::new(suit, rank)
}

fn cards(list: &[(Suit, Rank)]) -> CardSet {
    list.iter().map(|&(s, r)| c(s, r)).collect()
}

/// Expected facts of one fixture, written down from the deal and the
/// recorded actions by hand, not computed by production code.
struct Frozen {
    fixture: TrainingFixture,
    game: GameType,
    declarer: Seat,
    seat: Seat,
    completed_tricks: usize,
    trick_in_progress: usize,
    hand: CardSet,
    question: QuestionKind,
    expected: u8,
    /// Remaining Trumpf: (total, own, played, discarded, Skat unknown).
    trumpf: Option<(u8, u8, u8, u8, bool)>,
    /// Proven voids by seat (seat order).
    voids: Option<[bool; 3]>,
}

fn frozen() -> Vec<Frozen> {
    use Rank::*;
    use Suit::*;
    vec![
        // Karo game, Hinterhand declares after picking up the Skat and
        // discarding Herz 8 and 9. Trumpf: 4 Buben + 7 Karo = 11. Own: Karo 9,
        // Karo 10, Karo Bube = 3. Played: Pik Bube, Kreuz Bube, Karo 8 (trick
        // 2), Karo 7, Dame, König (trick 3) = 6. Discard: no Trumpf.
        // Outside: 11 - 3 - 6 - 0 = 2. The Skat is the own discard: known.
        Frozen {
            fixture: TrainingFixture::TrumpfDeclarerFarbspiel,
            game: GameType::Farbe(Karo),
            declarer: Seat::Hinterhand,
            seat: Seat::Hinterhand,
            completed_tricks: 3,
            trick_in_progress: 0,
            hand: cards(&[
                (Kreuz, Dame),
                (Kreuz, Koenig),
                (Kreuz, Ass),
                (Pik, Ass),
                (Karo, Neun),
                (Karo, Zehn),
                (Karo, Bube),
            ]),
            question: QuestionKind::TrumpfOutside,
            expected: 2,
            trumpf: Some((11, 3, 6, 0, false)),
            voids: None,
        },
        // Grand, Hinterhand declares; Vorhand defends. Trumpf: 4 Buben. Own: 0.
        // Played: Kreuz Bube (trick 2), Pik Bube (trick 2), Herz Bube (led in
        // the trick on the table) = 3. Outside: 4 - 0 - 3 = 1, and it may lie
        // in the unseen Skat.
        Frozen {
            fixture: TrainingFixture::TrumpfDefenderGrand,
            game: GameType::Grand,
            declarer: Seat::Hinterhand,
            seat: Seat::Vorhand,
            completed_tricks: 2,
            trick_in_progress: 1,
            hand: cards(&[
                (Kreuz, Koenig),
                (Pik, Sieben),
                (Pik, Neun),
                (Pik, Ass),
                (Herz, Neun),
                (Herz, Ass),
                (Karo, Acht),
                (Karo, Koenig),
            ]),
            question: QuestionKind::TrumpfOutside,
            expected: 1,
            trumpf: Some((4, 0, 3, 0, true)),
            voids: None,
        },
        // Grand, Vorhand declares. Trick 1: Kreuz Bube led, Mittelhand plays
        // Herz König (no Trumpf). Trick 2: Herz Bube led, Mittelhand Karo 9,
        // Hinterhand Pik 7 (no Trumpf). Both defenders: proven Trumpf void.
        Frozen {
            fixture: TrainingFixture::VoidsDeclarerGrand,
            game: GameType::Grand,
            declarer: Seat::Vorhand,
            seat: Seat::Vorhand,
            completed_tricks: 2,
            trick_in_progress: 0,
            hand: cards(&[
                (Kreuz, Koenig),
                (Pik, Dame),
                (Pik, Koenig),
                (Herz, Sieben),
                (Herz, Dame),
                (Karo, Acht),
                (Karo, Zehn),
                (Karo, Bube),
            ]),
            question: QuestionKind::ProvenVoids {
                class: Follow::Trumpf,
            },
            expected: 2,
            trumpf: None,
            voids: Some([false, true, true]),
        },
        // Kreuz game, Mittelhand declares. Trick 1: Herz led, all follow.
        // Trick 2: Vorhand leads Herz Dame, Mittelhand plays Karo 7 (no Herz,
        // not Trumpf in Kreuz). Hinterhand to play: exactly Mittelhand is
        // proven Herz void.
        Frozen {
            fixture: TrainingFixture::VoidsDefenderFarbspiel,
            game: GameType::Farbe(Kreuz),
            declarer: Seat::Mittelhand,
            seat: Seat::Hinterhand,
            completed_tricks: 1,
            trick_in_progress: 2,
            hand: cards(&[
                (Kreuz, Bube),
                (Pik, Sieben),
                (Pik, Acht),
                (Pik, Zehn),
                (Pik, Dame),
                (Herz, Sieben),
                (Karo, Neun),
                (Karo, Zehn),
                (Karo, Dame),
            ]),
            question: QuestionKind::ProvenVoids {
                class: Follow::Farbe(Herz),
            },
            expected: 1,
            trumpf: None,
            voids: Some([false, true, false]),
        },
    ]
}

/// Replays a fixture with the rule engine, validating every action first.
fn replay(fixture: TrainingFixture, actions: &[Action]) -> Game {
    let mut game = Game::new(fixture.seed(), RuleConfig::default());
    for (i, &action) in actions.iter().enumerate() {
        assert!(
            game.legal_actions().contains(&action),
            "{fixture:?} action {i} {action:?} not legal"
        );
        game.apply(action).unwrap();
    }
    game
}

#[test]
fn every_fixture_replays_legally_and_matches_its_frozen_facts() {
    for f in frozen() {
        let game = replay(f.fixture, &f.fixture.actions());
        let view = game.view(f.seat);
        assert_eq!(view.phase, Phase::Spiel, "{:?}", f.fixture);
        assert_eq!(view.current_seat, Some(f.seat), "{:?}", f.fixture);
        assert_eq!(view.declarer, Some(f.declarer), "{:?}", f.fixture);
        assert_eq!(view.contract.unwrap().game, f.game, "{:?}", f.fixture);
        assert_eq!(view.tricks.len(), f.completed_tricks, "{:?}", f.fixture);
        assert_eq!(view.current_trick.len(), f.trick_in_progress);
        assert_eq!(view.hand, f.hand, "{:?}", f.fixture);
        assert_eq!(f.fixture.question(), f.question);
        assert_eq!(f.fixture.seat(), f.seat);
        assert_eq!(f.fixture.version(), 1);

        // Independent oracles over the full deal (test only): Trumpf
        // outside lies in the other hands, plus the Skat if unseen; voids
        // follow from the public tricks.
        let all = game.omniscient_view();
        if let Some((total, own, played, discarded, unknown)) = f.trumpf {
            let trumpf = f.game.cards_of(Follow::Trumpf);
            let mut hidden = CardSet::EMPTY;
            for s in Seat::ALL.into_iter().filter(|&s| s != f.seat) {
                hidden |= all.hands[s.index()];
            }
            if unknown {
                hidden |= all.skat;
            }
            assert_eq!((hidden & trumpf).len() as u8, f.expected, "{:?}", f.fixture);
            assert_eq!(total - own - played - discarded, f.expected);
            assert_eq!((view.hand & trumpf).len() as u8, own);
            assert_eq!(view.own_skat.is_none(), unknown);
        }
        if let Some(voids) = f.voids {
            let QuestionKind::ProvenVoids { class } = f.question else {
                panic!("void fixture asks for voids");
            };
            let mut proven = [false; 3];
            let mut tricks: Vec<Vec<(Seat, Card)>> = view
                .tricks
                .iter()
                .map(|t| (0..3).map(|i| (t.seat_of(i), t.cards[i])).collect())
                .collect();
            let leader = view.current_trick.leader.unwrap();
            tricks.push(
                view.current_trick
                    .cards
                    .iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, &card)| (Seat::from_index(leader.index() + i), card))
                    .collect(),
            );
            for trick in tricks.iter().filter(|t| !t.is_empty()) {
                if f.game.follow(trick[0].1) != class {
                    continue;
                }
                for &(seat, card) in &trick[1..] {
                    if f.game.follow(card) != class {
                        proven[seat.index()] = true;
                    }
                }
            }
            assert_eq!(proven, voids, "{:?}", f.fixture);
            assert_eq!(voids.iter().filter(|v| **v).count() as u8, f.expected);
        }

        // Production derivation and session agree with the frozen facts.
        let mut s = TrainingSession::new(f.fixture.topic(), AssistanceLevel::AnswerBeforeReveal);
        while s.view().task.fixture != f.fixture {
            s.execute(TrainingCommand::Skip).unwrap();
            s.execute(TrainingCommand::Next).unwrap();
        }
        let v = s.view();
        assert_eq!(v.seat, f.seat);
        assert_eq!(v.declarer, f.declarer);
        assert_eq!(v.contract.game, f.game);
        assert_eq!(v.hand.iter().copied().collect::<CardSet>(), f.hand);
        assert_eq!(v.tricks.len(), f.completed_tricks);
        assert_eq!(v.current_trick.len(), f.trick_in_progress);
        assert_eq!(v.question, f.question);
        assert_eq!(v.help, None, "no help before a request");
        s.execute(TrainingCommand::Answer(f.expected)).unwrap();
        let feedback = s.view().feedback.unwrap();
        assert_eq!(feedback.outcome, QuizOutcome::Correct);
        assert_eq!(feedback.derivation.expected(), f.expected);
        match (feedback.derivation, f.trumpf, f.voids) {
            (Derivation::TrumpfOutside(t), Some((total, own, played, discarded, unknown)), _) => {
                assert_eq!(
                    (t.total, t.own, t.played, t.discarded, t.skat_unknown),
                    (total, own, played, discarded, unknown)
                );
            }
            (Derivation::ProvenVoids { seats, .. }, _, Some(voids)) => assert_eq!(seats, voids),
            other => panic!("unexpected derivation {other:?}"),
        }
    }
}

#[test]
fn fixtures_cover_both_families_both_roles_farbspiel_and_grand_on_distinct_deals() {
    let all = frozen();
    let mut seeds: Vec<u64> = all.iter().map(|f| f.fixture.seed()).collect();
    seeds.sort_unstable();
    seeds.dedup();
    assert_eq!(seeds.len(), all.len(), "each fixture its own deal");
    for topic in TrainingTopic::ALL {
        let of: Vec<&Frozen> = all.iter().filter(|f| f.fixture.topic() == topic).collect();
        assert!(
            of.iter().any(|f| f.seat == f.declarer),
            "{topic:?} declarer"
        );
        assert!(
            of.iter().any(|f| f.seat != f.declarer),
            "{topic:?} defender"
        );
        assert!(of.iter().any(|f| f.game == GameType::Grand), "{topic:?}");
        assert!(of.iter().any(|f| matches!(f.game, GameType::Farbe(_))));
        assert_eq!(
            topic.fixtures(),
            of.iter().map(|f| f.fixture).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        TrainingFixture::ALL.to_vec(),
        all.iter().map(|f| f.fixture).collect::<Vec<_>>()
    );
}

#[test]
fn the_void_boundary_needs_the_failure_to_follow_and_null_has_no_trumpf_question() {
    let fixture = TrainingFixture::VoidsDefenderFarbspiel;
    let actions = fixture.actions();
    let herz = QuestionKind::ProvenVoids {
        class: Follow::Farbe(Suit::Herz),
    };
    // One card earlier: Mittelhand has not yet failed to follow Herz.
    let before = replay(fixture, &actions[..actions.len() - 1]).view(fixture.seat());
    let k = public_evidence(&before);
    assert!(!k.0.contains(&herz), "no void question without evidence");
    assert_eq!(k.1, 0);
    // After Karo 7 on Herz Dame: exactly one proven void.
    let after = replay(fixture, &actions).view(fixture.seat());
    let k = public_evidence(&after);
    assert!(k.0.contains(&herz));
    assert_eq!(k.1, 1);

    // Null: the Trumpf question is not applicable, so no Null fixture.
    let mut null = Game::new(2, RuleConfig::default());
    for a in [Action::Pass, Action::Pass, Action::Bid(18), Action::Hand] {
        null.apply(a).unwrap();
    }
    null.apply(Action::Announce(Announcement::plain(GameType::Null)))
        .unwrap();
    let first = null.legal_actions()[0];
    null.apply(first).unwrap();
    let seat = null.current_seat().unwrap();
    let view = null.view(seat);
    let (kinds, _) = public_evidence(&view);
    assert!(!kinds.contains(&QuestionKind::TrumpfOutside));
    assert!(
        frozen().iter().all(|f| f.game != GameType::Null),
        "Null is excluded honestly, not given a Trumpf task"
    );
}

/// Focused-retrieval questions of a view and its Herz void count, from the
/// rule engine's public void proof and the documented evidence rule
/// (independent of `skat_analysis::counting`).
fn public_evidence(view: &PlayerView) -> (Vec<QuestionKind>, u8) {
    let voids = view.proven_voids();
    let game = view.contract.unwrap().game;
    let mut kinds = Vec::new();
    if game != GameType::Null {
        kinds.push(QuestionKind::TrumpfOutside);
    }
    let mut herz = 0;
    for class in std::iter::once(Follow::Trumpf).chain(Suit::ALL.map(Follow::Farbe)) {
        let set = game.cards_of(class);
        if set.is_empty() {
            continue;
        }
        let n = Seat::ALL
            .into_iter()
            .filter(|&s| s != view.seat && (set - voids[s.index()]).is_empty())
            .count() as u8;
        if class == Follow::Farbe(Suit::Herz) {
            herz = n;
        }
        if n > 0 {
            kinds.push(QuestionKind::ProvenVoids { class });
        }
    }
    (kinds, herz)
}

fn actions(events: &[RetrievalEvent]) -> Vec<RetrievalAction> {
    events.iter().map(|e| e.action).collect()
}

#[test]
fn answer_help_skip_retry_next_and_topic_follow_the_contract() {
    let mut s = TrainingSession::new(
        TrainingTopic::TrumpfOutside,
        AssistanceLevel::AnswerBeforeReveal,
    );
    let first = s.view();
    assert_eq!(first.task.fixture, TrainingFixture::TrumpfDeclarerFarbspiel);
    assert_eq!((first.position, first.presentation), ((1, 2), 1));
    assert_eq!(s.attempt_kind(), AttemptKind::Training { attempt: 1 });
    assert_eq!(actions(&s.take_events()), [RetrievalAction::Opportunity]);
    // Retry and next need a settled task; refusals change nothing.
    assert_eq!(
        s.execute(TrainingCommand::Retry),
        Err(TrainingError::NotSettled)
    );
    assert_eq!(
        s.execute(TrainingCommand::Next),
        Err(TrainingError::NotSettled)
    );
    assert_eq!(s.view(), first);
    // Help is recorded once and stays: switching support back does not
    // make the presentation unaided.
    s.execute(TrainingCommand::Help).unwrap();
    s.execute(TrainingCommand::Help).unwrap();
    assert_eq!(s.view().help.unwrap().expected(), 2);
    s.execute(TrainingCommand::Assistance(
        AssistanceLevel::RevealOnRequest,
    ))
    .unwrap();
    assert_eq!(s.view().help, None);
    s.execute(TrainingCommand::Answer(3)).unwrap();
    let events = s.take_events();
    assert_eq!(
        actions(&events),
        [
            RetrievalAction::Help,
            RetrievalAction::Answer {
                given: 3,
                correct: false
            }
        ]
    );
    assert!(events[1].assisted);
    assert_eq!(events[1].assistance, AssistanceLevel::RevealOnRequest);
    let v = s.view();
    assert_eq!(
        v.feedback.unwrap().outcome,
        QuizOutcome::Incorrect { given: 3 }
    );
    assert_eq!(v.help, None, "feedback replaces help");
    assert_eq!(
        s.execute(TrainingCommand::Answer(2)),
        Err(TrainingError::Settled)
    );
    assert_eq!(
        s.execute(TrainingCommand::Skip),
        Err(TrainingError::Settled)
    );
    assert_eq!(
        s.execute(TrainingCommand::Help),
        Err(TrainingError::Settled)
    );

    // Retry: the same frozen task, a new presentation, unaided again.
    s.execute(TrainingCommand::Retry).unwrap();
    let retry = s.view();
    assert_eq!(retry.task, first.task);
    assert_eq!(retry.presentation, 2);
    assert_eq!(
        (retry.hand.clone(), retry.question),
        (first.hand, first.question)
    );
    assert_eq!(s.attempt_kind(), AttemptKind::Training { attempt: 2 });
    s.execute(TrainingCommand::Answer(2)).unwrap();
    let events = s.take_events();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| !e.assisted));
    assert_eq!(s.view().feedback.unwrap().outcome, QuizOutcome::Correct);

    // Next: the other legal position of the topic, then back (a repeat).
    s.execute(TrainingCommand::Next).unwrap();
    assert_eq!(s.view().task.fixture, TrainingFixture::TrumpfDefenderGrand);
    assert_eq!(s.view().position, (2, 2));
    assert_eq!(s.presentation(), (TrainingFixture::TrumpfDefenderGrand, 1));
    s.execute(TrainingCommand::Skip).unwrap();
    let skipped = s.view().feedback.unwrap();
    assert_eq!(skipped.outcome, QuizOutcome::Skipped);
    assert_eq!(skipped.derivation.expected(), 1);
    s.execute(TrainingCommand::Next).unwrap();
    assert_eq!(
        s.presentation(),
        (TrainingFixture::TrumpfDeclarerFarbspiel, 3)
    );
    assert_eq!(s.attempt_kind(), AttemptKind::Training { attempt: 3 });

    // Visible aid starts assisted without a separate help event.
    s.execute(TrainingCommand::Assistance(AssistanceLevel::VisibleAid))
        .unwrap();
    s.take_events();
    s.execute(TrainingCommand::Topic(TrainingTopic::ProvenVoids))
        .unwrap();
    let v = s.view();
    assert_eq!(v.task.fixture, TrainingFixture::VoidsDeclarerGrand);
    assert!(v.assisted && v.help.is_some());
    let events = s.take_events();
    assert_eq!(actions(&events), [RetrievalAction::Opportunity]);
    assert!(events[0].assisted);
    assert_eq!(
        events[0].id,
        OpportunityId::Training(TrainingTask {
            fixture: TrainingFixture::VoidsDeclarerGrand,
            version: 1
        })
    );
}

#[test]
fn the_same_commands_give_the_same_tasks_views_and_events() {
    let commands = [
        TrainingCommand::Help,
        TrainingCommand::Answer(1),
        TrainingCommand::Next,
        TrainingCommand::Skip,
        TrainingCommand::Retry,
        TrainingCommand::Answer(1),
        TrainingCommand::Topic(TrainingTopic::TrumpfOutside),
        TrainingCommand::Answer(2),
    ];
    let run = || {
        let mut s =
            TrainingSession::new(TrainingTopic::ProvenVoids, AssistanceLevel::RevealOnRequest);
        let mut out = vec![(s.view(), s.take_events())];
        for command in commands {
            s.execute(command).unwrap();
            out.push((s.view(), s.take_events()));
        }
        out
    };
    assert_eq!(run(), run());
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "skat-training-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}

fn game_event(trick: u8, action: RetrievalAction) -> RetrievalEvent {
    RetrievalEvent {
        id: QuestionId {
            series: 2,
            game: 1,
            trick,
        }
        .into(),
        question: QuestionKind::TrumpfOutside,
        action,
        assistance: AssistanceLevel::AnswerBeforeReveal,
        assisted: false,
    }
}

/// Records a session's events, one attempt per presentation, as the GUI
/// does.
fn record_training(
    history: &mut LearningHistory,
    s: &mut TrainingSession,
    attempt: &mut Option<(AttemptId, (TrainingFixture, u32))>,
) {
    let events = s.take_events();
    if events.is_empty() {
        return;
    }
    let current = s.presentation();
    let id = match *attempt {
        Some((id, p)) if p == current => id,
        _ => {
            let id = history.begin_attempt(s.attempt_kind()).unwrap();
            *attempt = Some((id, current));
            id
        }
    };
    history.record(id, &events).unwrap();
}

#[test]
fn training_attempts_persist_apart_from_series_and_practice_without_duplicates() {
    let mut history = LearningHistory::new();
    let series = history.begin_attempt(AttemptKind::Series).unwrap();
    history
        .record(series, &[game_event(3, RetrievalAction::Opportunity)])
        .unwrap();
    let practice = history
        .begin_attempt(AttemptKind::Practice { attempt: 1 })
        .unwrap();
    history
        .record(practice, &[game_event(3, RetrievalAction::Opportunity)])
        .unwrap();

    let mut s = TrainingSession::new(
        TrainingTopic::TrumpfOutside,
        AssistanceLevel::AnswerBeforeReveal,
    );
    let mut attempt = None;
    record_training(&mut history, &mut s, &mut attempt);
    s.execute(TrainingCommand::Answer(2)).unwrap();
    record_training(&mut history, &mut s, &mut attempt);
    s.execute(TrainingCommand::Retry).unwrap();
    record_training(&mut history, &mut s, &mut attempt);
    s.execute(TrainingCommand::Help).unwrap();
    s.execute(TrainingCommand::Answer(2)).unwrap();
    record_training(&mut history, &mut s, &mut attempt);

    // A training task cannot be recorded under a series attempt, nor a
    // game question under a training attempt.
    let training_event = history.events().last().unwrap().event;
    assert_eq!(
        history.record(series, &[training_event]),
        Err(LearningError::InvalidContext)
    );
    let (training_id, _) = attempt.unwrap();
    assert_eq!(
        history.record(training_id, &[game_event(6, RetrievalAction::Opportunity)]),
        Err(LearningError::InvalidContext)
    );
    // Re-sent events are duplicates.
    let resent: Vec<_> = history
        .events()
        .iter()
        .filter(|e| e.attempt == training_id)
        .map(|e| e.event)
        .collect();
    assert_eq!(
        history.record(training_id, &resent).unwrap(),
        Recorded {
            added: 0,
            duplicates: 3
        }
    );

    let dir = temp_dir();
    let path = dir.join("learning-history.json");
    history.save_local(&path).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["format"], 2);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("TrumpfDeclarerFarbspiel"), "{text}");
    let reopened = LearningHistory::open_local(&path).unwrap();
    assert_eq!(reopened, history);
    let summary = reopened.summary();
    let row = |context| {
        summary
            .rows
            .iter()
            .find(|r| r.topic == LearningTopic::TrumpfOutside && r.context == context)
            .unwrap()
            .counts
    };
    assert_eq!(row(LearningContext::Series).opportunities, 1);
    assert_eq!(row(LearningContext::Practice).opportunities, 1);
    let first = row(LearningContext::Training);
    assert_eq!(
        (first.opportunities, first.answers, first.unaided_correct),
        (1, 1, 1)
    );
    let repeat = row(LearningContext::TrainingRepeat);
    assert_eq!(
        (
            repeat.opportunities,
            repeat.help,
            repeat.correct,
            repeat.unaided_answers
        ),
        (1, 1, 1, 0),
        "an assisted retry never counts as unaided"
    );
    assert_eq!(summary.attempts, 4);
    // The history holds no cards: no hand, no Skat, no seed of a fixture.
    for card in ["Kreuz", "Herz\"", "Bube", "seed"] {
        assert!(!text.contains(card), "{card} in {text}");
    }

    // Reset forgets everything, keeps the counter, and saves format 2.
    let mut reset = reopened;
    reset.reset();
    reset.save_local(&path).unwrap();
    let after = LearningHistory::open_local(&path).unwrap();
    assert!(after.is_empty());
    let mut after = after;
    assert_eq!(
        after.begin_attempt(AttemptKind::Training { attempt: 1 }),
        Ok(AttemptId(5))
    );
    fs::remove_dir_all(dir).unwrap();
}

/// A format 1 file exactly as the previous build wrote it (PR #133).
const FORMAT_1: &str = r#"{"format":1,"next_attempt":4,"attempts":[{"id":2,"kind":"Series"},{"id":3,"kind":{"Practice":{"attempt":2}}}],"events":[{"attempt":2,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":"Opportunity","assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":2,"event":{"id":{"series":7,"game":1,"trick":3},"question":"TrumpfOutside","action":{"Answer":{"given":2,"correct":true}},"assistance":"AnswerBeforeReveal","assisted":false}},{"attempt":3,"event":{"id":{"series":4,"game":1,"trick":6},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":"Help","assistance":"RevealOnRequest","assisted":true}},{"attempt":3,"event":{"id":{"series":4,"game":1,"trick":6},"question":{"ProvenVoids":{"class":{"Farbe":"Herz"}}},"action":{"Skip":{"disabled":false}},"assistance":"RevealOnRequest","assisted":true}}]}"#;

#[test]
fn a_format_1_file_migrates_without_loss_and_damaged_or_newer_files_stay_unchanged() {
    let dir = temp_dir();
    let path = dir.join("learning-history.json");
    fs::write(&path, FORMAT_1).unwrap();
    let mut migrated = LearningHistory::open_local(&path).unwrap();
    // Reading never rewrites the file.
    assert_eq!(fs::read_to_string(&path).unwrap(), FORMAT_1);
    let attempts: Vec<_> = migrated.attempts().to_vec();
    assert_eq!(
        attempts,
        [
            LearningAttempt {
                id: AttemptId(2),
                kind: AttemptKind::Series
            },
            LearningAttempt {
                id: AttemptId(3),
                kind: AttemptKind::Practice { attempt: 2 }
            }
        ]
    );
    assert_eq!(migrated.events().len(), 4);
    assert_eq!(
        migrated.events()[0].event.id,
        OpportunityId::Game(QuestionId {
            series: 7,
            game: 1,
            trick: 3
        })
    );
    let summary = migrated.summary();
    assert_eq!(summary.totals.opportunities, 1);
    assert_eq!(summary.totals.correct, 1);
    assert_eq!(summary.totals.help, 1);
    assert_eq!(summary.totals.skips, 1);
    // Every known event is still a duplicate after migration.
    let events: Vec<_> = migrated
        .events()
        .iter()
        .filter(|e| e.attempt == AttemptId(2))
        .map(|e| e.event)
        .collect();
    assert_eq!(
        migrated.record(AttemptId(2), &events).unwrap().duplicates,
        2
    );
    // The counter survives: the next identity is 4.
    let next = migrated
        .begin_attempt(AttemptKind::Training { attempt: 1 })
        .unwrap();
    assert_eq!(next, AttemptId(4));
    // Saving writes format 2 with the old events unchanged in shape.
    migrated.save_local(&path).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let old: serde_json::Value = serde_json::from_str(FORMAT_1).unwrap();
    assert_eq!(saved["format"], 2);
    assert_eq!(saved["events"], old["events"]);
    assert_eq!(saved["next_attempt"], 5);
    assert_eq!(LearningHistory::open_local(&path).unwrap(), migrated);

    // Format 1 cannot hold training content; format 3 is from a newer
    // build. Both are reported, never rewritten, and can be set aside.
    let mut s = TrainingSession::new(TrainingTopic::ProvenVoids, AssistanceLevel::VisibleAid);
    let training = s.take_events()[0];
    let mut with_training: serde_json::Value = serde_json::from_str(FORMAT_1).unwrap();
    with_training["events"][0]["event"] = serde_json::to_value(training).unwrap();
    let mut training_kind: serde_json::Value = serde_json::from_str(FORMAT_1).unwrap();
    training_kind["attempts"][0]["kind"] =
        serde_json::to_value(AttemptKind::Training { attempt: 1 }).unwrap();
    let mut newer: serde_json::Value = serde_json::from_str(FORMAT_1).unwrap();
    newer["format"] = 3.into();
    for (n, (value, error)) in [
        (
            with_training,
            LearningFileError::Invalid(LearningError::InvalidContext),
        ),
        (
            training_kind,
            LearningFileError::Invalid(LearningError::InvalidContext),
        ),
        (newer, LearningFileError::UnsupportedFormat),
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = serde_json::to_vec(&value).unwrap();
        let p = dir.join(format!("case-{n}.json"));
        fs::write(&p, &bytes).unwrap();
        assert_eq!(LearningHistory::open_local(&p), Err(error), "case {n}");
        assert_eq!(fs::read(&p).unwrap(), bytes);
        let aside = LearningHistory::set_aside(&p).unwrap();
        assert_eq!(fs::read(aside).unwrap(), bytes);
    }
    fs::remove_dir_all(dir).unwrap();
}
