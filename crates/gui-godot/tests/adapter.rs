// SPDX-License-Identifier: EUPL-1.2

//! Contract tests of the Godot adapter: no layout, no Godot runtime.

use std::time::{Duration, Instant};

use gui_godot::adapter::{Adapter, AdapterError};
use skat_app::{
    AppError, Command, Level, Opponents, SeriesLength, Session, SessionConfig, TableDisplay,
    TableView,
};

fn config(seed: u64) -> SessionConfig {
    SessionConfig {
        seed,
        series: SeriesLength::Fixed(2),
        kontra_re: false,
        opponents: Opponents::Levels([Level::Easy, Level::Easy]),
        ..SessionConfig::default()
    }
}

fn json(config: &SessionConfig) -> String {
    serde_json::to_string(config).unwrap()
}

fn view(adapter: &Adapter) -> TableView {
    serde_json::from_str(&adapter.view_json()).unwrap()
}

fn command(command: Command) -> String {
    serde_json::to_string(&command).unwrap()
}

/// Waits (bounded) for the worker and applies its result.
fn finish_opponent(adapter: &mut Adapter) -> Option<String> {
    let end = Instant::now() + Duration::from_secs(60);
    while Instant::now() < end {
        if let Some(events) = adapter.poll_opponent().unwrap() {
            return Some(events);
        }
        if !adapter.busy() {
            return None;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("opponent did not finish");
}

/// Plays the whole series through the adapter: first legal action each time.
fn play_adapter(seed: u64) -> TableView {
    let mut adapter = Adapter::new(&json(&config(seed))).unwrap();
    loop {
        let v = view(&adapter);
        if v.series_finished {
            return v;
        }
        if v.opponent_to_move {
            assert!(adapter.start_opponent());
            assert!(!adapter.start_opponent(), "one computation at a time");
            finish_opponent(&mut adapter).expect("move applied");
        } else if let Some(&action) = v.legal_actions.first() {
            adapter
                .execute_json(&command(Command::Act(action)))
                .unwrap();
        } else {
            adapter.execute_json(&command(Command::NextGame)).unwrap();
        }
    }
}

fn play_direct(seed: u64) -> TableView {
    let mut session = Session::new(config(seed));
    while !session.view().series_finished {
        session.run_opponents();
        let next = match session.view().legal_actions.first() {
            Some(&a) => Command::Act(a),
            None => Command::NextGame,
        };
        session.execute(next).unwrap();
    }
    session.view()
}

#[test]
fn adapter_matches_direct_session_and_is_deterministic() {
    let first = play_adapter(11);
    assert_eq!(first, play_adapter(11));
    assert_eq!(first, play_direct(11));
    assert_ne!(first.sheet, play_adapter(12).sheet);
}

#[test]
fn view_is_the_permitted_view_only() {
    let adapter = Adapter::new(&json(&config(3))).unwrap();
    let v = view(&adapter);
    assert_eq!(v.hand.len(), 10);
    assert!(
        v.open_cards.is_none(),
        "closed table hides every other card"
    );
    assert!(!adapter.view_json().contains("open_cards\":{"));
    let open = SessionConfig {
        display: TableDisplay::Open,
        ..config(3)
    };
    assert!(
        view(&Adapter::new(&json(&open)).unwrap())
            .open_cards
            .is_some()
    );
}

#[test]
fn errors_are_structured_and_leave_the_session_unchanged() {
    let mut adapter = Adapter::new(&json(&config(5))).unwrap();
    let before = adapter.view_json();
    let err = adapter.execute_json("{not json").unwrap_err();
    assert!(matches!(err, AdapterError::Invalid(_)));
    assert!(err.to_json().starts_with(r#"{"error":{"Invalid""#));
    let err = adapter
        .execute_json(&command(Command::NextGame))
        .unwrap_err();
    assert_eq!(err, AdapterError::App(AppError::GameInProgress));
    assert_eq!(err.to_json(), r#"{"error":{"App":"GameInProgress"}}"#);
    assert_eq!(adapter.view_json(), before);
    assert!(matches!(
        Adapter::new("{}").err(),
        Some(AdapterError::Invalid(_))
    ));
}

/// Drives the adapter until an opponent is to move.
fn to_opponent_turn(adapter: &mut Adapter) {
    for _ in 0..100 {
        let v = view(adapter);
        if v.opponent_to_move {
            return;
        }
        let action = *v.legal_actions.first().expect("human acts");
        adapter
            .execute_json(&command(Command::Act(action)))
            .unwrap();
    }
    panic!("no opponent turn reached");
}

#[test]
fn restart_discards_a_running_computation() {
    let mut adapter = Adapter::new(&json(&config(21))).unwrap();
    to_opponent_turn(&mut adapter);
    assert!(adapter.start_opponent());
    adapter.restart(&json(&config(22))).unwrap();
    assert_eq!(adapter.generation(), 1);
    assert!(!adapter.busy());
    let fresh = view(&adapter);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        adapter.poll_opponent().unwrap(),
        None,
        "stale result dropped"
    );
    assert_eq!(view(&adapter), fresh, "new session untouched");
    // The new session works normally afterwards.
    to_opponent_turn(&mut adapter);
    assert!(adapter.start_opponent());
    assert!(finish_opponent(&mut adapter).is_some());
}

#[test]
fn rejected_restart_keeps_the_old_session() {
    let mut adapter = Adapter::new(&json(&config(31))).unwrap();
    let before = adapter.view_json();
    assert!(adapter.restart("[]").is_err());
    assert_eq!(adapter.generation(), 0);
    assert_eq!(adapter.view_json(), before);
}

#[test]
fn human_move_makes_pending_result_stale() {
    let mut adapter = Adapter::new(&json(&config(41))).unwrap();
    to_opponent_turn(&mut adapter);
    assert!(adapter.start_opponent());
    let events = finish_opponent(&mut adapter);
    assert!(events.is_some());
    // Nothing pending: polling again changes nothing.
    assert_eq!(adapter.poll_opponent().unwrap(), None);
}

#[test]
fn dropping_a_busy_adapter_is_safe() {
    let mut adapter = Adapter::new(&json(&config(51))).unwrap();
    to_opponent_turn(&mut adapter);
    assert!(adapter.start_opponent());
    drop(adapter);
}
