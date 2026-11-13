// SPDX-License-Identifier: EUPL-1.2

//! M7 same-deal practice: a validated, isolated one-game attempt of a
//! recorded deal that never touches the live series, its score sheet,
//! recordings or files, and rejects results from other sessions.

use skat_app::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn config(opponents: Opponents) -> SessionConfig {
    SessionConfig {
        seed: 41,
        series: SeriesLength::Fixed(6),
        kontra_re: true,
        opponents,
        ..SessionConfig::default()
    }
}

/// Legal human choice that keeps games short: pass if possible.
fn low(view: &TableView) -> Action {
    if view.legal_actions.contains(&Action::Pass) {
        Action::Pass
    } else {
        view.legal_actions[0]
    }
}

/// Plays the current game to its end; returns the number of applied actions.
fn count(s: &mut Session, mut human: impl FnMut(&TableView) -> Action) -> usize {
    let mut n = 0;
    loop {
        if s.opponent_to_move() {
            s.execute(Command::AdvanceOpponent).unwrap();
        } else {
            let view = s.view();
            if view.legal_actions.is_empty() {
                return n;
            }
            s.execute(Command::Act(human(&view))).unwrap();
        }
        n += 1;
    }
}

/// Live series advanced to its third game (dealer: player 1); returns the
/// completed recording of game 2 (dealer: player 0, human Hinterhand).
fn late_series(opponents: Opponents) -> (Session, GameRecording) {
    let mut live = Session::new(config(opponents));
    for _ in 0..2 {
        count(&mut live, low);
        live.execute(Command::NextGame).unwrap();
    }
    let recording = live.completed_recordings_privileged()[1].clone();
    assert_eq!(recording.context.number, 2);
    assert_eq!(recording.context.dealer, PlayerId(0));
    // Covers bidding, Skat and card play (if the Easy policy changes, pick another seed).
    assert!(matches!(recording.end, RecordingEnd::Finished(_)));
    (live, recording)
}

/// A valid untrusted late-series recording before bidding, built from its
/// public fields only (no opponent is run).
fn synthetic(opponents: Opponents, number: u32, goldfish: Option<PlayerId>) -> GameRecording {
    let mut r = Session::new(config(opponents))
        .export_current_recording_privileged()
        .unwrap();
    r.context.number = number;
    r.context.dealer = PlayerId(((number + 1) % 3) as u8);
    r.context.opponent_seeds = [0xDEAD_BEEF, 0x5EED];
    r.context.goldfish = goldfish;
    r.seed = 0x1234_5678_9ABC;
    r.deal = Deal::from_seed(r.seed);
    r
}

fn hand_of(deal: &Deal, player: PlayerId, dealer: PlayerId) -> CardSet {
    deal.hand(player.seat(dealer))
}

#[test]
fn late_series_attempt_has_the_exact_deal_configuration_seats_and_opponents() {
    let (live, recording) = late_series(Opponents::Levels([Level::Easy, Level::Rookie]));
    let live_view = live.view();
    let practice = live.start_practice(&recording).unwrap();
    let view = practice.view();
    assert!(practice.is_practice() && view.practice && !live_view.practice);
    assert_eq!(view.config, recording.context.session);
    assert_eq!(practice.config().seed, 41, "series seed, not the game seed");
    assert_eq!(practice.game_seed(), recording.seed);
    assert_eq!(view.game_number, 2);
    assert_eq!(view.dealer, PlayerId(0));
    assert_eq!(view.players[0].seat, Seat::Hinterhand);
    assert_eq!(view.players[1].seat, Seat::Vorhand);
    assert_eq!(view.players[2].seat, Seat::Mittelhand);
    assert_eq!(view.phase, Phase::Reizen, "before bidding");
    assert!(view.bidding.is_empty() && view.tricks_played == 0);
    let hand: CardSet = view.hand.iter().copied().collect();
    assert_eq!(hand, hand_of(&recording.deal, PlayerId::HUMAN, PlayerId(0)));
    assert!(view.open_cards.is_none(), "ordinary table stays closed");
    assert!(view.sheet.records.is_empty() && view.players.iter().all(|p| p.total == 0));
    assert_eq!(practice.goldfish(), None);
    assert_eq!(practice.coach_settings(), live.coach_settings());

    // Same human choices reproduce the recorded game action for action:
    // identical cards, seats, rules, levels and opponent seeds.
    let mut replay = live.start_practice(&recording).unwrap();
    let mut n = 0;
    loop {
        if replay.opponent_to_move() {
            let job = replay.opponent_job().unwrap();
            assert!(
                !job.sees_all_cards(),
                "ordinary opponents keep their own view"
            );
            assert_eq!(job.level(), recording.context.session.level(job.player()));
            replay.finish_opponent(job.run(None)).unwrap();
        } else if replay.view().legal_actions.is_empty() {
            break;
        } else {
            replay.execute(Command::Act(recording.actions[n])).unwrap();
        }
        n += 1;
    }
    assert_eq!(n, recording.actions.len());
    let end = replay.view();
    let original = recording.reconstruct_privileged(n).unwrap().state;
    assert_eq!(end.phase, original.seats[0].phase);
    assert_eq!(end.bidding.len(), original.seats[0].bidding.len());
    assert_eq!(
        end.result.map(|r| r.result),
        original.seats[0].result,
        "same settlement as the original"
    );
    // The live series was only read.
    assert_eq!(live.view(), live_view);
}

#[test]
fn goldfish_god_mode_and_kontra_re_configuration_is_preserved_for_late_games() {
    let live = Session::new(config(Opponents::Levels([Level::Easy; 2])));
    for (opponents, goldfish) in [
        (Opponents::Goldfish, Some(PlayerId(2))),
        (Opponents::GodMode, None),
        (Opponents::Mephisto, None),
        (Opponents::Levels([Level::Strong, Level::Easy]), None),
    ] {
        let recording = synthetic(opponents, 5, goldfish);
        let mut practice = live.start_practice(&recording).unwrap();
        let view = practice.view();
        assert_eq!(view.game_number, 5);
        assert_eq!(view.dealer, PlayerId(0));
        assert_eq!(view.config.opponents, opponents);
        assert!(view.config.kontra_re);
        assert_eq!(practice.goldfish(), goldfish);
        // Only God Mode shows every card to the human and the coach.
        assert_eq!(view.open_cards.is_some(), opponents.god_mode());
        let job = practice.opponent_job().expect("Mittelhand bids first");
        assert_eq!(job.sees_all_cards(), opponents.mephisto());
        practice.finish_opponent(job.run(None)).unwrap();
        while practice.opponent_to_move() {
            practice.execute(Command::AdvanceOpponent).unwrap();
        }
        if practice.view().acting == Some(PlayerId::HUMAN) {
            practice.set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 10,
            });
            practice.execute(Command::RequestAdvice).unwrap();
            let coach = practice.coach_job().unwrap();
            assert_eq!(coach.sees_all_cards(), opponents.god_mode());
        }
    }
}

#[test]
fn alternative_choices_complete_without_touching_the_live_series_or_its_file() {
    let (mut live, recording) = late_series(Opponents::Levels([Level::Easy; 2]));
    let dir = temp_dir();
    let path = dir.join("game2.json");
    recording.save_local_new(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    let opened = GameRecording::open_local(&path).unwrap();
    let live_view = live.view();
    let live_records = live.completed_recordings_privileged().to_vec();
    let live_current = live.export_current_recording_privileged();

    // Completed attempt with a different first human choice.
    let mut practice = live.start_practice(&opened).unwrap();
    let mut n = 0;
    let mut deviated = false;
    loop {
        if practice.opponent_to_move() {
            practice.execute(Command::AdvanceOpponent).unwrap();
        } else {
            let view = practice.view();
            if view.legal_actions.is_empty() {
                break;
            }
            let original = opened.actions.get(n).copied();
            let action = if !deviated {
                deviated = true;
                *view
                    .legal_actions
                    .iter()
                    .find(|a| Some(**a) != original)
                    .expect("an alternative exists at the first human decision")
            } else {
                low(&view)
            };
            practice.execute(Command::Act(action)).unwrap();
        }
        n += 1;
    }
    assert!(deviated);
    let end = practice.view();
    assert!(matches!(end.phase, Phase::Beendet | Phase::Eingepasst));
    assert!(end.sheet.records.is_empty(), "practice is never scored");
    assert!(!end.can_next_game && !end.can_end_series && !end.series_finished);
    assert_eq!(
        practice.execute(Command::NextGame),
        Err(AppError::PracticeAttempt)
    );
    assert_eq!(
        practice.execute(Command::EndSeries),
        Err(AppError::PracticeAttempt)
    );
    assert_eq!(practice.view().game_number, 2, "no other deal is entered");
    assert!(practice.export_current_recording_privileged().is_none());
    assert!(practice.completed_recordings_privileged().is_empty());

    // Abandoned attempt: dropped mid-game.
    let mut abandoned = live.start_practice(&opened).unwrap();
    abandoned.run_opponents();
    let first = abandoned.view().legal_actions[0];
    abandoned.execute(Command::Act(first)).unwrap();
    drop(abandoned);

    assert_eq!(live.view(), live_view);
    assert_eq!(live.completed_recordings_privileged(), live_records);
    assert_eq!(live.export_current_recording_privileged(), live_current);
    assert_eq!(fs::read(&path).unwrap(), bytes, "original file unchanged");
    assert_eq!(GameRecording::open_local(&path).unwrap(), recording);
    // The live series continues exactly as before.
    count(&mut live, low);
    assert_eq!(live.sheet().records.len(), 3);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn moves_and_coach_results_of_other_sessions_or_attempts_are_stale() {
    let mut live = Session::new(config(Opponents::Levels([Level::Easy; 2])));
    let budget = Budget {
        samples: 1,
        nodes_per_task: 10,
    };
    live.set_coach_budget(budget);
    // Same game, same position: the live current game before bidding.
    let recording = live.export_current_recording_privileged().unwrap();
    let mut first = live.start_practice(&recording).unwrap();
    let mut second = live.start_practice(&recording).unwrap();
    let live_move = live.opponent_job().unwrap().run(None);
    let first_move = first.opponent_job().unwrap().run(None);
    assert_eq!(
        first.finish_opponent(live_move),
        Err(AppError::StaleOpponentMove)
    );
    assert_eq!(
        live.finish_opponent(first_move),
        Err(AppError::StaleOpponentMove)
    );
    assert_eq!(
        second.finish_opponent(first_move),
        Err(AppError::StaleOpponentMove)
    );
    // Each accepts only its own.
    first.finish_opponent(first_move).unwrap();
    live.finish_opponent(live_move).unwrap();
    let second_move = second.opponent_job().unwrap().run(None);
    second.finish_opponent(second_move).unwrap();
    for s in [&mut live, &mut first, &mut second] {
        s.run_opponents();
        assert_eq!(s.view().acting, Some(PlayerId::HUMAN));
        s.execute(Command::RequestAdvice).unwrap();
    }
    let live_result = live.coach_job().unwrap().run(None);
    let first_result = first.coach_job().unwrap().run(None);
    // Same decision and coach seed policy, yet never accepted across sessions.
    assert_eq!(
        first.coach_job().unwrap().seed(),
        live.coach_job().unwrap().seed()
    );
    assert_eq!(
        first.finish_coach(live_result.clone()),
        Err(AppError::StaleCoachResult)
    );
    assert_eq!(
        second.finish_coach(first_result.clone()),
        Err(AppError::StaleCoachResult)
    );
    assert_eq!(
        live.finish_coach(first_result.clone()),
        Err(AppError::StaleCoachResult)
    );
    assert!(first.finish_coach(first_result).is_ok());
    assert!(live.finish_coach(live_result).is_ok());
}

#[test]
fn invalid_or_unsupported_recordings_build_nothing_and_leave_the_live_series() {
    let (live, recording) = late_series(Opponents::Levels([Level::Easy; 2]));
    let before = live.view();
    let mut cases: Vec<(GameRecording, RecordingError)> = Vec::new();
    let mut r = recording.clone();
    r.format = 99;
    cases.push((r, RecordingError::UnsupportedFormat { found: 99 }));
    let mut r = recording.clone();
    r.engine = "future".into();
    cases.push((r, RecordingError::UnsupportedEngine));
    let mut r = recording.clone();
    r.context.dealer = PlayerId(2);
    cases.push((r, RecordingError::InvalidContext));
    let mut r = recording.clone();
    r.context.goldfish = Some(PlayerId(1));
    cases.push((r, RecordingError::InvalidContext));
    let mut r = recording.clone();
    r.rules.kontra_re = false;
    cases.push((r, RecordingError::RulesMismatch));
    let mut r = recording.clone();
    r.seed ^= 1;
    cases.push((r, RecordingError::DealMismatch));
    let mut r = recording.clone();
    r.end = RecordingEnd::InProgress;
    cases.push((r, RecordingError::EndMismatch));
    let mut r = recording.clone();
    let last = r.actions.len() - 1;
    r.actions[last] = Action::Bid(999);
    assert!(matches!(
        live.start_practice(&r),
        Err(RecordingError::IllegalAction { .. }) | Err(RecordingError::EndMismatch)
    ));
    let mut r = recording.clone();
    r.actions.push(Action::Pass);
    cases.push((
        r,
        RecordingError::ActionsAfterCompletion {
            index: recording.actions.len(),
        },
    ));
    for (bad, error) in cases {
        assert_eq!(live.start_practice(&bad).err(), Some(error));
    }
    assert_eq!(live.view(), before);
    // A valid record still starts afterwards.
    assert!(live.start_practice(&recording).is_ok());
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "skat-practice-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}
