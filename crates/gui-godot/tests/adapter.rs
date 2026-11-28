// SPDX-License-Identifier: EUPL-1.2

//! Contract tests of the Godot adapter: no layout, no Godot runtime.

use std::time::{Duration, Instant};

use gui_godot::adapter::{Adapter, AdapterError};
use skat_app::{
    AppError, AssistSettings, Command, CountingState, Level, OpponentEngine, Opponents,
    SeriesLength, Session, SessionConfig, TableDisplay, TableView,
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
fn sheet_and_series_result_cross_the_wire_like_the_egui_view_model() {
    let direct = play_direct(11);
    let wire = serde_json::to_value(play_adapter(11)).unwrap();
    assert_eq!(wire["sheet"], serde_json::to_value(&direct.sheet).unwrap());
    assert_eq!(
        wire["series_result"],
        serde_json::to_value(direct.series_result.as_ref().unwrap()).unwrap()
    );
    assert_eq!(direct.series_result, Some(direct.sheet.result()));
    assert_eq!(direct.sheet.records.len(), 2);
    let totals = direct.sheet.totals();
    assert!(
        direct
            .players
            .iter()
            .all(|p| p.total == totals[p.player.index()])
    );
}

#[test]
fn cast_resolves_to_catalogue_keys_like_the_egui_view_model() {
    let adapter = Adapter::new(&json(&config(11))).unwrap();
    let keys: Vec<String> = serde_json::from_str(&Adapter::figure_keys_json()).unwrap();
    assert_eq!(keys.len(), skat_app::FIGURES.len());
    let cast = view(&adapter).cast;
    let wire: serde_json::Value = serde_json::from_str(&adapter.view_json()).unwrap();
    for (pool, name) in [
        (skat_app::FigurePool::German, "german"),
        (skat_app::FigurePool::English, "english"),
    ] {
        for seat in 0..2 {
            let id = wire["cast"][name][seat].as_u64().unwrap() as usize;
            assert_eq!(keys[id], cast.pool(pool)[seat].key());
        }
    }
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

/// Issue #444: the Reizen, Skat, Drücken and Ansage phases, driven the way
/// GDScript does it: raw JSON of a legal action wrapped as `{"Act": ...}`.
mod declarer_flow {
    use super::*;
    use serde_json::{Value, json};
    use skat_app::{Action, Announcement, Card, CardSet, GameType, Phase};

    /// An action that is not legal in `legal` (an action of another phase).
    fn illegal_for(legal: &[Action]) -> Action {
        [
            Action::Hold,
            Action::Pass,
            Action::Bid(18),
            Action::Hand,
            Action::TakeSkat,
            Action::Discard(CardSet::EMPTY),
            Action::Announce(Announcement::plain(GameType::Grand)),
            Action::Play(Card::from_index(0).unwrap()),
        ]
        .into_iter()
        .find(|a| !legal.contains(a))
        .expect("some action is illegal")
    }

    /// Highest bid, else Hold, else the last action (Pass).
    fn bid_to_win(actions: &[Action]) -> usize {
        let bid = actions
            .iter()
            .rposition(|a| matches!(a, Action::Bid(_) | Action::Hold));
        bid.unwrap_or(actions.len() - 1)
    }

    fn raw_actions(adapter: &Adapter) -> Vec<Value> {
        let view: Value = serde_json::from_str(&adapter.view_json()).unwrap();
        view["legal_actions"].as_array().unwrap().clone()
    }

    fn act_raw(adapter: &mut Adapter, action: &Value) -> Result<String, AdapterError> {
        adapter.execute_json(&json!({ "Act": action }).to_string())
    }

    /// Every phase rejects an illegal action with a structured error and
    /// leaves the session unchanged.
    fn assert_rejects(adapter: &mut Adapter) {
        let v = view(adapter);
        let bad = illegal_for(&v.legal_actions);
        let before = adapter.view_json();
        let err = adapter
            .execute_json(&command(Command::Act(bad)))
            .unwrap_err();
        assert!(
            matches!(err, AdapterError::App(AppError::Rule(_))),
            "{:?} in {:?}: {err:?}",
            bad,
            v.phase
        );
        assert!(err.to_json().starts_with(r#"{"error":{"App":{"Rule""#));
        assert_eq!(adapter.view_json(), before);
    }

    #[test]
    fn every_declarer_phase_converts_commands_and_rejects_illegal_ones() {
        let mut seen = std::collections::BTreeSet::new();
        let (mut took_skat, mut hand, mut ouvert) = (false, false, false);
        for seed in 1..200 {
            let mut adapter = Adapter::new(&json(&config(seed))).unwrap();
            for _ in 0..400 {
                let v = view(&adapter);
                if v.series_finished || v.phase == Phase::Spiel {
                    break;
                }
                if v.opponent_to_move {
                    adapter.start_opponent();
                    finish_opponent(&mut adapter);
                    continue;
                }
                if v.legal_actions.is_empty() {
                    adapter.execute_json(&command(Command::NextGame)).unwrap();
                    continue;
                }
                seen.insert(format!("{:?}", v.phase));
                assert_rejects(&mut adapter);
                let raw = raw_actions(&adapter);
                assert_eq!(raw.len(), v.legal_actions.len());
                let pick = match v.phase {
                    Phase::Reizen => bid_to_win(&v.legal_actions),
                    Phase::SkatAufnahme => {
                        // Alternate Hand and Skat pickup over the seeds.
                        let want = if seed % 2 == 0 {
                            Action::Hand
                        } else {
                            Action::TakeSkat
                        };
                        let i = v.legal_actions.iter().position(|a| *a == want);
                        hand |= i.is_some() && want == Action::Hand;
                        took_skat |= i.is_some() && want == Action::TakeSkat;
                        i.unwrap_or(0)
                    }
                    Phase::Ansage => {
                        let i = v
                            .legal_actions
                            .iter()
                            .position(|a| matches!(a, Action::Announce(x) if x.ouvert));
                        ouvert |= i.is_some();
                        i.unwrap_or(0)
                    }
                    _ => 0,
                };
                act_raw(&mut adapter, &raw[pick]).unwrap();
            }
        }
        for phase in ["Reizen", "SkatAufnahme", "Druecken", "Ansage"] {
            assert!(seen.contains(phase), "{phase} not reached: {seen:?}");
        }
        assert!(took_skat && hand && ouvert, "{took_skat} {hand} {ouvert}");
    }

    #[test]
    fn discard_masks_in_the_view_are_exactly_two_held_cards() {
        for seed in 1..200 {
            let mut adapter = Adapter::new(&json(&config(seed))).unwrap();
            for _ in 0..400 {
                let v = view(&adapter);
                if v.series_finished || v.phase == Phase::Spiel {
                    break;
                }
                if v.opponent_to_move {
                    adapter.start_opponent();
                    finish_opponent(&mut adapter);
                    continue;
                }
                if v.phase == Phase::Druecken {
                    assert_eq!(v.hand.len(), 12);
                    for a in &v.legal_actions {
                        let Action::Discard(set) = a else {
                            panic!("only Discard in Drücken: {a:?}")
                        };
                        assert_eq!(set.iter().count(), 2);
                        assert!(set.iter().all(|c| v.hand.contains(&c)));
                    }
                    return;
                }
                let cmd = if v.legal_actions.is_empty() {
                    Command::NextGame
                } else if v.phase == Phase::Reizen {
                    Command::Act(v.legal_actions[bid_to_win(&v.legal_actions)])
                } else if v.phase == Phase::SkatAufnahme {
                    Command::Act(Action::TakeSkat)
                } else {
                    Command::Act(v.legal_actions[0])
                };
                adapter.execute_json(&command(cmd)).unwrap();
            }
        }
        panic!("no Drücken phase reached");
    }
}

#[test]
fn engine_selection_travels_over_the_wire_and_classic_is_omitted() {
    assert!(!json(&config(3)).contains("engine"));
    for engine in [
        OpponentEngine::Neural,
        OpponentEngine::PrePlay,
        OpponentEngine::Hybrid,
    ] {
        let selected = SessionConfig {
            engine,
            ..config(3)
        };
        let text = json(&selected);
        assert!(text.contains(&format!("\"engine\":\"{engine:?}\"")));
        // Missing models fall back to Classic; the session must start either way.
        let adapter = Adapter::new(&text).unwrap();
        assert_eq!(view(&adapter).hand.len(), 10);
    }
}

/// Plays (first legal action) until a counting question is pending.
fn until_question(adapter: &mut Adapter) -> TableView {
    for _ in 0..2000 {
        let v = view(adapter);
        if v.assist.quiz.question.is_some() {
            return v;
        }
        if v.series_finished {
            break;
        }
        if v.opponent_to_move {
            assert!(adapter.start_opponent());
            finish_opponent(adapter).expect("move applied");
        } else if let Some(&action) = v.legal_actions.first() {
            adapter
                .execute_json(&command(Command::Act(action)))
                .unwrap();
        } else {
            adapter.execute_json(&command(Command::NextGame)).unwrap();
        }
    }
    panic!("no question was asked");
}

fn assist_on(adapter: &mut Adapter) {
    let settings = AssistSettings {
        counting_aid: true,
        quiz: true,
        ..AssistSettings::default()
    };
    adapter
        .execute_json(&command(Command::SetAssist(settings)))
        .unwrap();
}

fn temp_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gui-godot-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("learning-history.json")
}

#[test]
fn counting_aid_and_quiz_show_only_the_permitted_view() {
    let mut adapter = Adapter::new(&json(&config(3))).unwrap();
    assert_eq!(view(&adapter).assist.counting, CountingState::Off);
    assist_on(&mut adapter);
    // The wire form equals the permitted view of a direct session.
    let mut direct = Session::new(config(3));
    direct
        .execute(Command::SetAssist(view(&adapter).assist.settings))
        .unwrap();
    assert_eq!(view(&adapter).assist, direct.view().assist);
    assert!(matches!(
        view(&adapter).assist.counting,
        CountingState::Shown(_)
    ));
    let asked = until_question(&mut adapter);
    // The question holds the decision and withholds aid and help, so the
    // answer cannot be read from the view.
    assert_eq!(asked.assist.counting, CountingState::Withheld);
    assert!(asked.assist.quiz.help.is_none());
    let wire = serde_json::to_value(&asked.assist).unwrap();
    assert_eq!(wire["quiz"]["help"], serde_json::Value::Null);
    assert_eq!(wire["counting"], serde_json::json!("Withheld"));
    // No hidden card enters the aid's JSON: the played cards are tricks.
    adapter
        .execute_json(&command(Command::RestoreQuizHelp))
        .unwrap();
    let helped = view(&adapter);
    assert!(helped.assist.quiz.help.is_some());
    assert!(matches!(helped.assist.counting, CountingState::Shown(_)));
    // Game actions stay rejected while the question is pending.
    let act = Command::Act(helped.legal_actions[0]);
    assert!(matches!(
        adapter.execute_json(&command(act)),
        Err(AdapterError::App(AppError::QuestionPending))
    ));
}

#[test]
fn quiz_events_reach_the_persistent_learning_history() {
    let path = temp_file("quiz");
    let mut adapter = Adapter::new(&json(&config(3))).unwrap();
    adapter.use_learning_file(path.clone());
    assist_on(&mut adapter);
    until_question(&mut adapter);
    let asked = adapter.learning_events();
    adapter.execute_json(&command(Command::SkipQuiz)).unwrap();
    assert!(adapter.learning_events() > asked, "skip is recorded");
    assert!(path.exists(), "history file written");
    let mut reopened = Adapter::new(&json(&config(3))).unwrap();
    reopened.use_learning_file(path.clone());
    assert_eq!(reopened.learning_events(), adapter.learning_events());
    // An answer is recorded too.
    until_question(&mut adapter);
    let before = adapter.learning_events();
    adapter
        .execute_json(&command(Command::AnswerQuiz(1)))
        .unwrap();
    assert!(adapter.learning_events() > before, "answer is recorded");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn damaged_learning_file_is_left_unchanged() {
    let path = temp_file("damaged");
    std::fs::write(&path, "not json").unwrap();
    let mut adapter = Adapter::new(&json(&config(3))).unwrap();
    adapter.use_learning_file(path.clone());
    assist_on(&mut adapter);
    until_question(&mut adapter);
    adapter.execute_json(&command(Command::SkipQuiz)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn recording_round_trip_exposes_only_the_human_historical_view() {
    let mut adapter = Adapter::new(&json(&config(11))).unwrap();
    loop {
        let v = view(&adapter);
        if v.can_next_game || v.series_finished {
            break;
        }
        if v.opponent_to_move {
            assert!(adapter.start_opponent());
            finish_opponent(&mut adapter).unwrap();
        } else if let Some(&action) = v.legal_actions.first() {
            adapter
                .execute_json(&command(Command::Act(action)))
                .unwrap();
        } else {
            panic!("game stalled");
        }
    }
    let path = std::env::temp_dir().join(format!(
        "grand-coach-godot-replay-{}.json",
        std::process::id()
    ));
    adapter.save_recording(&path).unwrap();
    assert!(adapter.save_recording(&path).is_err());
    let recording = skat_app::GameRecording::open_local(&path).unwrap();
    adapter.open_recording(&path).unwrap();
    for index in 0..=recording.actions.len() {
        let wire: serde_json::Value =
            serde_json::from_str(&adapter.replay_json(index).unwrap()).unwrap();
        let state = recording.reconstruct_privileged(index).unwrap().state;
        let expected = &state.seats[skat_app::PlayerId::HUMAN
            .seat(recording.context.dealer)
            .index()];
        assert_eq!(wire["view"], serde_json::to_value(expected).unwrap());
        assert_eq!(
            wire["hand"],
            serde_json::to_value(expected.hand.iter().collect::<Vec<_>>()).unwrap()
        );
        assert!(wire.get("deal").is_none() && wire.get("state").is_none());
    }
    assert!(adapter.replay_json(recording.actions.len() + 1).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn training_and_history_equal_the_direct_skat_app_result() {
    use skat_app::{
        AssistanceLevel, LearningHistory, TrainingCommand, TrainingSession, TrainingTopic,
        TrainingView,
    };
    let path = temp_file("training");
    let mut adapter = Adapter::new(&json(&config(3))).unwrap();
    adapter.use_learning_file(path.clone());
    let game_before = adapter.view_json();
    assert_eq!(adapter.training_view_json(), None);
    let level = AssistanceLevel::AnswerBeforeReveal;
    adapter
        .start_training(
            &serde_json::to_string(&TrainingTopic::TrumpfOutside).unwrap(),
            &serde_json::to_string(&level).unwrap(),
        )
        .unwrap();

    let mut direct = TrainingSession::new(TrainingTopic::TrumpfOutside, level);
    let mut history = LearningHistory::new();
    let mut attempt = None;
    let mut presented = None;
    let mut record = |session: &mut TrainingSession| {
        if presented != Some(session.presentation()) {
            presented = Some(session.presentation());
            attempt = None;
        }
        let events = session.take_events();
        let id =
            *attempt.get_or_insert_with(|| history.begin_attempt(session.attempt_kind()).unwrap());
        if !events.is_empty() {
            history.record(id, &events).unwrap();
        }
    };
    record(&mut direct);
    for command in [
        TrainingCommand::Help,
        TrainingCommand::Answer(2),
        TrainingCommand::Retry,
        TrainingCommand::Skip,
        TrainingCommand::Next,
    ] {
        adapter
            .training_command_json(&serde_json::to_string(&command).unwrap())
            .unwrap();
        direct.execute(command).unwrap();
        record(&mut direct);
        let wire = adapter.training_view_json().unwrap();
        assert_eq!(wire, serde_json::to_string(&direct.view()).unwrap());
    }
    // A refused command leaves the training and the history unchanged.
    let before = (
        adapter.training_view_json(),
        adapter.learning_summary_json(),
    );
    assert!(matches!(
        adapter.training_command_json(&serde_json::to_string(&TrainingCommand::Retry).unwrap()),
        Err(AdapterError::Training(_))
    ));
    assert_eq!(
        before,
        (
            adapter.training_view_json(),
            adapter.learning_summary_json()
        )
    );

    // Statistics equal the direct summary and the file reads back unchanged.
    assert_eq!(
        adapter.learning_summary_json(),
        serde_json::to_string(&history.summary()).unwrap()
    );
    let mut reopened = Adapter::new(&json(&config(3))).unwrap();
    reopened.use_learning_file(path.clone());
    assert_eq!(
        reopened.learning_summary_json(),
        adapter.learning_summary_json()
    );
    assert!(adapter.learning_events() > 0);

    // Permitted view only: the training's own hand, no deal or other hands,
    // and the running game is untouched.
    let wire: serde_json::Value =
        serde_json::from_str(&adapter.training_view_json().unwrap()).unwrap();
    let keys: Vec<&String> = wire.as_object().unwrap().keys().collect();
    for hidden in ["deal", "hands", "skat", "state", "seats"] {
        assert!(
            !keys.iter().any(|k| *k == hidden),
            "{hidden} crossed the wire"
        );
    }
    let _: TrainingView = serde_json::from_value(wire).unwrap();
    assert_eq!(adapter.view_json(), game_before);
    adapter.stop_training();
    assert_eq!(adapter.training_view_json(), None);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
