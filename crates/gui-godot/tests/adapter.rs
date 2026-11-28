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
            for _ in 0..12 {
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
    let expected = skat_app::LearningHistory::open_local(&path)
        .unwrap()
        .summary();
    let summary: skat_app::LearningSummary =
        serde_json::from_str(&adapter.learning_summary_json()).unwrap();
    assert_eq!(summary, expected);
    assert_eq!(summary.totals.skips, 1);
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
fn same_deal_practice_parks_the_series_and_keeps_cards_private() {
    let mut adapter = Adapter::new(&json(&config(31))).unwrap();
    let original = view(&adapter);
    adapter.practice().unwrap();
    let first = view(&adapter);
    assert!(first.practice);
    assert_eq!(first.hand, original.hand);
    assert!(first.open_cards.is_none());
    adapter.practice().unwrap();
    let retry = view(&adapter);
    assert_eq!(retry.hand, first.hand);
    assert!(retry.open_cards.is_none());
    adapter.return_to_series();
    assert_eq!(view(&adapter), original);

    let mut open_config = config(31);
    open_config.display = TableDisplay::Open;
    let mut open = Adapter::new(&json(&open_config)).unwrap();
    let deal = view(&open)
        .open_cards
        .expect("open display reveals the deal");
    open.practice().unwrap();
    assert_eq!(view(&open).open_cards, Some(deal.clone()));
    open.practice().unwrap();
    assert_eq!(view(&open).open_cards, Some(deal));
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

/// Advances the adapter and a direct session in lockstep (first legal action)
/// to the first human card choice among at least two cards.
fn to_card_choice(adapter: &mut Adapter, session: &mut Session) {
    for _ in 0..2000 {
        let v = view(adapter);
        if v.opponent_to_move {
            assert!(adapter.start_opponent());
            finish_opponent(adapter).expect("move applied");
            session.run_opponents();
            continue;
        }
        assert_eq!(v, session.view());
        let plays = v
            .legal_actions
            .iter()
            .filter(|a| matches!(a, skat_app::Action::Play(_)))
            .count();
        if plays >= 2 {
            return;
        }
        let next = match v.legal_actions.first() {
            Some(&a) => Command::Act(a),
            None => Command::NextGame,
        };
        adapter.execute_json(&command(next)).unwrap();
        session.execute(next).unwrap();
    }
    panic!("no card choice reached");
}

/// Session and generation counters are process-global, so equal sessions
/// differ in them; everything else must be identical.
fn plain(text: &str) -> serde_json::Value {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("lineage");
                m.remove("generation");
                if let Some(serde_json::Value::Array(b)) = m.get_mut("boundary") {
                    b.remove(0); // (lineage, game, actions)
                }
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut v = serde_json::from_str(text).unwrap();
    strip(&mut v);
    v
}

/// The adapter's and the direct session's own identity for the open dialogue.
fn guided_ids(adapter: &Adapter, session: &Session) -> (skat_app::GuidedId, skat_app::GuidedId) {
    let wire: Option<skat_app::GuidedView> = serde_json::from_str(&adapter.guided_json()).unwrap();
    (wire.unwrap().id, session.guided().unwrap().id)
}

/// Equivalent commands on both sides: events, dialogue and question views
/// must be equal.
fn both(
    adapter: &mut Adapter,
    session: &mut Session,
    wire: Command,
    direct: Command,
) -> Result<(), AppError> {
    let direct = session.execute(direct);
    let wire = adapter.execute_json(&command(wire));
    let outcome = match (direct, wire) {
        (Ok(events), Ok(text)) => {
            assert_eq!(
                plain(&text),
                plain(&serde_json::to_string(&events).unwrap())
            );
            Ok(())
        }
        (Err(e), Err(AdapterError::App(w))) => {
            assert_eq!(e, w);
            Err(e)
        }
        other => panic!("diverged: {other:?}"),
    };
    assert_eq!(
        plain(&adapter.guided_json()),
        plain(&serde_json::to_string(&session.guided()).unwrap())
    );
    assert_eq!(
        plain(&adapter.live_abstich_json()),
        plain(&serde_json::to_string(&session.live_abstich()).unwrap())
    );
    assert_eq!(view(adapter), session.view());
    outcome
}

fn same(adapter: &mut Adapter, session: &mut Session, c: Command) -> Result<(), AppError> {
    both(adapter, session, c, c)
}

fn guided_view(adapter: &Adapter) -> Option<skat_app::GuidedView> {
    serde_json::from_str(&adapter.guided_json()).unwrap()
}

/// Lets the opponents move on both sides until the human acts again.
fn settle(adapter: &mut Adapter, session: &mut Session) {
    while view(adapter).opponent_to_move {
        assert!(adapter.start_opponent());
        finish_opponent(adapter).expect("move applied");
    }
    session.run_opponents();
}

#[test]
fn guided_dialogue_runs_to_observation_with_the_permitted_view_only() {
    let mut adapter = Adapter::new(&json(&config(21))).unwrap();
    let mut session = Session::new(config(21));
    to_card_choice(&mut adapter, &mut session);
    assert_eq!(adapter.guided_json(), "null");

    same(&mut adapter, &mut session, Command::StartGuided).unwrap();
    let opened = guided_view(&adapter).unwrap();
    let permitted = view(&adapter);
    // Everything the dialogue names stays inside the permitted human view.
    assert!(
        opened
            .legal_cards
            .iter()
            .all(|c| permitted.hand.contains(c))
    );
    assert!(opened.rule.is_none() && opened.figures.is_none() && opened.observation.is_none());
    assert!(!adapter.guided_json().contains("seed"));
    assert_ne!(
        opened.step,
        skat_app::GuidedStep::Unsupported,
        "fixture supports a dialogue"
    );

    // Reveal is deliberate and shows the worked answer, never plays it.
    let (wire, direct) = guided_ids(&adapter, &session);
    both(
        &mut adapter,
        &mut session,
        Command::Guided(wire, skat_app::GuidedAction::Reveal),
        Command::Guided(direct, skat_app::GuidedAction::Reveal),
    )
    .unwrap();
    assert_eq!(view(&adapter).legal_actions, permitted.legal_actions);

    // A stale response is refused identically and changes nothing.
    let mut stale = serde_json::to_value(wire).unwrap();
    stale["actions"] = 999.into();
    let stale: skat_app::GuidedId = serde_json::from_value(stale).unwrap();
    let command = Command::Guided(stale, skat_app::GuidedAction::Return);
    assert!(same(&mut adapter, &mut session, command).is_err());

    // Playing any legal card is still free; the dialogue then observes the result.
    let play = *permitted
        .legal_actions
        .iter()
        .find(|a| matches!(a, skat_app::Action::Play(_)))
        .unwrap();
    same(&mut adapter, &mut session, Command::Act(play)).unwrap();
    settle(&mut adapter, &mut session);
    let mut steps = Vec::new();
    for _ in 0..4 {
        let Some(g) = guided_view(&adapter) else {
            break;
        };
        steps.push(g.step);
        if g.step == skat_app::GuidedStep::Observe {
            let (wire, direct) = guided_ids(&adapter, &session);
            let answer = skat_app::GuidedAction::Answer(skat_app::GuidedAnswer::Void(false));
            both(
                &mut adapter,
                &mut session,
                Command::Guided(wire, answer),
                Command::Guided(direct, answer),
            )
            .unwrap();
        } else {
            break;
        }
    }
    assert_eq!(
        steps,
        [
            skat_app::GuidedStep::Observe,
            skat_app::GuidedStep::Complete
        ],
        "the dialogue reaches the real observation after the learner's own card"
    );
}

#[test]
fn live_abstich_question_matches_the_direct_session_and_hides_evidence_until_answered() {
    let mut adapter = Adapter::new(&json(&config(21))).unwrap();
    let mut session = Session::new(config(21));
    to_card_choice(&mut adapter, &mut session);
    let open = |adapter: &Adapter| -> Option<skat_app::LiveAbstichView> {
        serde_json::from_str(&adapter.live_abstich_json()).unwrap()
    };
    assert!(open(&adapter).is_none());
    same(&mut adapter, &mut session, Command::StartLiveAbstich).unwrap();
    let permitted = view(&adapter);
    let first = open(&adapter).unwrap();
    assert!(first.evidence.is_none() && first.response.is_none());
    assert!(first.candidates.iter().all(|c| permitted.hand.contains(c)));
    let mut answered = false;
    for &card in &first.candidates {
        let wire = Command::SelectLiveAbstich(first.boundary, card);
        let direct = Command::SelectLiveAbstich(session.live_abstich().unwrap().boundary, card);
        both(&mut adapter, &mut session, wire, direct).unwrap();
        let selected = open(&adapter).unwrap();
        // Selecting never plays the card and never reveals certainty.
        assert!(selected.evidence.is_none() && selected.response.is_none());
        assert_eq!(view(&adapter).legal_actions, permitted.legal_actions);
        if let Some(id) = selected.id.filter(|id| id.opponent.is_some()) {
            let answer = skat_app::LiveAbstichAction::Answer(skat_app::FocusAnswer::Possible);
            let direct_id = session.live_abstich().unwrap().id.unwrap();
            both(
                &mut adapter,
                &mut session,
                Command::LiveAbstich(id, answer),
                Command::LiveAbstich(direct_id, answer),
            )
            .unwrap();
            let settled = open(&adapter).unwrap();
            assert!(settled.response.is_some() && settled.evidence.is_some());
            answered = true;
            break;
        }
    }
    assert!(answered, "a supported candidate was answered");
    same(&mut adapter, &mut session, Command::CloseLiveAbstich).unwrap();
    assert!(open(&adapter).is_none());
    assert_eq!(view(&adapter).legal_actions, permitted.legal_actions);
}

/// Goldfish, Mephisto and God Mode through the adapter equal the direct
/// `skat-app` result at every step, and only God Mode (D-62) carries the real
/// deal (`open_cards`) in the view; the others stay the closed permitted view.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn opponent_modes_match_skat_app_and_disclose_only_in_god_mode() {
    for opponents in [Opponents::Goldfish, Opponents::Mephisto, Opponents::GodMode] {
        let config = SessionConfig {
            opponents,
            ..config(7)
        };
        let mut adapter = Adapter::new(&json(&config)).unwrap();
        let mut direct = Session::new(config);
        for _ in 0..400 {
            let wire = adapter.view_json();
            assert_eq!(wire, serde_json::to_string(&direct.view()).unwrap());
            let v = view(&adapter);
            assert_eq!(v.open_cards.is_some(), opponents == Opponents::GodMode);
            if v.series_finished {
                break;
            }
            let next = if v.opponent_to_move {
                while view(&adapter).opponent_to_move {
                    assert!(adapter.start_opponent());
                    finish_opponent(&mut adapter).expect("move applied");
                }
                direct.run_opponents();
                continue;
            } else if let Some(&a) = v.legal_actions.first() {
                Command::Act(a)
            } else {
                Command::NextGame
            };
            adapter.execute_json(&command(next)).unwrap();
            direct.execute(next).unwrap();
        }
    }
}

/// Fast form of the above: the opening view of every mode equals the direct
/// result, and only God Mode shows other cards.
#[test]
fn opponent_mode_opening_views_match_skat_app() {
    for opponents in [Opponents::Goldfish, Opponents::Mephisto, Opponents::GodMode] {
        let config = SessionConfig {
            opponents,
            ..config(7)
        };
        let adapter = Adapter::new(&json(&config)).unwrap();
        let direct = serde_json::to_string(&Session::new(config).view()).unwrap();
        assert_eq!(adapter.view_json(), direct);
        assert_eq!(
            view(&adapter).open_cards.is_some(),
            opponents == Opponents::GodMode
        );
    }
}

#[test]
fn reizen_dialogue_matches_the_direct_session_and_stays_in_the_permitted_view() {
    // First seed whose first human decision is a bidding action.
    let seed = (1..200u64)
        .find(|&s| {
            let mut a = Adapter::new(&json(&config(s))).unwrap();
            let mut d = Session::new(config(s));
            settle(&mut a, &mut d);
            view(&a)
                .legal_actions
                .iter()
                .any(|x| matches!(x, skat_app::Action::Pass | skat_app::Action::Bid(_)))
        })
        .expect("a seed starts with a human bid");
    let mut adapter = Adapter::new(&json(&config(seed))).unwrap();
    let mut session = Session::new(config(seed));
    settle(&mut adapter, &mut session);
    let permitted = view(&adapter);

    same(&mut adapter, &mut session, Command::StartGuided).unwrap();
    let opened = guided_view(&adapter).unwrap();
    let reizen = opened.reizen.expect("a Reizen dialogue");
    assert_eq!(opened.step, skat_app::GuidedStep::Orient);
    assert!(opened.bid_rule.is_none() && reizen.holds.is_none() && reizen.stake.is_none());
    let wire = adapter.guided_json();
    assert!(!wire.contains("seed") && !wire.contains("skat\""));

    // A deliberate answer is checked identically; a wrong one can be retried.
    let (id, direct) = guided_ids(&adapter, &session);
    for holds in [true, false] {
        let answer = skat_app::GuidedAction::Answer(skat_app::GuidedAnswer::Holds(holds));
        both(
            &mut adapter,
            &mut session,
            Command::Guided(id, answer),
            Command::Guided(direct, answer),
        )
        .unwrap();
        let g = guided_view(&adapter).unwrap();
        if g.step != skat_app::GuidedStep::Orient {
            break;
        }
        assert_eq!(g.correct, Some(false));
        let retry = skat_app::GuidedAction::Retry;
        both(
            &mut adapter,
            &mut session,
            Command::Guided(id, retry),
            Command::Guided(direct, retry),
        )
        .unwrap();
    }
    let after = guided_view(&adapter).unwrap();
    assert_eq!(after.step, skat_app::GuidedStep::Threshold);

    let reveal = skat_app::GuidedAction::Reveal;
    both(
        &mut adapter,
        &mut session,
        Command::Guided(id, reveal),
        Command::Guided(direct, reveal),
    )
    .unwrap();
    assert!(guided_view(&adapter).unwrap().bid_rule.is_some());
    // The dialogue never acts: the legal choices are untouched.
    assert_eq!(view(&adapter).legal_actions, permitted.legal_actions);
}

/// Scans seeds with a direct session (first non-Pass legal action) for the
/// first human decision the probe accepts; returns the seed and the commands
/// leading there, so adapter and direct session can replay them in lockstep.
fn find_decision(
    series: SeriesLength,
    probe: impl Fn(&mut Session) -> bool,
) -> (SessionConfig, Vec<Command>) {
    for seed in 1..4000u64 {
        let cfg = SessionConfig {
            series,
            kontra_re: true,
            ..config(seed)
        };
        let mut session = Session::new(cfg);
        let mut path = Vec::new();
        for _ in 0..400 {
            session.run_opponents();
            let v = session.view();
            if !v.legal_actions.is_empty() && probe(&mut session) {
                return (cfg, path);
            }
            // Even seeds declare (and pick up the Skat), odd seeds defend.
            let wanted = |a: &&skat_app::Action| {
                if seed % 2 == 1 {
                    **a == skat_app::Action::Pass
                } else {
                    !matches!(a, skat_app::Action::Pass | skat_app::Action::Hand)
                }
            };
            let next = match v.legal_actions.iter().find(wanted) {
                Some(&a) => Command::Act(a),
                None if v.series_finished && v.legal_actions.is_empty() => break,
                None => match v.legal_actions.first() {
                    Some(&a) => Command::Act(a),
                    None => Command::NextGame,
                },
            };
            if let Err(e) = session.execute(next) {
                eprintln!("{seed} {next:?} {e:?}");
                break;
            }
            path.push(next);
        }
    }
    panic!("no matching decision found");
}

fn replay(cfg: &SessionConfig, path: &[Command]) -> (Adapter, Session) {
    let mut adapter = Adapter::new(&json(cfg)).unwrap();
    let mut session = Session::new(*cfg);
    for &c in path {
        settle(&mut adapter, &mut session);
        same(&mut adapter, &mut session, c).unwrap();
    }
    settle(&mut adapter, &mut session);
    (adapter, session)
}

/// Opens the dialogue, answers once (retrying a wrong answer), reveals the
/// worked answer and checks that every step equals the direct session, that
/// the wire carries no seed and that the legal choices stay untouched.
fn run_dialogue(
    adapter: &mut Adapter,
    session: &mut Session,
    answer: skat_app::GuidedAnswer,
) -> skat_app::GuidedView {
    let permitted = view(adapter);
    same(adapter, session, Command::StartGuided).unwrap();
    assert!(!adapter.guided_json().contains("seed"));
    let (id, direct) = guided_ids(adapter, session);
    let attempt = skat_app::GuidedAction::Answer(answer);
    let _ = both(
        adapter,
        session,
        Command::Guided(id, attempt),
        Command::Guided(direct, attempt),
    );
    if guided_view(adapter).and_then(|g| g.correct) == Some(false) {
        let retry = skat_app::GuidedAction::Retry;
        both(
            adapter,
            session,
            Command::Guided(id, retry),
            Command::Guided(direct, retry),
        )
        .unwrap();
    }
    let reveal = skat_app::GuidedAction::Reveal;
    both(
        adapter,
        session,
        Command::Guided(id, reveal),
        Command::Guided(direct, reveal),
    )
    .unwrap();
    assert_eq!(view(adapter).legal_actions, permitted.legal_actions);
    guided_view(adapter).expect("dialogue still open")
}

/// Opens the dialogue on the direct session, asks `accept` about it and
/// closes it again, so the scan continues from an unchanged game.
fn probe_guided(
    session: &mut Session,
    accept: impl Fn(&TableView, &skat_app::GuidedView) -> bool,
) -> bool {
    let table = session.view();
    if session.execute(Command::StartGuided).is_err() {
        return false;
    }
    let Some(g) = session.guided() else {
        return false;
    };
    let ok = g.step != skat_app::GuidedStep::Unsupported && accept(&table, &g);
    session
        .execute(Command::Guided(g.id, skat_app::GuidedAction::Return))
        .unwrap();
    ok
}

fn has_action(v: &TableView, wanted: skat_app::Action) -> bool {
    v.legal_actions.contains(&wanted)
}

#[test]
fn skat_pickup_discard_and_announce_dialogues_match_the_direct_session() {
    for (phase, answer) in [
        (
            skat_app::Phase::SkatAufnahme,
            skat_app::GuidedAnswer::Game(skat_app::GuidedGameBranch::Suit),
        ),
        (
            skat_app::Phase::Druecken,
            skat_app::GuidedAnswer::Game(skat_app::GuidedGameBranch::Grand),
        ),
        (
            skat_app::Phase::Ansage,
            skat_app::GuidedAnswer::Game(skat_app::GuidedGameBranch::Null),
        ),
    ] {
        let (cfg, path) = find_decision(SeriesLength::Fixed(2), |s| {
            s.view().phase == phase
                && probe_guided(s, |_, g| {
                    g.preplay.as_ref().is_some_and(|p| p.phase == phase)
                })
        });
        let (mut adapter, mut session) = replay(&cfg, &path);
        let before = view(&adapter);
        let opened = {
            same(&mut adapter, &mut session, Command::StartGuided).unwrap();
            let g = guided_view(&adapter).unwrap();
            let (id, direct) = guided_ids(&adapter, &session);
            let r = skat_app::GuidedAction::Return;
            both(
                &mut adapter,
                &mut session,
                Command::Guided(id, r),
                Command::Guided(direct, r),
            )
            .unwrap();
            g
        };
        // Orientation shows only public facts: no candidate game, rule or Skat.
        let p = opened.preplay.expect("a preplay dialogue");
        assert_eq!(p.phase, phase);
        assert!(p.game.is_none() && p.threshold.is_none() && opened.pre_rule.is_none());
        if phase == skat_app::Phase::Druecken {
            // Neutral answer controls are the own twelve cards.
            assert!(p.cards.iter().all(|c| before.hand.contains(c)));
        }
        let done = run_dialogue(&mut adapter, &mut session, answer);
        assert!(done.pre_rule.is_some(), "{phase:?}: worked rule revealed");
        assert!(done.preplay.unwrap().game.is_some());
    }
}

#[test]
fn kontra_and_re_dialogues_match_the_direct_session_and_never_call() {
    for (wanted, factor) in [(skat_app::Action::Kontra, 2), (skat_app::Action::Re, 4)] {
        let (cfg, path) = find_decision(SeriesLength::Fixed(6), |s| {
            has_action(&s.view(), wanted)
                && probe_guided(s, |_, g| {
                    g.kontra.is_some() && !g.kontra.unwrap().ouvert_round
                })
        });
        let (mut adapter, mut session) = replay(&cfg, &path);
        let permitted = view(&adapter);
        let done = run_dialogue(
            &mut adapter,
            &mut session,
            skat_app::GuidedAnswer::Factor(factor),
        );
        let k = done.kontra.expect("a Kontra/Re dialogue");
        assert_eq!(k.factor, Some(factor));
        assert!(
            done.kontra_rule.is_some(),
            "{wanted:?}: worked rule revealed"
        );
        // The dialogue never calls: the table still waits for the learner.
        assert!(has_action(&view(&adapter), wanted));
        assert_eq!(view(&adapter).kontra, permitted.kontra);
    }
}

#[test]
fn lead_and_null_dialogues_match_the_direct_session() {
    let lead = |s: &mut Session| {
        let v = s.view();
        let plays = v
            .legal_actions
            .iter()
            .filter(|a| matches!(a, skat_app::Action::Play(_)))
            .count();
        v.phase == skat_app::Phase::Spiel
            && v.current_trick.is_empty()
            && plays >= 2
            && probe_guided(s, |_, g| g.lead.is_some())
    };
    let (cfg, path) = find_decision(SeriesLength::Fixed(2), lead);
    let (mut adapter, mut session) = replay(&cfg, &path);
    let opened = {
        same(&mut adapter, &mut session, Command::StartGuided).unwrap();
        guided_view(&adapter).unwrap()
    };
    let seat = opened.lead.expect("an empty-trick orientation");
    let (id, direct) = guided_ids(&adapter, &session);
    let r = skat_app::GuidedAction::Return;
    both(
        &mut adapter,
        &mut session,
        Command::Guided(id, r),
        Command::Guided(direct, r),
    )
    .unwrap();
    let done = run_dialogue(
        &mut adapter,
        &mut session,
        skat_app::GuidedAnswer::Lead {
            seat,
            declarer: true,
        },
    );
    assert!(done.lead.is_some());

    // A Null contract is worded from the same permitted view.
    let (cfg, path) = find_decision(SeriesLength::Fixed(12), |s| {
        let v = s.view();
        v.phase == skat_app::Phase::Spiel
            && v.contract
                .is_some_and(|c| c.game == skat_app::GameType::Null)
            && v.legal_actions
                .iter()
                .any(|a| matches!(a, skat_app::Action::Play(_)))
            && probe_guided(s, |_, g| {
                g.task == Some(skat_app::GuidedBranch::Null) || g.lead.is_some()
            })
    });
    let (mut adapter, mut session) = replay(&cfg, &path);
    let first = adapter.guided_json();
    assert_eq!(first, "null");
    same(&mut adapter, &mut session, Command::StartGuided).unwrap();
    let g = guided_view(&adapter).unwrap();
    assert_ne!(g.step, skat_app::GuidedStep::Unsupported);
    assert!(g.rule.is_none(), "no rule before the deliberate reveal");
}

#[test]
fn card_memory_and_model_comparison_match_the_direct_session() {
    let mut adapter = Adapter::new(&json(&config(21))).unwrap();
    let mut session = Session::new(config(21));
    let coach = skat_app::CoachSettings {
        show_figures: true,
        ..skat_app::CoachSettings::default()
    };
    same(&mut adapter, &mut session, Command::SetCoach(coach)).unwrap();
    to_card_choice(&mut adapter, &mut session);
    same(&mut adapter, &mut session, Command::StartGuided).unwrap();
    let permitted = view(&adapter);
    let (id, direct) = guided_ids(&adapter, &session);
    // Card memory is withheld until the worked answer is allowed.
    let open = skat_app::GuidedAction::Memory(true);
    assert!(
        both(
            &mut adapter,
            &mut session,
            Command::Guided(id, open),
            Command::Guided(direct, open)
        )
        .is_err()
    );
    assert!(guided_view(&adapter).unwrap().memory.is_none());
    let reveal = skat_app::GuidedAction::Reveal;
    both(
        &mut adapter,
        &mut session,
        Command::Guided(id, reveal),
        Command::Guided(direct, reveal),
    )
    .unwrap();
    both(
        &mut adapter,
        &mut session,
        Command::Guided(id, open),
        Command::Guided(direct, open),
    )
    .unwrap();
    let memory = guided_view(&adapter)
        .unwrap()
        .memory
        .expect("memory opened");
    // Only cards the learner's own permitted view can know.
    assert!(permitted.hand.iter().all(|c| memory.own.contains(*c)));
    assert_eq!(memory.own.len(), permitted.hand.len());
    let wire = adapter.guided_json();
    assert!(!wire.contains("seed"));

    // Play the learner's own card; the model comparison is a separate request.
    let play = *permitted
        .legal_actions
        .iter()
        .find(|a| matches!(a, skat_app::Action::Play(_)))
        .unwrap();
    same(&mut adapter, &mut session, Command::Act(play)).unwrap();
    settle(&mut adapter, &mut session);
    if let Some(g) = guided_view(&adapter)
        && g.step == skat_app::GuidedStep::Observe
    {
        let (id, direct) = guided_ids(&adapter, &session);
        let answer = skat_app::GuidedAction::Answer(skat_app::GuidedAnswer::Void(false));
        both(
            &mut adapter,
            &mut session,
            Command::Guided(id, answer),
            Command::Guided(direct, answer),
        )
        .unwrap();
    }
    let (id, direct) = guided_ids(&adapter, &session);
    let figures = skat_app::GuidedAction::Figures;
    both(
        &mut adapter,
        &mut session,
        Command::Guided(id, figures),
        Command::Guided(direct, figures),
    )
    .unwrap();
    assert!(guided_view(&adapter).unwrap().figures_requested);
}
