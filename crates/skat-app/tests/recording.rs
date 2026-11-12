// SPDX-License-Identifier: EUPL-1.2

//! M7 API integration: actual session actions, isolated legal replay and
//! historical information. Fixed fixtures pin choices, not current AI policy.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_app::*;
use skat_core::Game;

fn config() -> SessionConfig {
    SessionConfig {
        seed: 7,
        series: SeriesLength::Fixed(3),
        kontra_re: true,
        opponents: Opponents::Levels([Level::Easy; 2]),
    }
}

fn current(s: &Session) -> GameRecording {
    s.export_current_recording_privileged().unwrap()
}

fn fixture(name: &str) -> GameRecording {
    let json = match name {
        "herz" => include_str!("fixtures/recordings/herz.json"),
        "grand" => include_str!("fixtures/recordings/grand.json"),
        "null" => include_str!("fixtures/recordings/null.json"),
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(json).unwrap()
}

fn core_events(events: &[AppEvent]) -> Vec<Event> {
    events
        .iter()
        .filter_map(|e| match e {
            AppEvent::Game { event, .. } => Some(*event),
            _ => None,
        })
        .collect()
}

#[test]
fn session_captures_both_opponent_paths_once_and_keeps_game_boundaries() {
    let mut s = Session::new(config());
    let mut control = s.clone();
    let mut observer = Game::new(s.game_seed(), RuleConfig { kontra_re: true });
    let mut expected_events = Vec::new();
    let mut turn = 0;
    let mut deal_rng = ChaCha8Rng::seed_from_u64(config().seed);
    let mut opponent_rng = ChaCha8Rng::seed_from_u64(config().seed);
    opponent_rng.set_stream(1);
    opponent_rng.next_u64(); // Goldfish selection uses the first draw.
    let mut expected_seed = deal_rng.next_u64();
    let mut expected_opponent_seeds = [opponent_rng.next_u64(), opponent_rng.next_u64()];
    while !s.view().series_finished {
        let before = current(&s);
        assert_eq!(before.seed, expected_seed);
        assert_eq!(before.context.opponent_seeds, expected_opponent_seeds);
        assert_eq!(before.context.session, config());
        assert_eq!(
            before.rules,
            RuleConfig {
                kontra_re: config().kontra_re
            }
        );
        let view = s.view();
        let events = if s.opponent_to_move() {
            if turn % 2 == 0 {
                let mv = s.opponent_job().unwrap().run(None);
                let events = s.finish_opponent(mv).unwrap();
                control.execute(Command::AdvanceOpponent).unwrap();
                let after = current(&s);
                let expected = if s.view().series_finished {
                    AppError::SeriesFinished
                } else if s.opponent_to_move() {
                    AppError::StaleOpponentMove
                } else {
                    AppError::NoOpponentToMove
                };
                assert_eq!(s.finish_opponent(mv), Err(expected));
                assert_eq!(current(&s), after);
                events
            } else {
                control.execute(Command::AdvanceOpponent).unwrap();
                s.execute(Command::AdvanceOpponent).unwrap()
            }
        } else if let Some(&a) = view.legal_actions.first() {
            // Keep bidding low; all other actions come from the legal API.
            let a = if view.legal_actions.contains(&Action::Pass) {
                Action::Pass
            } else {
                a
            };
            control.execute(Command::Act(a)).unwrap();
            s.execute(Command::Act(a)).unwrap()
        } else {
            let archived = s.completed_recordings_privileged().to_vec();
            assert_eq!(archived.len(), view.sheet.records.len());
            assert_eq!(archived.last(), Some(&before));
            control.execute(Command::NextGame).unwrap();
            s.execute(Command::NextGame).unwrap();
            assert_eq!(s.completed_recordings_privileged(), archived);
            expected_seed = deal_rng.next_u64();
            expected_opponent_seeds = [opponent_rng.next_u64(), opponent_rng.next_u64()];
            let next = current(&s);
            assert_eq!(next.context.number, before.context.number + 1);
            assert_eq!(next.context.dealer, before.context.dealer.next());
            assert_ne!(next.seed, before.seed);
            assert_ne!(next.context.opponent_seeds, before.context.opponent_seeds);
            assert!(next.actions.is_empty());
            observer = Game::new(next.seed, next.rules);
            expected_events.clear();
            continue;
        };
        turn += 1;
        let after = current(&s);
        assert_eq!(after.actions.len(), before.actions.len() + 1);
        assert_eq!(&after.actions[..before.actions.len()], before.actions);
        let actual_events = observer.apply(*after.actions.last().unwrap()).unwrap();
        assert_eq!(core_events(&events), actual_events);
        expected_events.extend(actual_events);
        let reconstructed = after.reconstruct_privileged(after.actions.len()).unwrap();
        assert_eq!(reconstructed.state, observer.omniscient_view());
        assert_eq!(reconstructed.events, expected_events);
        for seat in Seat::ALL {
            assert_eq!(reconstructed.state.seats[seat.index()], observer.view(seat));
        }
        // Export/reconstruction must not consume session or AI RNG, alter
        // bookkeeping, or introduce recording data into the ordinary view.
        assert_eq!(s.view(), control.view());
        assert_eq!(current(&s), current(&control));
    }
    assert_eq!(s.completed_recordings_privileged().len(), 3);
    for (recording, score) in s
        .completed_recordings_privileged()
        .iter()
        .zip(&s.sheet().records)
    {
        assert_eq!(recording.context.number, score.number);
        assert_eq!(recording.context.dealer, score.dealer);
        let position = recording
            .reconstruct_privileged(recording.actions.len())
            .unwrap();
        assert_eq!(position.state.seats[0].result, score.result);
    }
}

#[test]
fn rejected_and_non_game_commands_never_become_actions() {
    let mut s = Session::new(config());
    let before = current(&s);
    for command in [
        Command::Act(Action::Pass),
        Command::NextGame,
        Command::ProceedAnyway,
        Command::SkipQuiz,
    ] {
        assert!(s.execute(command).is_err());
        assert_eq!(current(&s), before);
    }
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    s.execute(Command::SetAssist(AssistSettings::default()))
        .unwrap();
    assert_eq!(current(&s), before);
    s.run_opponents();
    let after = current(&s);
    assert!(!after.actions.is_empty());
    after.reconstruct_privileged(after.actions.len()).unwrap();
    let json = serde_json::to_value(s.view()).unwrap();
    for forbidden in [
        "recording",
        "recordings",
        "original_hands",
        "original_skat",
        "opponent_seeds",
    ] {
        assert!(!json.as_object().unwrap().contains_key(forbidden));
        assert!(
            !serde_json::to_string(&json)
                .unwrap()
                .contains(&format!("\"{forbidden}\""))
        );
    }
    assert!(s.view().open_cards.is_none());
    assert!(s.opponent_job().is_none_or(|job| !job.sees_all_cards()));
    let zero = Session::new(SessionConfig {
        series: SeriesLength::Fixed(0),
        ..config()
    });
    assert!(zero.export_current_recording_privileged().is_none());
    assert!(zero.completed_recordings_privileged().is_empty());
}

#[test]
fn all_pass_is_completed_without_a_result_or_card_play() {
    let mut r = current(&Session::new(config()));
    r.actions = vec![Action::Pass; 3];
    r.end = RecordingEnd::Eingepasst;
    let final_state = r.reconstruct_privileged(3).unwrap();
    assert_eq!(final_state.state.seats[0].phase, Phase::Eingepasst);
    assert!(final_state.state.seats[0].result.is_none());
    assert!(
        final_state
            .state
            .seats
            .iter()
            .all(|v| v.tricks.is_empty() && v.legal_actions.is_empty())
    );
    assert!(matches!(final_state.events.last(), Some(Event::Eingepasst)));
    assert!(
        r.reconstruct_privileged(0).unwrap().state.seats[0]
            .bidding
            .is_empty()
    );
}

#[test]
fn invalid_recordings_fail_atomically_even_when_requesting_an_early_prefix() {
    let mut s = Session::new(config());
    s.run_opponents();
    let before = (
        s.view(),
        current(&s),
        s.completed_recordings_privileged().to_vec(),
    );
    let valid = fixture("grand");
    let mut cases = Vec::new();
    let mut r = valid.clone();
    r.format += 1;
    cases.push((
        r,
        RecordingError::UnsupportedFormat {
            found: RECORDING_FORMAT + 1,
        },
    ));
    let mut r = valid.clone();
    r.engine.push('x');
    cases.push((r, RecordingError::UnsupportedEngine));
    let mut r = valid.clone();
    r.context.dealer = PlayerId(255);
    cases.push((r, RecordingError::InvalidContext));
    let mut r = valid.clone();
    r.context.number = 0;
    cases.push((r, RecordingError::InvalidContext));
    let mut r = valid.clone();
    r.context.goldfish = Some(PlayerId(0));
    cases.push((r, RecordingError::InvalidContext));
    let mut r = valid.clone();
    r.rules.kontra_re = !r.rules.kontra_re;
    cases.push((r, RecordingError::RulesMismatch));
    let mut r = valid.clone();
    r.seed += 1;
    cases.push((r, RecordingError::DealMismatch));
    let mut r = valid.clone();
    r.actions = vec![Action::Pass; MAX_RECORDING_ACTIONS + 1];
    cases.push((
        r,
        RecordingError::TooManyActions {
            count: MAX_RECORDING_ACTIONS + 1,
        },
    ));
    let mut r = valid.clone();
    r.actions.pop();
    cases.push((r, RecordingError::EndMismatch));
    let mut r = valid.clone();
    r.actions.push(Action::Pass);
    cases.push((
        r,
        RecordingError::ActionsAfterCompletion {
            index: valid.actions.len(),
        },
    ));
    let mut r = valid.clone();
    r.end = RecordingEnd::InProgress;
    cases.push((r, RecordingError::EndMismatch));
    let mut r = valid.clone();
    if let RecordingEnd::Finished(ref mut result) = r.end {
        result.score += 1;
    }
    cases.push((r, RecordingError::EndMismatch));
    for (recording, expected) in cases {
        assert_eq!(recording.reconstruct_privileged(0), Err(expected));
        assert_eq!(
            (
                s.view(),
                current(&s),
                s.completed_recordings_privileged().to_vec()
            ),
            before
        );
    }
    let mut illegal = valid.clone();
    illegal.actions[0] = Action::Re;
    assert!(matches!(
        illegal.reconstruct_privileged(0),
        Err(RecordingError::IllegalAction { index: 0, .. })
    ));
    assert_eq!(
        valid.reconstruct_privileged(valid.actions.len() + 1),
        Err(RecordingError::InvalidPrefix {
            requested: valid.actions.len() + 1,
            available: valid.actions.len()
        })
    );
    let json = serde_json::to_string(&valid).unwrap();
    assert!(serde_json::from_str::<GameRecording>(&json[..json.len() - 1]).is_err());
    let mut data = serde_json::to_value(&valid).unwrap();
    data["actions"] = serde_json::json!(vec![Action::Pass; MAX_RECORDING_ACTIONS + 1]);
    assert!(serde_json::from_value::<GameRecording>(data).is_err());
    let mut data = serde_json::to_value(&valid).unwrap();
    data["unexpected"] = true.into();
    assert!(serde_json::from_value::<GameRecording>(data).is_err());
}

fn cards(text: &str) -> CardSet {
    text.split_whitespace()
        .map(|s| {
            let suit = match s.as_bytes()[0] {
                b'C' => Suit::Kreuz,
                b'S' => Suit::Pik,
                b'H' => Suit::Herz,
                b'D' => Suit::Karo,
                _ => panic!("suit"),
            };
            let rank = match s.as_bytes()[1] {
                b'7' => Rank::Sieben,
                b'8' => Rank::Acht,
                b'9' => Rank::Neun,
                b'T' => Rank::Zehn,
                b'J' => Rank::Bube,
                b'Q' => Rank::Dame,
                b'K' => Rank::Koenig,
                b'A' => Rank::Ass,
                _ => panic!("rank"),
            };
            Card::new(suit, rank)
        })
        .collect()
}

#[test]
fn frozen_families_roundtrip_every_boundary_with_independent_expected_facts() {
    // Expected game values and eyes follow the explicitly documented cards
    // and winners in fixtures/recordings/README.md, not the saved result.
    for (name, family, value, score, eyes, tricks, winners) in [
        (
            "herz",
            GameType::Farbe(Suit::Herz),
            20,
            -40,
            45,
            10,
            vec![0, 1, 1, 0, 2, 0, 1, 2, 2, 0],
        ),
        (
            "grand",
            GameType::Grand,
            192,
            -1536,
            37,
            10,
            vec![1, 1, 0, 0, 0, 1, 2, 2, 2, 0],
        ),
        (
            "null",
            GameType::Null,
            59,
            -118,
            21,
            6,
            vec![1, 1, 1, 2, 2, 0],
        ),
    ] {
        let r = fixture(name);
        let encoded = serde_json::to_vec(&r).unwrap();
        let decoded: GameRecording = serde_json::from_slice(&encoded).unwrap();
        let mut core = Game::new(7, r.rules);
        let mut events = Vec::new();
        let mut expected = vec![(core.omniscient_view(), events.clone())];
        for &action in &r.actions {
            events.extend(core.apply(action).unwrap());
            expected.push((core.omniscient_view(), events.clone()));
        }
        // Walk backwards and forwards more than once; reconstruction must
        // validate the whole record while returning only that prefix's facts.
        for prefix in (0..=r.actions.len()).rev().chain(0..=r.actions.len()) {
            let position = decoded.reconstruct_privileged(prefix).unwrap();
            assert_eq!(position.applied_actions, prefix);
            assert_eq!(position.state, expected[prefix].0);
            assert_eq!(position.events, expected[prefix].1);
        }
        let position = decoded.reconstruct_privileged(r.actions.len()).unwrap();
        let view = &position.state.seats[0];
        let result = view.result.unwrap();
        assert_eq!(result.outcome.contract.game, family);
        assert_eq!(result.game_value, value);
        assert_eq!(result.score, score);
        assert_eq!(result.outcome.declarer_augen, eyes);
        assert_eq!(result.outcome.tricks_played, tricks);
        assert!(!result.won);
        assert_eq!(
            view.tricks
                .iter()
                .map(|t| t.winner.index())
                .collect::<Vec<_>>(),
            winners
        );
        assert!(view.legal_actions.is_empty());
        assert!(matches!(
            position.events.last(),
            Some(Event::GameFinished { .. })
        ));
    }
}

#[test]
fn historic_skat_and_ouvert_visibility_is_not_invented_from_the_final_deal() {
    let r = fixture("herz");
    let initial = r.reconstruct_privileged(0).unwrap();
    let vh = cards("C9 CT CJ CQ H9 HT HQ HK D9 DK");
    assert_eq!(initial.state.hands[0], vh);
    assert_eq!(initial.state.original_skat, cards("S9 DA"));
    for seat in &initial.state.seats {
        assert!(seat.own_skat.is_none());
        assert!(seat.declarer_cards.is_none());
        assert!(seat.bidding.is_empty());
        assert_eq!(seat.hand.len(), 10);
    }
    assert_eq!(
        initial.state.seats[1].legal_actions.last(),
        Some(&Action::Pass)
    );
    let before_pickup = r.reconstruct_privileged(3).unwrap();
    assert_eq!(
        before_pickup.state.seats[0].legal_actions,
        vec![Action::Hand, Action::TakeSkat]
    );
    let picked_up = r.reconstruct_privileged(4).unwrap();
    assert_eq!(picked_up.state.hands[0], vh | cards("S9 DA"));
    assert!(picked_up.state.skat.is_empty());
    assert!(picked_up.state.seats[0].own_skat.is_none());
    let discarded = r.reconstruct_privileged(5).unwrap();
    assert_eq!(discarded.state.seats[0].own_skat, Some(cards("C9 CT")));
    assert_eq!(
        discarded.state.seats[0].hand,
        (vh | cards("S9 DA")) - cards("C9 CT")
    );
    assert!(discarded.state.seats[1].own_skat.is_none());
    assert!(discarded.state.seats[2].own_skat.is_none());
    assert!(
        r.reconstruct_privileged(r.actions.len())
            .unwrap()
            .state
            .seats[1]
            .own_skat
            .is_none()
    );

    let r = fixture("grand");
    // Both defenders have bid/held enough to be eligible for Kontra. Cards
    // stay concealed through announcement and pending Re, before reveal.
    for prefix in [0, 6, 7, 8, 9] {
        let p = r.reconstruct_privileged(prefix).unwrap();
        assert!(
            p.state
                .seats
                .iter()
                .all(|v| v.declarer_cards.is_none() && v.own_skat.is_none())
        );
    }
    let announced = r.reconstruct_privileged(8).unwrap();
    assert_eq!(
        announced.state.seats[1].legal_actions,
        vec![Action::Kontra, Action::KeinKontra]
    );
    let kontra = r.reconstruct_privileged(9).unwrap();
    assert_eq!(
        kontra.state.seats[0].legal_actions,
        vec![Action::Re, Action::KeinRe]
    );
    assert_eq!(kontra.state.seats[0].kontra, KontraLevel::Kontra);
    let revealed = r.reconstruct_privileged(10).unwrap();
    assert_eq!(revealed.state.seats[0].kontra, KontraLevel::Re);
    assert!(
        revealed
            .state
            .seats
            .iter()
            .all(|v| v.declarer_cards == Some(vh))
    );
    assert!(revealed.state.seats.iter().all(|v| v.own_skat.is_none()));
    assert_eq!(
        revealed
            .events
            .iter()
            .filter(|e| matches!(e, Event::Revealed { .. }))
            .count(),
        1
    );

    let r = fixture("null");
    assert!(
        r.reconstruct_privileged(4).unwrap().state.seats[1]
            .declarer_cards
            .is_none()
    );
    assert_eq!(
        r.reconstruct_privileged(5).unwrap().state.seats[1].declarer_cards,
        Some(vh)
    );
    let end = r.reconstruct_privileged(r.actions.len()).unwrap();
    assert_eq!(end.state.hands.map(|h| h.len()), [4; 3]);
    assert_eq!(end.state.seats[0].tricks.len(), 6);
    assert_eq!(
        r.actions
            .iter()
            .filter(|a| matches!(a, Action::Play(_)))
            .count(),
        18
    );
}

#[test]
fn table_configuration_and_goldfish_selection_are_descriptive_provenance() {
    for opponents in [
        Opponents::Goldfish,
        Opponents::Mephisto,
        Opponents::GodMode,
        Opponents::Neural,
        Opponents::Experimental,
        Opponents::Levels([Level::Easy, Level::Strong]),
    ] {
        // No opponent or model is executed for export/reconstruction.
        let session = Session::new(SessionConfig {
            opponents,
            ..config()
        });
        let recording = current(&session);
        assert_eq!(recording.context.session.opponents, opponents);
        assert_eq!(recording.context.goldfish, session.goldfish());
        assert_eq!(
            recording.context.application_version,
            env!("CARGO_PKG_VERSION")
        );
        if opponents == Opponents::Goldfish {
            assert!(matches!(recording.context.goldfish, Some(PlayerId(1 | 2))));
        } else {
            assert!(recording.context.goldfish.is_none());
        }
        let position = recording.reconstruct_privileged(0).unwrap();
        // Even records from God Mode return genuinely permitted seat views
        // inside the separately privileged container.
        assert!(
            position
                .state
                .seats
                .iter()
                .all(|v| v.declarer_cards.is_none() && v.own_skat.is_none())
        );
    }
}
