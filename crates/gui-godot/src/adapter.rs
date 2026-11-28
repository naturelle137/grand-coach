// SPDX-License-Identifier: EUPL-1.2

//! Thin, Godot-free adapter around one [`skat_app::Session`].
//!
//! Everything crossing the boundary is the serde form of the `skat-app` plain
//! data types, so GDScript only moves text. Opponent moves are computed on a
//! worker thread; each result carries the generation of the session it was
//! computed for, so a restarted or dropped session never receives it.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use serde::Serialize;
use skat_app::{
    AppError, AppEvent, AttemptId, AttemptKind, Cancel, CoachResult, Command, GameRecording,
    LearningFileError, LearningHistory, OpponentMove, PlayerId, RecordingEnd, RecordingFileError,
    RetrievalEvent, Session, SessionConfig,
};

/// Structured failure at the boundary, serialised as `{"error": ...}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum AdapterError {
    /// Command or configuration text was not valid for its data type.
    Invalid(String),
    /// The configuration is not allowed (e.g. an unavailable engine).
    Config,
    /// Recording file failure.
    Recording(RecordingFileError),
    /// No completed game is available.
    NoRecording,
    /// `skat-app` rejected the command; the session is unchanged.
    App(AppError),
}

impl AdapterError {
    /// JSON text `{"error": ...}` handed to the frontend.
    pub fn to_json(&self) -> String {
        #[derive(Serialize)]
        struct Wrapper<'a> {
            error: &'a AdapterError,
        }
        serde_json::to_string(&Wrapper { error: self }).expect("plain data serialises")
    }
}

struct Computed {
    generation: u64,
    mv: OpponentMove,
}

struct CoachDone {
    generation: u64,
    result: CoachResult,
}

/// One session plus its asynchronous opponent computation.
pub struct Adapter {
    session: Session,
    generation: u64,
    cancel: Cancel,
    sender: Sender<Computed>,
    receiver: Receiver<Computed>,
    in_flight: bool,
    coach_sender: Sender<CoachDone>,
    coach_receiver: Receiver<CoachDone>,
    coach_in_flight: bool,
    learning: Learning,
    replay: Option<GameRecording>,
}

/// The learning history (D-70) and the attempt of the running series. The
/// history is saved only while its file was read cleanly: a damaged or
/// unsupported file is kept unchanged.
#[derive(Default)]
struct Learning {
    history: LearningHistory,
    attempt: Option<AttemptId>,
    path: Option<PathBuf>,
    damaged: bool,
}

impl Learning {
    /// Stores the retrieval events among `events`, starting the series
    /// attempt on the first one.
    fn record(&mut self, events: &[AppEvent]) {
        let retrieval: Vec<RetrievalEvent> = events
            .iter()
            .filter_map(|e| match e {
                AppEvent::Retrieval(r) => Some(*r),
                _ => None,
            })
            .collect();
        if retrieval.is_empty() {
            return;
        }
        let attempt = match self.attempt {
            Some(id) => id,
            None => match self.history.begin_attempt(AttemptKind::Series) {
                Ok(id) => *self.attempt.insert(id),
                Err(_) => return,
            },
        };
        if self
            .history
            .record(attempt, &retrieval)
            .is_ok_and(|r| r.added > 0)
            && !self.damaged
            && let Some(path) = &self.path
        {
            // A failed write keeps the in-memory history; the next event retries.
            let _ = self.history.save_local(path);
        }
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

fn parse_config(json: &str) -> Result<Session, AdapterError> {
    let config: SessionConfig =
        serde_json::from_str(json).map_err(|e| AdapterError::Invalid(e.to_string()))?;
    Session::try_new(config).map_err(|_| AdapterError::Config)
}

impl Adapter {
    /// Starts a session from a serialised [`SessionConfig`].
    pub fn new(config_json: &str) -> Result<Adapter, AdapterError> {
        let session = parse_config(config_json)?;
        let (sender, receiver) = channel();
        let (coach_sender, coach_receiver) = channel();
        Ok(Adapter {
            session,
            generation: 0,
            cancel: Cancel::new(),
            sender,
            receiver,
            in_flight: false,
            coach_sender,
            coach_receiver,
            coach_in_flight: false,
            learning: Learning::default(),
            replay: None,
        })
    }

    /// Replaces the session. Running work is cancelled and its result can no
    /// longer reach the new session. On error the old session stays.
    pub fn restart(&mut self, config_json: &str) -> Result<(), AdapterError> {
        let session = parse_config(config_json)?;
        self.cancel.cancel();
        self.cancel = Cancel::new();
        self.generation += 1;
        let (sender, receiver) = channel();
        self.sender = sender;
        self.receiver = receiver;
        self.in_flight = false;
        let (coach_sender, coach_receiver) = channel();
        self.coach_sender = coach_sender;
        self.coach_receiver = coach_receiver;
        self.coach_in_flight = false;
        self.session = session;
        self.learning.attempt = None;
        Ok(())
    }

    /// Keeps the learning history in the file at `path`: a valid file is
    /// loaded, a missing one is created by the first event, a damaged one
    /// stays unchanged and the history then lives in memory only.
    pub fn use_learning_file(&mut self, path: PathBuf) {
        let mut learning = Learning::default();
        match LearningHistory::open_local(&path) {
            Ok(history) => learning.history = history,
            Err(error) => {
                learning.history.invalidate_exposure();
                learning.damaged = !matches!(error, LearningFileError::Missing);
            }
        }
        learning.path = Some(path);
        self.learning = learning;
    }

    /// Number of recorded retrieval events (question, answer, skip, help).
    pub fn learning_events(&self) -> usize {
        self.learning.history.events().len()
    }

    /// Generation of the current session; changes on every restart.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The human's permitted view as JSON (`TableView`).
    pub fn view_json(&self) -> String {
        serde_json::to_string(&self.session.view()).expect("plain data serialises")
    }

    /// Resource keys of the figure catalogue, indexed by `FigureId`, so a
    /// frontend resolves the view's cast to names and portraits (D-72).
    pub fn figure_keys_json() -> String {
        let keys: Vec<&str> = skat_app::FIGURES.iter().map(|f| f.key).collect();
        serde_json::to_string(&keys).expect("plain data serialises")
    }

    /// Executes a serialised [`Command`]; returns the events as JSON.
    pub fn execute_json(&mut self, command_json: &str) -> Result<String, AdapterError> {
        let command: Command =
            serde_json::from_str(command_json).map_err(|e| AdapterError::Invalid(e.to_string()))?;
        let events = self.session.execute(command).map_err(AdapterError::App)?;
        self.learning.record(&events);
        Ok(serde_json::to_string(&events).expect("plain data serialises"))
    }

    /// Saves the current completed game without overwriting an existing file.
    pub fn save_recording(&self, path: &std::path::Path) -> Result<(), AdapterError> {
        let recording = self
            .session
            .export_current_recording_privileged()
            .filter(|r| r.end != RecordingEnd::InProgress)
            .ok_or(AdapterError::NoRecording)?;
        recording
            .save_local_new(path)
            .map_err(AdapterError::Recording)
    }

    /// Opens and validates a recording before replacing the current replay.
    pub fn open_recording(&mut self, path: &std::path::Path) -> Result<(), AdapterError> {
        let recording = GameRecording::open_local(path).map_err(AdapterError::Recording)?;
        self.replay = Some(recording);
        Ok(())
    }

    /// Historical view of the human at an action boundary; no concealed deal crosses the wire.
    pub fn replay_json(&self, index: usize) -> Result<String, AdapterError> {
        let recording = self.replay.as_ref().ok_or(AdapterError::NoRecording)?;
        let position = recording
            .reconstruct_privileged(index)
            .map_err(|e| AdapterError::Recording(RecordingFileError::InvalidRecording(e)))?;
        let view = &position.state.seats[PlayerId::HUMAN.seat(recording.context.dealer).index()];
        Ok(
            serde_json::json!({"index": index, "total": recording.actions.len(), "view": view,
            "hand": view.hand.iter().collect::<Vec<_>>()})
            .to_string(),
        )
    }

    /// Starts computing the pending opponent move on a worker thread.
    /// Returns `false` if no opponent is to move or work is already running.
    pub fn start_opponent(&mut self) -> bool {
        if self.in_flight {
            return false;
        }
        let Some(job) = self.session.opponent_job() else {
            return false;
        };
        let (generation, cancel, sender) =
            (self.generation, self.cancel.clone(), self.sender.clone());
        self.in_flight = true;
        thread::spawn(move || {
            let mv = job.run(Some(&cancel));
            // The receiver is gone after a restart or drop: nothing to do.
            let _ = sender.send(Computed { generation, mv });
        });
        true
    }

    /// Applies a finished opponent move, if any. Stale results are dropped.
    /// Returns the events as JSON, or `None` if nothing was applied.
    pub fn poll_opponent(&mut self) -> Result<Option<String>, AdapterError> {
        let Ok(done) = self.receiver.try_recv() else {
            return Ok(None);
        };
        self.in_flight = false;
        if done.generation != self.generation {
            return Ok(None);
        }
        match self.session.finish_opponent(done.mv) {
            Ok(events) => {
                self.learning.record(&events);
                Ok(Some(
                    serde_json::to_string(&events).expect("plain data serialises"),
                ))
            }
            Err(AppError::StaleOpponentMove | AppError::NoOpponentToMove) => Ok(None),
            Err(e) => Err(AdapterError::App(e)),
        }
    }

    /// Starts the coach analysis due for the human's current decision on a
    /// worker thread (permitted view only, D-23). Review analyses of past
    /// moves are not run. Returns `false` if none is due or one is running.
    pub fn start_coach(&mut self) -> bool {
        if self.coach_in_flight {
            return false;
        }
        let Some(job) = self.session.coach_job().filter(|j| !j.is_review()) else {
            return false;
        };
        let (generation, cancel, sender) = (
            self.generation,
            self.cancel.clone(),
            self.coach_sender.clone(),
        );
        self.coach_in_flight = true;
        thread::spawn(move || {
            let result = job.run(Some(&cancel));
            let _ = sender.send(CoachDone { generation, result });
        });
        true
    }

    /// Applies a finished coach analysis, if any; the action a warning held
    /// back is then played or held with its warning. Stale results are
    /// dropped. Returns the events as JSON, or `None` if nothing was applied.
    pub fn poll_coach(&mut self) -> Result<Option<String>, AdapterError> {
        let Ok(done) = self.coach_receiver.try_recv() else {
            return Ok(None);
        };
        self.coach_in_flight = false;
        if done.generation != self.generation {
            return Ok(None);
        }
        match self.session.finish_coach(done.result) {
            Ok(events) => {
                self.learning.record(&events);
                Ok(Some(
                    serde_json::to_string(&events).expect("plain data serialises"),
                ))
            }
            Err(AppError::StaleCoachResult) => Ok(None),
            Err(e) => Err(AdapterError::App(e)),
        }
    }

    /// `true` while a coach analysis has not been collected.
    pub fn coach_busy(&self) -> bool {
        self.coach_in_flight
    }

    /// `true` while an opponent computation has not been collected.
    pub fn busy(&self) -> bool {
        self.in_flight
    }
}
