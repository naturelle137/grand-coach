// SPDX-License-Identifier: EUPL-1.2

//! The egui application: draws the [`TableView`] of `skat-app` and turns
//! clicks into [`Command`]s. No game logic here.

use eframe::egui::{self, Color32, RichText, Vec2};
use std::sync::mpsc::{Receiver, TryRecvError};

use skat_app::{
    Action, Advice, Announcement, AppError, AppEvent, AssistSettings, Cancel, Card, CardSet,
    CoachBasis, CoachJob, CoachLevel, CoachResult, CoachSettings, Command, CountingState, Event,
    GameRecording, GameType, Level, OpenCards, OpponentMove, Opponents, Pending, Phase, PlayedCard,
    PlayerId, SeriesLength, Session, SessionConfig, Suit, TableDisplay, TableView, reizwerte,
};

use skat_app::coach_model::{Concept, Stage};

use crate::assist;
use crate::cards::{self, Deck, Theme};
use crate::coach::{self, Style};
use crate::i18n::{Faces, I18n, Language};
use crate::style;
use crate::text;

const CARD: Vec2 = Vec2::new(78.0, 117.0);
const SMALL_CARD: Vec2 = Vec2::new(44.0, 66.0);
/// Closed opponent cards and the own Skat in a small window.
const COMPACT_CARD: Vec2 = Vec2::new(32.0, 48.0);
/// Widest hand card in a small window, so the trick stays in view.
const COMPACT_HAND_CARD: f32 = 66.0;
/// Cards of a debriefing moment: ten fit the coach panel's width.
const MOMENT_CARD: Vec2 = Vec2::new(30.0, 45.0);
/// Gap between the cards of the trick triangle.
const TRICK_GAP: f32 = 8.0;
/// Height of the player name below a trick card.
const TRICK_LABEL_HEIGHT: f32 = 20.0;
const TABLE_GREEN: Color32 = Color32::from_rgb(34, 102, 58);
const TABLE_RED: Color32 = Color32::from_rgb(110, 22, 26);
const TABLE_BLUE: Color32 = Color32::from_rgb(30, 74, 140);
/// Side of an opponent's caricature on the table (D-72).
const PORTRAIT: f32 = 56.0;
/// Narrowest hand card before the hand wraps into a second row.
const MIN_HAND_CARD: f32 = 52.0;
/// From this window width on, the hand sits in the centre between coach
/// and side panel; below it spans the window, so every card and action
/// stays reachable down to 960 px (issue #134).
const WIDE_LAYOUT: f32 = 1200.0;

/// Size of the trick triangle: left and right card on top, the human's card
/// below between them.
fn trick_area() -> Vec2 {
    Vec2::new(
        3.0 * CARD.x + 2.0 * TRICK_GAP,
        1.5 * CARD.y + TRICK_LABEL_HEIGHT,
    )
}

/// Offset of a player's card in the trick triangle, towards that player's
/// seat at the table: left opponent left, right opponent right, human below.
fn trick_slot(player: PlayerId) -> Vec2 {
    match player.0 {
        0 => Vec2::new(CARD.x + TRICK_GAP, 0.5 * CARD.y),
        1 => Vec2::ZERO,
        _ => Vec2::new(2.0 * (CARD.x + TRICK_GAP), 0.0),
    }
}

/// Draft of the "new series" dialog.
#[derive(Clone, Debug)]
struct NewSeries {
    free: bool,
    games: u16,
    kontra_re: bool,
    mode: Mode,
    levels: [Level; 2],
    /// Open display (D-68); implied in God Mode.
    open: bool,
    seed: String,
}

/// Opponent choice in the "new series" dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Levels,
    Goldfish,
    Mephisto,
    God,
    Neural,
    Experimental,
}

/// An opponent decision computed on a background thread. Dropping it
/// cancels the computation (new series).
struct Thinking {
    result: Receiver<OpponentMove>,
    cancel: Cancel,
}

impl Drop for Thinking {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// A coach analysis computed on a background thread. Dropping it cancels
/// the computation (a new game or series started, or the current decision
/// goes before a review).
struct CoachThinking {
    job: CoachJob,
    result: Receiver<CoachResult>,
    cancel: Cancel,
}

impl Drop for CoachThinking {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Most attempts per entered practice before the learner must return
/// (bounds repeated starts and their cancelled background work).
pub const MAX_PRACTICE_ATTEMPTS: u32 = 50;

/// The table state of one session: the session and everything the window
/// keeps for it, including its outstanding background work.
struct Parked {
    session: Session,
    selected: CardSet,
    draft: Announcement,
    jump_bid: Option<u16>,
    error: Option<AppError>,
    next_move_at: f64,
    thinking: Option<Thinking>,
    ready: Option<OpponentMove>,
    show_trick_until: f64,
    new_series: Option<NewSeries>,
    coach_thinking: Option<CoachThinking>,
    quiz_value: u8,
    learning_attempt: Option<skat_app::AttemptId>,
}

/// A running practice attempt (M7): the live series waits, untouched and
/// with its outstanding opponent and coach work, until the learner returns.
struct Practice {
    live: Parked,
    source: GameRecording,
    attempt: u32,
}

/// Practice lifecycle request from this frame, applied after drawing.
enum PracticeRequest {
    Start(GameRecording),
    Restart,
    Return,
}

/// The Grand-Coach window.
pub struct GrandCoachApp {
    session: Session,
    recording: crate::recording::RecordingUi,
    i18n: I18n,
    deck: Deck,
    /// Cards chosen for Drücken.
    selected: CardSet,
    /// Game chosen in the announcement form.
    draft: Announcement,
    /// Value chosen in the jump-bid box.
    jump_bid: Option<u16>,
    error: Option<AppError>,
    /// Seconds between two opponent moves; tests set 0.
    pub opponent_delay: f64,
    next_move_at: f64,
    /// Opponent decision being computed.
    thinking: Option<Thinking>,
    /// Computed opponent move waiting for its turn to be shown.
    ready: Option<OpponentMove>,
    /// Until when the last completed trick stays in the middle.
    show_trick_until: f64,
    new_series: Option<NewSeries>,
    /// Developer option: show the seed in the status line and let the
    /// "new series" dialog set it.
    show_seed: bool,
    /// Coach analysis being computed.
    coach_thinking: Option<CoachThinking>,
    /// Number chosen for the open counting question.
    quiz_value: u8,
    /// God Mode: show every card face up (on by default); off, the
    /// opponents' cards and the Skat show their backs.
    see_all: bool,
    /// The isolated practice attempt, if one runs; `session` is then the
    /// practice session and the live table waits inside.
    practice: Option<Practice>,
    practice_request: Option<PracticeRequest>,
    /// The local learning history and its window (D-70).
    learning: crate::learning::LearningUi,
    /// Learning-history attempt of `session`; assigned on its first
    /// retrieval event, so every series and practice attempt has its own.
    learning_attempt: Option<skat_app::AttemptId>,
    /// The focused counting training screen (D-71); while open, the live
    /// table waits undriven and unchanged.
    training: crate::training::TrainingUi,
    /// The game in which the concealed Augen were revealed by holding the
    /// button; `None` while it is not held (issue #134).
    augen_hold: Option<GameKey>,
}

/// Identifies the shown game: a new game, series or practice attempt
/// conceals the Augen again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GameKey {
    seed: u64,
    number: u32,
    practice: Option<u32>,
}

impl GrandCoachApp {
    /// Creates the app, installs the SVG image loader and the light theme.
    pub fn new(ctx: &egui::Context, config: SessionConfig) -> GrandCoachApp {
        egui_extras::install_image_loaders(ctx);
        style::apply(ctx);
        GrandCoachApp {
            session: Session::new(config),
            recording: Default::default(),
            i18n: I18n::new(Language::De),
            deck: Deck::French4,
            selected: CardSet::EMPTY,
            draft: Announcement::plain(GameType::Grand),
            jump_bid: None,
            error: None,
            opponent_delay: 0.8,
            next_move_at: 0.0,
            thinking: None,
            ready: None,
            show_trick_until: 0.0,
            new_series: None,
            show_seed: false,
            coach_thinking: None,
            quiz_value: 0,
            see_all: true,
            practice: None,
            practice_request: None,
            learning: Default::default(),
            learning_attempt: None,
            training: Default::default(),
            augen_hold: None,
        }
    }

    /// Opens the focused counting training (topic choice); the live table
    /// waits unchanged until [`Self::exit_training`].
    pub fn open_training(&mut self) {
        if !self.recording.active() && self.new_series.is_none() {
            self.training.enter();
        }
    }

    /// Leaves the training and returns to the waiting live table.
    pub fn exit_training(&mut self) {
        self.training.exit();
    }

    /// Whether the training screen is open.
    pub fn training_active(&self) -> bool {
        self.training.active()
    }

    /// The running catalogue task (tests and diagnostics).
    pub fn focus_view(&self) -> Option<skat_app::FocusView> {
        self.training.focus_view()
    }

    /// The running training task (tests and diagnostics).
    pub fn training_view(&self) -> Option<skat_app::TrainingView> {
        self.training.view()
    }

    /// Whether a practice attempt is running.
    pub fn practicing(&self) -> bool {
        self.practice.is_some()
    }

    /// The live series, also while a practice attempt runs (read-only, for
    /// tests and diagnostics).
    pub fn live_session(&self) -> &Session {
        self.practice
            .as_ref()
            .map_or(&self.session, |p| &p.live.session)
    }

    /// Number of the running practice attempt, from 1.
    pub fn practice_attempt(&self) -> Option<u32> {
        self.practice.as_ref().map(|p| p.attempt)
    }

    /// Installs `session` with a fresh table state and returns the previous
    /// one, including its background work (kept running, not polled).
    fn park(&mut self, session: Session) -> Parked {
        use std::mem::replace;
        Parked {
            session: replace(&mut self.session, session),
            selected: replace(&mut self.selected, CardSet::EMPTY),
            draft: replace(&mut self.draft, Announcement::plain(GameType::Grand)),
            jump_bid: self.jump_bid.take(),
            error: self.error.take(),
            next_move_at: replace(&mut self.next_move_at, 0.0),
            thinking: self.thinking.take(),
            ready: self.ready.take(),
            show_trick_until: replace(&mut self.show_trick_until, 0.0),
            new_series: self.new_series.take(),
            coach_thinking: self.coach_thinking.take(),
            quiz_value: replace(&mut self.quiz_value, 0),
            learning_attempt: self.learning_attempt.take(),
        }
    }

    /// Restores a parked table state; the replaced one is dropped, which
    /// cancels its background work and disconnects its channels.
    fn unpark(&mut self, parked: Parked) {
        let current = self.park(parked.session);
        self.selected = parked.selected;
        self.draft = parked.draft;
        self.jump_bid = parked.jump_bid;
        self.error = parked.error;
        self.next_move_at = parked.next_move_at;
        self.thinking = parked.thinking;
        self.ready = parked.ready;
        self.show_trick_until = parked.show_trick_until;
        self.new_series = parked.new_series;
        self.coach_thinking = parked.coach_thinking;
        self.quiz_value = parked.quiz_value;
        self.learning_attempt = parked.learning_attempt;
        drop(current);
    }

    /// Starts a practice attempt of `recording` before bidding. The whole
    /// recording is validated first; the live session is only parked, never
    /// changed, and a rejected recording leaves everything as it was.
    fn start_practice(&mut self, recording: GameRecording) -> Result<(), skat_app::RecordingError> {
        if self.practice.is_some() {
            return Ok(());
        }
        let session = self.session.start_practice(&recording)?;
        let live = self.park(session);
        self.practice = Some(Practice {
            live,
            source: recording,
            attempt: 1,
        });
        self.recording.close();
        Ok(())
    }

    /// A fresh attempt of the same deal; the previous attempt and its
    /// background work are dropped. Keeps the practice's own settings.
    fn restart_practice(&mut self) {
        let Some(practice) = &self.practice else {
            return;
        };
        if practice.attempt >= MAX_PRACTICE_ATTEMPTS {
            return;
        }
        let session = self
            .session
            .start_practice(&practice.source)
            .expect("validated when the practice started");
        drop(self.park(session));
        if let Some(practice) = &mut self.practice {
            practice.attempt += 1;
        }
    }

    /// Ends the attempt (finished or not) and returns to the live series
    /// exactly as it was left.
    fn return_to_series(&mut self) {
        if let Some(practice) = self.practice.take() {
            self.unpark(practice.live);
        }
    }

    fn apply_practice_request(&mut self) {
        match self.practice_request.take() {
            Some(PracticeRequest::Start(recording)) => {
                if self.start_practice(recording).is_err() {
                    self.recording.practice_failed();
                }
            }
            Some(PracticeRequest::Restart) => self.restart_practice(),
            Some(PracticeRequest::Return) => self.return_to_series(),
            None => {}
        }
    }

    /// The practice banner with its lifecycle controls.
    fn practice_bar(&mut self, ui: &mut egui::Ui, view: &TableView) {
        let Some(practice) = &self.practice else {
            return;
        };
        let attempt = practice.attempt;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(self.i18n.tf(
                    "practice-banner",
                    &[
                        ("number", view.game_number.into()),
                        ("attempt", attempt.into()),
                    ],
                ))
                .color(Color32::from_rgb(140, 70, 0))
                .strong(),
            );
            ui.label(self.t("practice-isolation"));
            ui.separator();
            if ui.button(self.t("practice-return")).clicked() {
                self.practice_request = Some(PracticeRequest::Return);
            }
            let more = attempt < MAX_PRACTICE_ATTEMPTS;
            if ui
                .add_enabled(more, egui::Button::new(self.t("practice-restart")))
                .on_disabled_hover_text(self.t("practice-limit"))
                .clicked()
            {
                self.practice_request = Some(PracticeRequest::Restart);
            }
        });
    }

    /// Sets the work budget of coach analyses (tests; default interactive).
    pub fn set_coach_budget(&mut self, budget: skat_app::Budget) {
        self.session.set_coach_budget(budget);
    }

    /// Changes the coach settings; the game keeps running.
    pub fn set_coach(&mut self, settings: CoachSettings) {
        self.send(Command::SetCoach(settings), 0.0);
    }

    /// Changes the assistance settings (counting aid, quiz); the game
    /// keeps running.
    pub fn set_assist(&mut self, settings: AssistSettings) {
        self.send(Command::SetAssist(settings), 0.0);
    }

    /// `true` while the recording window computes a replay comment
    /// (tests and diagnostics).
    pub fn replay_computing(&self) -> bool {
        self.recording.replay_computing()
    }

    /// `true` while the recording window runs its review (tests and
    /// diagnostics).
    pub fn review_running(&self) -> bool {
        self.recording.review_running()
    }

    /// The replay coach of the open recording window (tests, screenshots).
    pub fn replay_coach_mut(&mut self) -> Option<&mut skat_app::ReplayCoach> {
        self.recording.replay_coach_mut()
    }

    /// The running session (read-only, for tests and diagnostics).
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Switches the UI language; the game keeps running.
    pub fn set_language(&mut self, language: Language) {
        self.i18n.language = language;
    }

    /// Switches the card faces; the game keeps running.
    pub fn set_deck(&mut self, deck: Deck) {
        self.deck = deck;
    }

    /// God Mode: shows every card face up, or the backs (on by default).
    pub fn set_see_all(&mut self, see_all: bool) {
        self.see_all = see_all;
    }

    /// Shows or hides the seed (developer option, off by default).
    pub fn set_show_seed(&mut self, show: bool) {
        self.show_seed = show;
    }

    /// Opens the "new series" dialog with the current settings and seed, so
    /// starting it unchanged plays exactly the current series. The window
    /// does this at launch so the series settings are visible (issue #25).
    pub fn show_new_series(&mut self) {
        self.open_new_series_with(self.session.config().seed);
    }

    /// Whether the "new series" dialog is open.
    pub fn new_series_open(&self) -> bool {
        self.new_series.is_some()
    }

    /// Sends a command to the session and keeps a rejection for display.
    fn send(&mut self, command: Command, now: f64) {
        let result = self.session.execute(command);
        self.handle(result, now);
    }

    fn handle(&mut self, result: Result<Vec<AppEvent>, AppError>, now: f64) {
        match result {
            Ok(events) => {
                self.error = None;
                self.record_learning(&events);
                if events
                    .iter()
                    .any(|e| matches!(e, AppEvent::GameStarted { .. }))
                {
                    // A new game: the analyses of the last one are dropped.
                    self.coach_thinking = None;
                }
                let trick_done = events.iter().any(|e| {
                    matches!(
                        e,
                        AppEvent::Game {
                            event: Event::TrickWon { .. },
                            ..
                        }
                    )
                });
                let pause = if trick_done {
                    2.0 * self.opponent_delay
                } else {
                    self.opponent_delay
                };
                if trick_done {
                    self.show_trick_until = now + pause;
                }
                self.next_move_at = now + pause;
                if events
                    .iter()
                    .any(|e| matches!(e, AppEvent::GameStarted { .. }))
                {
                    self.selected = CardSet::EMPTY;
                    self.jump_bid = None;
                    self.draft = Announcement::plain(GameType::Grand);
                }
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Records the retrieval events of the running session or practice
    /// attempt in the learning history (D-70).
    fn record_learning(&mut self, events: &[AppEvent]) {
        let retrieval: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                AppEvent::Retrieval(r) => Some(*r),
                _ => None,
            })
            .collect();
        let kind = match &self.practice {
            Some(p) => skat_app::AttemptKind::Practice { attempt: p.attempt },
            None => skat_app::AttemptKind::Series,
        };
        self.learning
            .record(&mut self.learning_attempt, kind, &retrieval);
    }

    /// Keeps the learning history in the file at `path` (D-70): a valid
    /// file is loaded, a missing one created on the first event, a damaged
    /// or unsupported one left unchanged and reported in the window.
    pub fn use_learning_file(&mut self, path: std::path::PathBuf) {
        self.learning.use_file(path);
    }

    /// The learning history (read-only, for tests and diagnostics).
    pub fn learning_history(&self) -> &skat_app::LearningHistory {
        self.learning.history()
    }

    /// The learning history file, if one is used.
    pub fn learning_file(&self) -> Option<&std::path::Path> {
        self.learning.path()
    }

    /// Opens the learning history window.
    pub fn show_learning(&mut self) {
        self.learning.show();
    }

    /// Whether the learning history window is open.
    pub fn learning_open(&self) -> bool {
        self.learning.is_open()
    }

    fn act(&mut self, action: Action, now: f64) {
        self.send(Command::Act(action), now);
    }

    /// Lets the opponents move at a readable pace. Each decision is
    /// computed on a background thread, so the window stays responsive
    /// while an opponent thinks; it starts at once and overlaps the pause.
    fn drive_opponents(&mut self, ctx: &egui::Context, now: f64) {
        if !self.session.opponent_to_move() {
            self.thinking = None;
            self.ready = None;
            return;
        }
        if self.thinking.is_none()
            && self.ready.is_none()
            && let Some(job) = self.session.opponent_job()
        {
            let (sender, result) = std::sync::mpsc::channel();
            let cancel = Cancel::new();
            let (stop, repaint) = (cancel.clone(), ctx.clone());
            std::thread::spawn(move || {
                // The receiver is gone if the series was replaced meanwhile.
                let _ = sender.send(job.run(Some(&stop)));
                repaint.request_repaint();
            });
            self.thinking = Some(Thinking { result, cancel });
        }
        if let Some(thinking) = &self.thinking {
            match thinking.result.try_recv() {
                Ok(mv) => {
                    self.ready = Some(mv);
                    self.thinking = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.thinking = None,
            }
        }
        if now >= self.next_move_at
            && let Some(mv) = self.ready.take()
        {
            let result = self.session.finish_opponent(mv);
            self.handle(result, now);
        }
        let wait = (self.next_move_at - now).clamp(0.0, 0.05);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
    }

    /// Runs the coach analysis the session asks for on a background thread
    /// and hands the result back; the window stays responsive meanwhile.
    /// A decision played meanwhile keeps its analysis running: it backs
    /// the note and the debriefing. The current decision goes first: an
    /// analysis of an earlier one is cancelled and rerun afterwards.
    fn drive_coach(&mut self, ctx: &egui::Context, now: f64) {
        if let Some(thinking) = &self.coach_thinking
            && let Some(next) = self.session.coach_job()
            && !next.is_review()
            && !next.same_decision(&thinking.job)
        {
            self.coach_thinking = None;
        }
        if self.coach_thinking.is_none()
            && let Some(job) = self.session.coach_job()
        {
            let (sender, result) = std::sync::mpsc::channel();
            let cancel = Cancel::new();
            let (stop, repaint) = (cancel.clone(), ctx.clone());
            let run = job.clone();
            std::thread::spawn(move || {
                // The receiver is gone if the analysis was dropped.
                let final_result = run.run_with_preview(Some(&stop), |preview| {
                    let _ = sender.send(preview);
                    repaint.request_repaint();
                });
                let _ = sender.send(final_result);
                repaint.request_repaint();
            });
            self.coach_thinking = Some(CoachThinking {
                job,
                result,
                cancel,
            });
        }
        if let Some(thinking) = &self.coach_thinking {
            match thinking.result.try_recv() {
                Ok(result) => {
                    if !result.is_preview() {
                        self.coach_thinking = None;
                    } else {
                        ctx.request_repaint();
                    }
                    match self.session.finish_coach(result) {
                        Ok(events) => self.handle(Ok(events), now),
                        // Computed for an earlier decision: ignore it.
                        Err(AppError::StaleCoachResult) => {}
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.coach_thinking = None,
            }
        }
    }

    fn t(&self, id: &str) -> String {
        self.i18n.t(id)
    }

    fn player(&self, player: PlayerId) -> String {
        text::player(&self.i18n, player)
    }

    // ----------------------------------------------------------------------
    // Top: menu and status.

    fn menu_bar(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(self.t("menu-game"), |ui| {
                // A practice attempt has nothing to save and starts nothing.
                let live = self.practice.is_none();
                if ui
                    .add_enabled(live, egui::Button::new(self.t("record-save")))
                    .clicked()
                {
                    self.recording.show_dialog(true, &self.session);
                    ui.close();
                }
                if ui
                    .add_enabled(live, egui::Button::new(self.t("record-open")))
                    .clicked()
                {
                    self.recording.show_dialog(false, &self.session);
                    ui.close();
                }
                if ui.button(self.t("learning-open")).clicked() {
                    self.learning.show();
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.recording.active() && self.new_series.is_none(),
                        egui::Button::new(self.t("training-open")),
                    )
                    .clicked()
                {
                    self.open_training();
                    ui.close();
                }
                ui.separator();
                ui.add_enabled_ui(live && !self.recording.active(), |ui| {
                    if ui.button(self.t("menu-new-series")).clicked() {
                        self.open_new_series();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            view.can_end_series,
                            egui::Button::new(self.t("menu-end-series")),
                        )
                        .clicked()
                    {
                        self.send(Command::EndSeries, now);
                        ui.close();
                    }
                });
            });
            ui.add_enabled_ui(!self.recording.active(), |ui| {
                ui.menu_button(self.t("menu-settings"), |ui| {
                    self.settings_menu(ui);
                });
                if view.open_cards.is_some() {
                    ui.separator();
                    let label = self.t("option-see-all");
                    ui.checkbox(&mut self.see_all, label)
                        .on_hover_text(self.t("option-see-all-description"));
                }
            });
        });
    }

    fn settings_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(self.t("setting-language")).strong());
        for language in Language::ALL {
            let id = match language {
                Language::De => "language-de",
                Language::En => "language-en",
            };
            let label = self.t(id);
            ui.radio_value(&mut self.i18n.language, language, label);
        }
        ui.separator();
        ui.label(RichText::new(self.t("setting-deck")).strong());
        for deck in Deck::ALL {
            let label = self.t(deck.message());
            ui.radio_value(&mut self.deck, deck, label);
        }
        ui.separator();
        self.coach_settings(ui);
        ui.separator();
        self.assist_settings(ui);
        ui.separator();
        ui.label(RichText::new(self.t("setting-developer")).strong());
        let label = self.t("setting-show-seed");
        ui.checkbox(&mut self.show_seed, label)
            .on_hover_text(self.t("seed-description"));
    }

    fn coach_settings(&mut self, ui: &mut egui::Ui) {
        let mut settings = self.session.coach_settings();
        ui.label(RichText::new(self.t("menu-coach")).strong());
        ui.label(self.t("setting-coach-level"));
        for level in CoachLevel::ALL {
            let label = self.t(coach_level_message(level));
            ui.radio_value(&mut settings.level, level, label);
        }
        let label = self.t("setting-coach-warnings");
        ui.checkbox(&mut settings.warnings, label)
            .on_hover_text(self.t("setting-coach-warnings-description"));
        let label = self.t("setting-coach-figures");
        ui.checkbox(&mut settings.show_figures, label);
        // One optional focus, chosen by the player only.
        ui.label(self.t("setting-coach-focus"))
            .on_hover_text(self.t("setting-coach-focus-description"));
        let focuses = std::iter::once(None).chain(Concept::ALL.into_iter().map(Some));
        for focus in focuses {
            let label = coach::focus(&self.i18n, focus);
            ui.radio_value(&mut settings.focus, focus, label);
        }
        if settings != self.session.coach_settings() {
            self.set_coach(settings);
        }
    }

    fn assist_settings(&mut self, ui: &mut egui::Ui) {
        let mut settings = self.session.view().assist.settings;
        ui.label(RichText::new(self.t("menu-assist")).strong());
        let label = self.t("setting-aid");
        ui.checkbox(&mut settings.counting_aid, label)
            .on_hover_text(self.t("setting-aid-description"));
        let label = self.t("setting-quiz");
        ui.checkbox(&mut settings.quiz, label)
            .on_hover_text(self.t("setting-quiz-description"));
        ui.checkbox(&mut settings.retrieval, self.t("setting-retrieval"));
        ui.label(self.t("setting-retrieval-level"));
        for (level, key) in [
            (skat_app::AssistanceLevel::VisibleAid, "retrieval-visible"),
            (
                skat_app::AssistanceLevel::RevealOnRequest,
                "retrieval-request",
            ),
            (
                skat_app::AssistanceLevel::AnswerBeforeReveal,
                "retrieval-answer",
            ),
        ] {
            ui.radio_value(&mut settings.assistance, level, self.t(key));
        }
        if settings != self.session.view().assist.settings {
            self.set_assist(settings);
        }
    }

    /// The counting aid and the result of the last counting question.
    fn aid_section(&self, ui: &mut egui::Ui, view: &TableView) {
        let i = &self.i18n;
        let assist = &view.assist;
        if assist.counting == CountingState::Off && assist.quiz.feedback.is_none() {
            return;
        }
        ui.separator();
        match &assist.counting {
            CountingState::Off => {}
            CountingState::Withheld => {
                ui.heading(self.t("aid-title"));
                ui.label(
                    RichText::new(self.t(if assist.settings.retrieval {
                        "retrieval-withheld"
                    } else {
                        "aid-withheld"
                    }))
                    .weak(),
                );
            }
            CountingState::Shown(aid) => {
                ui.heading(self.t("aid-title"));
                ui.label(RichText::new(self.t("aid-exact")).weak());
                for line in assist::aid_lines(i, aid) {
                    ui.label(line);
                }
                let voids = assist::void_lines(i, aid);
                ui.label(RichText::new(self.t("aid-voids")).strong());
                if voids.is_empty() {
                    ui.label(self.t("aid-void-none"));
                }
                for line in voids {
                    ui.label(line);
                }
            }
        }
        if let Some(feedback) = &assist.quiz.feedback {
            ui.label(RichText::new(self.t("quiz-feedback-title")).strong());
            for line in assist::feedback_lines(i, feedback) {
                ui.label(line);
            }
        }
    }

    /// The open counting question: answer, skip or switch the quiz off.
    /// The move waits meanwhile.
    fn quiz_window(&mut self, ctx: &egui::Context, view: &TableView, now: f64) {
        let Some(kind) = view.assist.quiz.question else {
            self.quiz_value = 0;
            return;
        };
        let text = assist::question(&self.i18n, kind);
        let (mut command, mut delta) = (None, 0i16);
        egui::Window::new(self.t("quiz-title"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_width(420.0)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(text);
                ui.label(
                    RichText::new(self.t(if view.assist.settings.retrieval {
                        "retrieval-hint"
                    } else {
                        "quiz-hint"
                    }))
                    .weak(),
                );
                if view.assist.settings.retrieval {
                    let key = match view.assist.settings.assistance {
                        skat_app::AssistanceLevel::VisibleAid => "retrieval-visible",
                        skat_app::AssistanceLevel::RevealOnRequest => "retrieval-request",
                        skat_app::AssistanceLevel::AnswerBeforeReveal => "retrieval-answer",
                    };
                    ui.label(self.t(key));
                }
                if let Some(derivation) = view.assist.quiz.help {
                    for line in assist::derivation_lines(&self.i18n, derivation) {
                        ui.label(line);
                    }
                    ui.label(self.t("retrieval-assisted"));
                } else if view.assist.settings.retrieval
                    && ui.button(self.t("retrieval-help")).clicked()
                {
                    command = Some(Command::RestoreQuizHelp);
                }
                let request = view.assist.settings.retrieval
                    && view.assist.settings.assistance
                        == skat_app::AssistanceLevel::RevealOnRequest;
                let enter_label = self.t("retrieval-enter");
                let mut numeric = |ui: &mut egui::Ui| {
                    ui.label(
                        RichText::new(
                            self.i18n
                                .tf("quiz-value", &[("value", self.quiz_value.into())]),
                        )
                        .strong(),
                    );
                    ui.horizontal(|ui| {
                        for (id, step) in [
                            ("quiz-minus-ten", -10),
                            ("quiz-minus-one", -1),
                            ("quiz-plus-one", 1),
                            ("quiz-plus-ten", 10),
                        ] {
                            if ui.button(self.t(id)).clicked() {
                                delta = step;
                            }
                        }
                    });
                    if ui.button(self.t("quiz-submit")).clicked() {
                        command = Some(Command::AnswerQuiz(self.quiz_value));
                    }
                };
                if request {
                    egui::CollapsingHeader::new(enter_label)
                        .id_salt(("retrieval_input", view.assist.quiz.id))
                        .show(ui, numeric);
                } else {
                    numeric(ui);
                }
                ui.horizontal(|ui| {
                    if ui.button(self.t("quiz-skip")).clicked() {
                        command = Some(Command::SkipQuiz);
                    }
                    if ui.button(self.t("quiz-off")).clicked() {
                        let mut settings = view.assist.settings;
                        settings.quiz = false;
                        command = Some(Command::SetAssist(settings));
                    }
                });
            });
        self.quiz_value = (i16::from(self.quiz_value) + delta).clamp(0, 120) as u8;
        if let Some(command) = command {
            if !matches!(command, Command::RestoreQuizHelp) {
                self.quiz_value = 0;
            }
            self.send(command, now);
        }
    }

    // ----------------------------------------------------------------------
    // Left: the coach.

    fn coach_visible(view: &TableView) -> bool {
        view.coach.settings.level != CoachLevel::Off || view.coach.pending.is_some()
    }

    fn coach_panel(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        if let Some(g) = self.session.guided() {
            if let Some(action) = crate::guided::panel(ui, &self.i18n, view, &g) {
                self.send(Command::Guided(g.id, action), now);
            }
            return;
        }
        if view.acting == Some(PlayerId::HUMAN)
            && view.coach.settings.level != CoachLevel::Off
            && view.assist.quiz.question.is_none()
            && ui.button(self.t("guided-start")).clicked()
        {
            self.send(Command::StartGuided, now);
            return;
        }
        let coach = &view.coach;
        let human = view.acting == Some(PlayerId::HUMAN);
        ui.heading(self.t("coach-title"));
        if view.open_cards.is_some() {
            // The table shows every card; say what the coach uses (D-68).
            let id = match view.coach_basis {
                CoachBasis::OwnView => "coach-basis-own-view-open",
                CoachBasis::RealDeal => "coach-basis-real-deal",
            };
            ui.label(RichText::new(self.t(id)).small().italics());
        }
        if coach.help_available && human {
            // One stage per click (D-49); the label names the next one.
            let label = coach.next_stage.map_or_else(
                || coach::stage_button(&self.i18n, Stage::Figures),
                |s| coach::stage_button(&self.i18n, s),
            );
            if ui
                .add_enabled(coach.next_stage.is_some(), egui::Button::new(label))
                .clicked()
            {
                self.send(Command::RequestAdvice, now);
            }
            // Help is optional: put it away until the next decision.
            if coach.skippable && ui.button(self.t("button-coach-skip")).clicked() {
                self.send(Command::SkipAdvice, now);
            }
        }
        if let Some(f) = coach.settings.focus {
            ui.label(
                RichText::new(self.i18n.tf(
                    "coach-focus-current",
                    &[("focus", coach::concept(&self.i18n, f).into())],
                ))
                .weak(),
            );
        }
        let view = &self.session.view();
        let coach = &view.coach;
        let i = &self.i18n;
        let figures = coach.settings.show_figures;
        ui.separator();
        let mut note: Vec<_> = coach
            .last_move
            .iter()
            .map(|last| coach::note(i, last))
            .collect();
        if let Some(l) = coach.last_move.as_ref().and_then(|m| m.lesson.as_ref()) {
            note.extend(coach::lesson(i, view, l));
        }
        let mut lines = Vec::new();
        let debriefing = coach
            .debriefing
            .as_ref()
            .map(|d| coach::debriefing(i, view, d, figures));
        match &coach.advice {
            Advice::None if coach.analysing && human => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(i.t("coach-status-analysing"));
                });
            }
            Advice::None if human => {
                ui.label(RichText::new(i.t("coach-status-idle")).weak());
            }
            Advice::None if coach.debriefing.is_some() => {}
            Advice::None => {
                ui.label(RichText::new(i.t("coach-status-waiting")).weak());
            }
            Advice::Computing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(i.t("coach-status-computing"));
                });
            }
            Advice::Ready(h) | Advice::Provisional(h) => {
                lines.extend(coach::hints(i, view, h, figures));
            }
            Advice::NoLesson => {
                ui.label(RichText::new(i.t("coach-status-no-lesson")).weak());
            }
            Advice::Unavailable(reason) => {
                ui.label(coach::unavailable(i, *reason));
            }
        }
        let details = i.t("coach-details");
        egui::ScrollArea::vertical().show(ui, |ui| {
            show_lines(ui, &note);
            if let Some(d) = debriefing {
                show_lines(ui, &d.lines);
                for (n, m) in d.moments.into_iter().enumerate() {
                    show_lines(ui, std::slice::from_ref(&m.title));
                    self.situation(ui, &m.situation);
                    // The public history before the decision, collapsed.
                    egui::CollapsingHeader::new(&m.situation.history_title)
                        .id_salt(("moment_history", view.game_number, n))
                        .show(ui, |ui| show_lines(ui, &m.situation.history));
                    show_lines(ui, &m.summary);
                    // The lesson's evidence, collapsed by default.
                    egui::CollapsingHeader::new(&m.lesson_title)
                        .id_salt(("moment_lesson", view.game_number, n))
                        .show(ui, |ui| show_lines(ui, &m.lesson));
                    // Collapsed by default; its state is the window's only.
                    egui::CollapsingHeader::new(&details)
                        .id_salt(("coach-details", view.game_number, n))
                        .show(ui, |ui| show_lines(ui, &m.details));
                }
            }
            show_lines(ui, &lines);
        });
    }

    /// The table of a debriefing moment as the player saw it then (issue
    /// #47): facts, the hand with the chosen cards marked, and the trick in
    /// progress with the players' names. Display only.
    fn situation(&self, ui: &mut egui::Ui, s: &coach::SituationText) {
        show_lines(ui, &s.lines);
        ui.label(RichText::new(&s.hand_label).small());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for &card in &s.hand {
                let response = self.card_image(ui, card, MOMENT_CARD);
                if s.chosen.contains(&card) {
                    ui.painter().rect_stroke(
                        response.rect.expand(1.0),
                        3.0,
                        egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        });
        if s.trick_label.is_empty() {
            return;
        }
        ui.label(RichText::new(&s.trick_label).small());
        ui.horizontal(|ui| {
            for (player, card) in &s.trick {
                ui.vertical(|ui| {
                    self.card_image(ui, *card, MOMENT_CARD);
                    ui.label(RichText::new(player).small());
                });
            }
        });
    }

    /// The warning before a weak action: proceed or decide again.
    fn warning_window(&mut self, ctx: &egui::Context, view: &TableView, now: f64) {
        let Some(Pending::Warning { warning, .. }) = &view.coach.pending else {
            return;
        };
        let lines = coach::warning(&self.i18n, view, warning, view.coach.settings.show_figures);
        let (mut proceed, mut back) = (false, false);
        egui::Window::new(self.t("coach-warning-title"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                for line in lines {
                    let text = RichText::new(line.text);
                    ui.label(match line.style {
                        Style::Strong | Style::Heading => text.strong(),
                        Style::Weak => text.weak(),
                        Style::Normal => text,
                    });
                }
                ui.horizontal(|ui| {
                    back = ui.button(self.t("button-coach-reconsider")).clicked();
                    proceed = ui.button(self.t("button-coach-proceed")).clicked();
                });
            });
        if proceed {
            self.send(Command::ProceedAnyway, now);
        } else if back {
            self.send(Command::Reconsider, now);
        }
    }

    /// Opens the "new series" dialog proposing the next seed.
    fn open_new_series(&mut self) {
        self.open_new_series_with(self.session.config().seed.wrapping_add(1));
    }

    fn open_new_series_with(&mut self, seed: u64) {
        let config = self.session.config();
        let (free, games) = match config.series {
            SeriesLength::Fixed(n) => (false, n),
            SeriesLength::Free => (true, 36),
        };
        self.new_series = Some(NewSeries {
            free,
            games,
            kontra_re: config.kontra_re,
            mode: match config.opponents {
                Opponents::Levels(_) => Mode::Levels,
                Opponents::Goldfish => Mode::Goldfish,
                Opponents::Mephisto => Mode::Mephisto,
                Opponents::GodMode => Mode::God,
                Opponents::Experimental => Mode::Experimental,
                Opponents::Neural => Mode::Neural,
            },
            levels: match config.opponents {
                Opponents::Levels(levels) => levels,
                _ => [Level::Medium; 2],
            },
            open: config.display == TableDisplay::Open,
            seed: seed.to_string(),
        });
    }

    fn status(&self, ui: &mut egui::Ui, view: &TableView) {
        let i = &self.i18n;
        ui.horizontal_wrapped(|ui| {
            let number = i64::from(view.game_number);
            let game = match view.config.series {
                SeriesLength::Fixed(total) => i.tf(
                    "status-game",
                    &[("number", number.into()), ("total", total.into())],
                ),
                SeriesLength::Free => i.tf("status-game-free", &[("number", number.into())]),
            };
            ui.label(RichText::new(game).strong());
            ui.separator();
            ui.label(text::phase(i, view.phase));
            ui.separator();
            ui.label(i.tf(
                "status-dealer",
                &[("player", self.player(view.dealer).into())],
            ));
            if let Some(acting) = view.acting {
                ui.separator();
                ui.label(
                    RichText::new(i.tf("status-turn", &[("player", self.player(acting).into())]))
                        .strong(),
                );
            }
            if view.bid > 0 {
                ui.separator();
                ui.label(i.tf("status-bid", &[("value", view.bid.into())]));
            }
            if let Some(declarer) = view.declarer {
                ui.separator();
                ui.label(i.tf(
                    "status-declarer",
                    &[("player", self.player(declarer).into())],
                ));
            }
            if let Some(contract) = view.contract {
                ui.separator();
                ui.label(RichText::new(text::contract(i, contract)).strong());
            } else if view.hand_game == Some(true) {
                ui.separator();
                ui.label(i.t("contract-hand"));
            }
            if let Some(kontra) = text::kontra(i, view.kontra) {
                ui.separator();
                ui.label(
                    RichText::new(kontra)
                        .color(Color32::from_rgb(200, 40, 40))
                        .strong(),
                );
            }
            ui.separator();
            ui.label(match view.config.opponents {
                Opponents::Levels([left, right]) => i.tf(
                    "status-levels",
                    &[
                        ("left", i.t(level_message(left)).into()),
                        ("right", i.t(level_message(right)).into()),
                    ],
                ),
                Opponents::Goldfish => {
                    i.tf("status-opponents", &[("mode", i.t("mode-goldfish").into())])
                }
                Opponents::Mephisto => {
                    i.tf("status-opponents", &[("mode", i.t("mode-mephisto").into())])
                }
                Opponents::GodMode => i.tf("status-opponents", &[("mode", i.t("mode-god").into())]),
                Opponents::Experimental => i.tf(
                    "status-opponents",
                    &[("mode", i.t("mode-experimental").into())],
                ),
                Opponents::Neural => {
                    i.tf("status-opponents", &[("mode", i.t("mode-neural").into())])
                }
            });
            // Display versus coach basis (D-68), always named apart.
            if view.config.display == TableDisplay::Open {
                ui.separator();
                ui.label(RichText::new(i.t("status-open-display")).strong());
            }
            if view.open_cards.is_some() {
                ui.separator();
                let (id, hover) = match view.coach_basis {
                    CoachBasis::OwnView => ("status-coach-own-view", "coach-basis-own-view-open"),
                    CoachBasis::RealDeal => ("status-coach-real-deal", "coach-basis-real-deal"),
                };
                ui.label(RichText::new(i.t(id)).strong())
                    .on_hover_text(i.t(hover));
            }
            if self.show_seed {
                ui.separator();
                ui.label(i.tf(
                    "status-seed",
                    &[("seed", view.config.seed.to_string().into())],
                ))
                .on_hover_text(i.t("seed-description"));
            }
        });
    }

    // ----------------------------------------------------------------------
    // Right: score sheet and last trick.

    fn side_panel(&mut self, ui: &mut egui::Ui, view: &TableView) {
        // Every section scrolls and collapses, so the panel fits any height.
        egui::ScrollArea::vertical()
            .id_salt("side_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::CollapsingHeader::new(RichText::new(self.t("trick-last")).heading())
                    .id_salt("side_last_trick")
                    .default_open(true)
                    .show(ui, |ui| self.last_trick(ui, view));
                egui::CollapsingHeader::new(RichText::new(self.t("augen-title")).heading())
                    .id_salt("side_augen")
                    .default_open(true)
                    .show(ui, |ui| self.augen(ui, view));
                self.aid_section(ui, view);
                ui.separator();
                egui::CollapsingHeader::new(RichText::new(self.t("sheet-title")).heading())
                    .id_salt("side_sheet")
                    .default_open(true)
                    .show(ui, |ui| self.sheet(ui, view));
            });
    }

    fn last_trick(&self, ui: &mut egui::Ui, view: &TableView) {
        let i = &self.i18n;
        match &view.last_trick {
            Some(trick) => {
                ui.horizontal(|ui| {
                    for played in &trick.cards {
                        self.card_image(ui, played.card, SMALL_CARD);
                    }
                });
                ui.label(i.tf(
                    "trick-winner",
                    &[
                        ("player", self.player(trick.winner).into()),
                        ("augen", trick.augen.into()),
                    ],
                ));
            }
            None => {
                ui.label("–");
            }
        }
    }

    /// The shown game, for concealing the Augen again when it changes.
    fn game_key(&self, view: &TableView) -> GameKey {
        GameKey {
            seed: view.config.seed,
            number: view.game_number,
            practice: self.practice_attempt(),
        }
    }

    /// The Augen won in completed tricks (issue #134): concealed unless the
    /// button is held down with the mouse or the keyboard in the current
    /// game while the window has focus. The button's label and every text
    /// shown while concealed are free of numbers, so neither a tooltip nor
    /// assistive technology can reveal them.
    fn augen(&mut self, ui: &mut egui::Ui, view: &TableView) {
        let button = ui.add(egui::Button::new(self.t("augen-reveal")));
        // Held with the mouse: the primary button is down and the press
        // began on the button, however long it lasts.
        let rect = button.interact_rect;
        let pointer = ui.input(|i| {
            i.pointer.primary_down() && i.pointer.press_origin().is_some_and(|p| rect.contains(p))
        });
        let keys = button.has_focus()
            && ui.input(|i| i.key_down(egui::Key::Space) || i.key_down(egui::Key::Enter));
        let window = ui.input(|i| i.viewport().focused != Some(false));
        let held = (pointer || keys) && window;
        let game = self.game_key(view);
        let reveal = hold_reveals(&mut self.augen_hold, held, game);
        let i = &self.i18n;
        let declarer = view.declarer.filter(|_| view.tricks_played > 0);
        match declarer {
            None => {
                ui.label(RichText::new(i.t("augen-none")).weak());
            }
            Some(_) if !reveal => {
                ui.label(RichText::new(i.t("augen-hidden")).weak());
            }
            Some(declarer) => {
                let augen = view.trick_augen;
                ui.label(
                    RichText::new(i.tf(
                        "augen-declarer",
                        &[
                            ("player", self.player(declarer).into()),
                            ("augen", augen[declarer.index()].into()),
                        ],
                    ))
                    .strong(),
                );
                let defenders: Vec<PlayerId> = PlayerId::ALL
                    .into_iter()
                    .filter(|&p| p != declarer)
                    .collect();
                let names: Vec<String> = defenders.iter().map(|&p| self.player(p)).collect();
                let sum: u16 = defenders.iter().map(|p| u16::from(augen[p.index()])).sum();
                ui.label(
                    RichText::new(i.tf(
                        "augen-defence",
                        &[("player", names.join(" + ").into()), ("augen", sum.into())],
                    ))
                    .strong(),
                );
                for p in defenders {
                    ui.label(
                        RichText::new(i.tf(
                            "augen-player",
                            &[
                                ("player", self.player(p).into()),
                                ("augen", augen[p.index()].into()),
                            ],
                        ))
                        .small(),
                    );
                }
            }
        }
        ui.label(RichText::new(i.t("augen-skat-excluded")).small().weak());
    }

    /// The score sheet with Seeger-Fabian totals; always visible.
    fn sheet(&self, ui: &mut egui::Ui, view: &TableView) {
        let i = &self.i18n;
        if view.practice {
            ui.label(self.t("practice-not-scored"));
            return;
        }
        egui::ScrollArea::both()
            .id_salt("sheet_scroll")
            .max_height(360.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                egui::Grid::new("sheet").striped(true).show(ui, |ui| {
                    ui.strong(self.t("sheet-number"));
                    ui.strong(self.t("sheet-game"));
                    for p in PlayerId::ALL {
                        ui.strong(self.player(p));
                    }
                    ui.end_row();
                    for record in &view.sheet.records {
                        ui.label(record.number.to_string());
                        match (record.declarer, record.contract, record.result) {
                            (Some(declarer), Some(contract), Some(result)) => {
                                let mark = if result.won { "+" } else { "−" };
                                ui.label(format!(
                                    "{} {} {mark}{}",
                                    self.player(declarer),
                                    text::contract(i, contract),
                                    result.score.abs()
                                ));
                            }
                            _ => {
                                ui.label(self.t("sheet-eingepasst"));
                            }
                        }
                        for total in record.totals {
                            ui.label(total.to_string());
                        }
                        ui.end_row();
                    }
                    ui.strong(self.t("sheet-total"));
                    ui.label("");
                    for p in PlayerId::ALL {
                        ui.strong(view.players[p.index()].total.to_string());
                    }
                    ui.end_row();
                });
            });
    }

    // ----------------------------------------------------------------------
    // Centre: opponents, Skat and the trick.

    fn table(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        // The table scrolls rather than overlapping when results, the Skat
        // and a trick meet in a small window.
        egui::ScrollArea::vertical()
            .id_salt("table_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let wide = ui.available_width() >= 3.0 * (trick_area().x + 24.0);
                if self.session.guided().is_some() || !wide {
                    // Keep the fixed-size trick triangle clear of opponent
                    // headers: both opponents above it, at their sides.
                    ui.columns(2, |columns| {
                        self.opponent(&mut columns[0], view, PlayerId(1));
                        columns[1].with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                            self.opponent(ui, view, PlayerId(2))
                        });
                    });
                    ui.add_space(12.0);
                    self.middle(ui, view, now);
                    return;
                }
                ui.columns(3, |columns| {
                    self.opponent(&mut columns[0], view, PlayerId(1));
                    self.middle(&mut columns[1], view, now);
                    columns[2].with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                        self.opponent(ui, view, PlayerId(2))
                    });
                });
            });
    }

    /// Strength or table mode of an opponent, shown apart from its name.
    fn strength(&self, view: &TableView, player: PlayerId) -> String {
        let i = &self.i18n;
        match view.config.level(player) {
            Some(level) => i.tf(
                "opponent-strength",
                &[("level", i.t(level_message(level)).into())],
            ),
            None => i.tf(
                "opponent-mode",
                &[("mode", i.t(mode_message(view.config.opponents)).into())],
            ),
        }
    }

    /// An opponent's seat: caricature, name, seat and role, strength or
    /// mode, last call, cards and tricks (D-72).
    fn opponent(&self, ui: &mut egui::Ui, view: &TableView, player: PlayerId) {
        let i = &self.i18n;
        let info = view.players[player.index()];
        let acting = view.acting == Some(player);
        let plate = style::seat_plate(self.table_color(), acting);
        let compact = compact(ui.ctx());
        plate.show(ui, |ui| {
            ui.set_max_width(ui.available_width().min(320.0));
            let portrait = if compact { 0.7 * PORTRAIT } else { PORTRAIT };
            ui.horizontal(|ui| {
                if let Some(figure) = i.figure(player) {
                    let alt = i.tf(
                        "opponent-portrait",
                        &[("player", self.player(player).into())],
                    );
                    ui.add(
                        egui::Image::new(cards::portrait(figure))
                            .fit_to_exact_size(Vec2::splat(portrait))
                            .alt_text(alt),
                    );
                }
                ui.vertical(|ui| {
                    let mut title = RichText::new(self.player(player)).color(style::ON_TABLE);
                    title = if compact {
                        title.strong()
                    } else {
                        title.heading()
                    };
                    if acting {
                        title = title.underline().color(style::ON_TABLE_ACCENT);
                    }
                    ui.add(egui::Label::new(title).wrap());
                    let mut line = text::seat(i, info.seat);
                    if view.declarer == Some(player) {
                        line = format!("{line} · {}", self.t("label-declarer"));
                    }
                    ui.add(egui::Label::new(RichText::new(line).color(style::ON_TABLE)).wrap());
                    ui.add(
                        egui::Label::new(
                            RichText::new(self.strength(view, player))
                                .small()
                                .color(style::ON_TABLE_WEAK),
                        )
                        .wrap(),
                    );
                });
            });
            ui.horizontal_wrapped(|ui| {
                if let Some(call) = info.last_call {
                    ui.label(RichText::new(text::call(i, call)).color(style::ON_TABLE_ACCENT));
                }
                ui.label(
                    RichText::new(i.tf(
                        "label-cards-tricks",
                        &[("cards", info.cards.into()), ("tricks", info.tricks.into())],
                    ))
                    .color(style::ON_TABLE),
                );
            });
            let open = view.declarer == Some(player);
            match (&view.declarer_cards, open) {
                (Some(cards), true) => {
                    ui.label(RichText::new(self.t("label-open-cards")).color(style::ON_TABLE));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for &card in cards {
                            self.card_image(ui, card, SMALL_CARD);
                        }
                    });
                }
                _ => {
                    if let Some(open) = self.open_cards(view) {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = -SMALL_CARD.x * 0.6;
                            for &card in &open.hands[player.index()] {
                                self.card_image(ui, card, SMALL_CARD);
                            }
                        });
                    } else {
                        // Closed backs overlap in one row that fits the seat.
                        let size = if compact { COMPACT_CARD } else { SMALL_CARD };
                        let n = f32::from(info.cards.max(2));
                        let fit = (ui.available_width() - size.x) / (n - 1.0) - size.x;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = fit.min(-size.x * 0.6);
                            for _ in 0..info.cards {
                                ui.add(
                                    egui::Image::new(cards::image(self.back()))
                                        .fit_to_exact_size(size),
                                );
                            }
                        });
                    }
                }
            }
        });
    }

    fn middle(&self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        let i = &self.i18n;
        ui.vertical_centered(|ui| {
            // God Mode shows the Skat all game long, also after Drücken.
            let open_skat = self
                .open_cards(view)
                .map(|o| o.skat.as_slice())
                .filter(|s| !s.is_empty() && view.result.is_none());
            if let Some(skat) = open_skat {
                ui.label(RichText::new(self.t("label-skat")).color(style::ON_TABLE));
                ui.horizontal(|ui| {
                    for &card in skat {
                        self.card_image(ui, card, SMALL_CARD);
                    }
                });
            } else if matches!(view.phase, Phase::Reizen | Phase::SkatAufnahme) {
                ui.label(RichText::new(self.t("label-skat")).color(style::ON_TABLE));
                ui.horizontal(|ui| {
                    for _ in 0..2 {
                        ui.add(
                            egui::Image::new(cards::image(self.back()))
                                .fit_to_exact_size(SMALL_CARD),
                        );
                    }
                });
            }
            let (title, cards): (String, Vec<PlayedCard>) = if !view.current_trick.is_empty() {
                (self.t("trick-current"), view.current_trick.clone())
            } else if let Some(last) = view
                .last_trick
                .as_ref()
                .filter(|_| now < self.show_trick_until)
            {
                let winner = self.player(last.winner);
                (
                    i.tf(
                        "trick-winner",
                        &[("player", winner.into()), ("augen", last.augen.into())],
                    ),
                    last.cards.clone(),
                )
            } else {
                (String::new(), Vec::new())
            };
            if !cards.is_empty() {
                ui.label(RichText::new(title).color(style::ON_TABLE));
                self.trick_triangle(ui, &cards);
            }
            if let Some(result) = &view.result {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.heading(self.t("result-title"));
                    for line in text::result_lines(i, result.declarer, &result.result) {
                        ui.label(line);
                    }
                    for p in PlayerId::ALL {
                        let points = result.points[p.index()];
                        if points != 0 {
                            ui.label(i.tf(
                                "result-points",
                                &[("player", self.player(p).into()), ("points", points.into())],
                            ));
                        }
                    }
                });
            }
            if view.phase == Phase::Eingepasst {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(self.t("result-eingepasst"));
                });
            }
            if let Some(series) = &view.series_result {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.heading(i.tf("series-finished", &[("games", series.games.into())]));
                    for (rank, p) in series.ranking.iter().enumerate() {
                        ui.label(i.tf(
                            "series-rank",
                            &[
                                ("rank", (rank + 1).into()),
                                ("player", self.player(*p).into()),
                                ("total", series.totals[p.index()].into()),
                                ("won", series.won[p.index()].into()),
                                ("lost", series.lost[p.index()].into()),
                            ],
                        ));
                    }
                });
            }
        });
    }

    // ----------------------------------------------------------------------
    // Bottom: actions and the human's hand.

    fn actions(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        let legal = view.legal_actions.clone();
        ui.horizontal_wrapped(|ui| {
            if let Some(error) = self.error {
                ui.label(
                    RichText::new(text::error(&self.i18n, error))
                        .color(Color32::from_rgb(220, 50, 50))
                        .strong(),
                );
                ui.separator();
            }
            if view.series_finished {
                if ui.button(self.t("menu-new-series")).clicked() {
                    self.open_new_series();
                }
                return;
            }
            if view.practice && view.acting.is_none() {
                // Return or a new attempt: in the practice bar above.
                ui.label(RichText::new(self.t("practice-finished")).strong());
                return;
            }
            if view.acting.is_none() {
                if view.can_next_game && ui.button(self.t("button-next-game")).clicked() {
                    self.send(Command::NextGame, now);
                }
                if view.can_end_series && ui.button(self.t("menu-end-series")).clicked() {
                    self.send(Command::EndSeries, now);
                }
                return;
            }
            match &view.coach.pending {
                Some(Pending::Checking { action }) => {
                    ui.spinner();
                    ui.label(self.i18n.tf(
                        "coach-checking",
                        &[("action", coach::action(&self.i18n, view, *action).into())],
                    ));
                    if ui.button(self.t("button-coach-skip-check")).clicked() {
                        self.send(Command::ProceedAnyway, now);
                    }
                    if ui.button(self.t("button-coach-reconsider")).clicked() {
                        self.send(Command::Reconsider, now);
                    }
                    return;
                }
                Some(Pending::Warning { .. }) => return,
                None => {}
            }
            if let Some(acting) = view.acting.filter(|p| !p.is_human()) {
                ui.label(
                    self.i18n
                        .tf("status-waiting", &[("player", self.player(acting).into())]),
                );
                return;
            }
            match view.phase {
                Phase::Reizen => self.bidding(ui, view, &legal, now),
                Phase::SkatAufnahme => {
                    if ui.button(self.t("button-hand")).clicked() {
                        self.act(Action::Hand, now);
                    }
                    if ui.button(self.t("button-take-skat")).clicked() {
                        self.act(Action::TakeSkat, now);
                    }
                }
                Phase::Druecken => {
                    ui.label(self.t("hint-discard"));
                    if ui.button(self.t("button-discard")).clicked() {
                        let cards = self.selected;
                        self.act(Action::Discard(cards), now);
                        if self.error.is_none() {
                            self.selected = CardSet::EMPTY;
                        }
                    }
                }
                Phase::Ansage => self.announcement(ui, view, now),
                Phase::Spiel => {
                    for (action, id) in [
                        (Action::Kontra, "button-kontra"),
                        (Action::KeinKontra, "button-kein-kontra"),
                        (Action::Re, "button-re"),
                        (Action::KeinRe, "button-kein-re"),
                    ] {
                        if legal.contains(&action) && ui.button(self.t(id)).clicked() {
                            self.act(action, now);
                        }
                    }
                    if legal.iter().any(|a| matches!(a, Action::Play(_))) {
                        ui.label(self.t("hint-play"));
                    }
                }
                Phase::Beendet | Phase::Eingepasst => {}
            }
        });
    }

    fn bidding(&mut self, ui: &mut egui::Ui, view: &TableView, legal: &[Action], now: f64) {
        if legal.contains(&Action::Hold) {
            if ui
                .button(self.i18n.tf("button-hold", &[("value", view.bid.into())]))
                .clicked()
            {
                self.act(Action::Hold, now);
            }
        } else {
            let values: Vec<u16> = legal
                .iter()
                .filter_map(|a| match a {
                    Action::Bid(v) => Some(*v),
                    _ => None,
                })
                .collect();
            if let Some(&next) = values.first()
                && ui
                    .button(self.i18n.tf("button-bid", &[("value", next.into())]))
                    .clicked()
            {
                self.act(Action::Bid(next), now);
            }
            if values.len() > 1 {
                ui.label(self.t("label-jump-bid"));
                let jump = self
                    .jump_bid
                    .filter(|v| values.contains(v))
                    .unwrap_or(values[0]);
                egui::ComboBox::from_id_salt("jump-bid")
                    .selected_text(jump.to_string())
                    .show_ui(ui, |ui| {
                        for &value in reizwerte().iter().filter(|v| values.contains(v)) {
                            ui.selectable_value(&mut self.jump_bid, Some(value), value.to_string());
                        }
                    });
                if ui.button(self.t("button-jump")).clicked() {
                    self.act(Action::Bid(jump), now);
                }
            }
        }
        if ui.button(self.t("button-pass")).clicked() {
            self.act(Action::Pass, now);
        }
    }

    fn announcement(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        let i = &self.i18n;
        let hand = view.hand_game == Some(true);
        let mut draft = self.draft;
        ui.label(RichText::new(i.t("label-announce")).strong());
        let games = [
            GameType::Farbe(Suit::Kreuz),
            GameType::Farbe(Suit::Pik),
            GameType::Farbe(Suit::Herz),
            GameType::Farbe(Suit::Karo),
            GameType::Grand,
            GameType::Null,
        ];
        for game in games {
            ui.radio_value(&mut draft.game, game, text::game_type(i, game));
        }
        ui.separator();
        let trump_game = draft.game != GameType::Null;
        ui.add_enabled(
            hand && trump_game,
            egui::Checkbox::new(&mut draft.schneider, i.t("contract-schneider")),
        );
        ui.add_enabled(
            hand && trump_game,
            egui::Checkbox::new(&mut draft.schwarz, i.t("contract-schwarz")),
        );
        ui.add_enabled(
            hand || !trump_game,
            egui::Checkbox::new(&mut draft.ouvert, i.t("contract-ouvert")),
        );
        // Keep the implied levels consistent (Ouvert implies Schwarz implies
        // Schneider in suit and Grand games); the engine checks the rest.
        if trump_game {
            if !hand {
                draft.schneider = false;
                draft.schwarz = false;
                draft.ouvert = false;
            }
            if draft.ouvert {
                draft.schwarz = true;
            }
            if draft.schwarz {
                draft.schneider = true;
            }
        } else {
            draft.schneider = false;
            draft.schwarz = false;
        }
        self.draft = draft;
        if ui.button(self.t("button-announce")).clicked() {
            self.act(Action::Announce(draft), now);
        }
    }

    fn hand(&mut self, ui: &mut egui::Ui, view: &TableView, now: f64) {
        let playable: Vec<Card> = view
            .legal_actions
            .iter()
            .filter_map(|a| match a {
                Action::Play(c) => Some(*c),
                _ => None,
            })
            .collect();
        let free = view.acting == Some(PlayerId::HUMAN) && view.coach.pending.is_none();
        let discarding = view.phase == Phase::Druecken && free;
        let playing = view.phase == Phase::Spiel && free;
        // Cards shrink to fit the width, down to a minimum; then they wrap,
        // so every card stays reachable in a small window (issue #134).
        let gap = 4.0;
        let n = view.hand.len().max(1) as f32;
        // Each card is a button: a slim padding and the frame add to it.
        ui.spacing_mut().button_padding = Vec2::splat(2.0);
        let frame = 2.0 * ui.spacing().button_padding.x + 2.0;
        let fit = (ui.available_width() - n * frame - gap * (n - 1.0)) / n;
        let widest = if compact(ui.ctx()) {
            COMPACT_HAND_CARD
        } else {
            CARD.x
        };
        let width = fit.clamp(MIN_HAND_CARD, widest);
        let size = Vec2::new(width, width * CARD.y / CARD.x);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &card in &view.hand {
                let selected = self.selected.contains(card);
                let dim = playing && !playable.contains(&card);
                let mut image =
                    egui::Image::new(cards::image(&self.face(card))).fit_to_exact_size(size);
                if dim {
                    image = image.tint(Color32::from_gray(150));
                }
                let response = ui.add(
                    egui::Button::image(image)
                        .selected(selected)
                        .frame(selected),
                );
                let label = text::card(&self.i18n, card);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone())
                });
                let response = response.on_hover_text(text::card(&self.i18n, card));
                if response.clicked() {
                    if discarding {
                        if !self.selected.remove(card) {
                            self.selected.insert(card);
                        }
                    } else if playing {
                        self.act(Action::Play(card), now);
                    }
                }
            }
        });
        if let Some(skat) = &view.own_skat {
            let size = if compact(ui.ctx()) {
                COMPACT_CARD
            } else {
                SMALL_CARD
            };
            ui.horizontal(|ui| {
                ui.label(self.t("label-own-skat"));
                for &card in skat {
                    self.card_image(ui, card, size);
                }
            });
        }
    }

    /// Faces, back and table of the session's table mode.
    pub fn theme(&self) -> Theme {
        match self.session.config().opponents {
            Opponents::Mephisto => Theme::Mephisto,
            Opponents::GodMode => Theme::God,
            _ => Theme::Classic,
        }
    }

    /// Face of `card` in the chosen deck (path below `assets/cards/`); the
    /// Mephisto or God Mode variant in those modes.
    pub fn face(&self, card: Card) -> String {
        cards::face_path(self.deck, card, self.theme())
    }

    /// Card back (path below `assets/cards/`) of the table mode.
    pub fn back(&self) -> &'static str {
        cards::back_path(self.theme())
    }

    /// Table colour: red against Mephisto, blue in God Mode, green
    /// otherwise.
    pub fn table_color(&self) -> Color32 {
        match self.theme() {
            Theme::Classic => TABLE_GREEN,
            Theme::Mephisto => TABLE_RED,
            Theme::God => TABLE_BLUE,
        }
    }

    /// The open cards of an open display or God Mode while the player wants
    /// to see them.
    fn open_cards<'a>(&self, view: &'a TableView) -> Option<&'a OpenCards> {
        view.open_cards.as_ref().filter(|_| self.see_all)
    }

    /// Draws a trick as a triangle: every card lies on the side of the
    /// player who played it, in playing order (issue #24).
    fn trick_triangle(&self, ui: &mut egui::Ui, trick: &[PlayedCard]) {
        let (area, _) = ui.allocate_exact_size(trick_area(), egui::Sense::hover());
        for played in trick {
            let card = egui::Rect::from_min_size(area.min + trick_slot(played.player), CARD);
            let label = text::card(&self.i18n, played.card);
            ui.put(
                card,
                egui::Image::new(cards::image(&self.face(played.card))).fit_to_exact_size(CARD),
            )
            .on_hover_text(label);
            let name = egui::Rect::from_min_size(
                card.left_bottom(),
                Vec2::new(CARD.x, TRICK_LABEL_HEIGHT),
            );
            // Long names are shortened under the card; the full name shows
            // on hover and at the seat.
            let player = self.player(played.player);
            ui.put(
                name,
                egui::Label::new(RichText::new(&player).color(style::ON_TABLE)).truncate(),
            )
            .on_hover_text(player);
        }
        // `put` leaves the cursor after the last card placed; whatever
        // follows (the result) goes below the whole triangle.
        ui.advance_cursor_after_rect(area);
    }

    fn card_image(&self, ui: &mut egui::Ui, card: Card, size: Vec2) -> egui::Response {
        let label = text::card(&self.i18n, card);
        ui.add(egui::Image::new(cards::image(&self.face(card))).fit_to_exact_size(size))
            .on_hover_text(label)
    }

    // ----------------------------------------------------------------------
    // Dialog for a new series.

    fn new_series_window(&mut self, ctx: &egui::Context, now: f64) {
        let Some(mut draft) = self.new_series.clone() else {
            return;
        };
        let mut open = true;
        let mut start = false;
        let mut cancel = false;
        let seed = parse_seed(&draft.seed);
        egui::Window::new(self.t("new-series-title"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.radio_value(&mut draft.free, false, self.t("series-fixed"));
                ui.add_enabled(
                    !draft.free,
                    egui::Slider::new(&mut draft.games, 3..=72).text(self.t("series-games")),
                );
                ui.radio_value(&mut draft.free, true, self.t("series-free"));
                ui.checkbox(&mut draft.kontra_re, self.t("option-kontra-re"));
                ui.separator();
                ui.radio_value(&mut draft.mode, Mode::Levels, self.t("mode-levels"));
                for (k, id) in ["opponent-left", "opponent-right"].into_iter().enumerate() {
                    ui.add_enabled_ui(draft.mode == Mode::Levels, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(self.t(id));
                            for level in Level::ALL {
                                let label = self.t(level_message(level));
                                ui.radio_value(&mut draft.levels[k], level, label)
                                    .on_hover_text(self.t(level_description(level)));
                            }
                        });
                    });
                }
                ui.radio_value(&mut draft.mode, Mode::Goldfish, self.t("mode-goldfish"))
                    .on_hover_text(self.t("mode-goldfish-description"));
                ui.radio_value(&mut draft.mode, Mode::Mephisto, self.t("mode-mephisto"));
                ui.radio_value(&mut draft.mode, Mode::God, self.t("mode-god"))
                    .on_hover_text(self.t("mode-god-description"));
                ui.radio_value(
                    &mut draft.mode,
                    Mode::Experimental,
                    self.t("mode-experimental"),
                );
                ui.radio_value(&mut draft.mode, Mode::Neural, self.t("mode-neural"))
                    .on_hover_text(self.t("mode-neural-description"));
                let mut notes = Vec::new();
                match draft.mode {
                    Mode::Levels => {
                        notes.push(level_description(draft.levels[0]));
                        if draft.levels[1] != draft.levels[0] {
                            notes.push(level_description(draft.levels[1]));
                        }
                    }
                    Mode::Goldfish => notes.push("mode-goldfish-description"),
                    Mode::Mephisto => {}
                    Mode::God => notes.push("mode-god-description"),
                    Mode::Experimental => notes.push("mode-experimental-description"),
                    Mode::Neural => {
                        notes.push("mode-neural-description");
                        if !skat_app::neural_model_available() {
                            notes.push("mode-neural-missing");
                        }
                    }
                }
                for note in notes {
                    ui.label(RichText::new(self.t(note)).small().weak());
                }
                ui.separator();
                // Open display (D-68): a display choice only, independent
                // of the opponents; God Mode always shows every card.
                let god = draft.mode == Mode::God;
                let mut shown = draft.open || god;
                ui.add_enabled(
                    !god,
                    egui::Checkbox::new(&mut shown, self.t("option-open-display")),
                )
                .on_hover_text(self.t("option-open-display-description"));
                if !god {
                    draft.open = shown;
                }
                let note = match draft.mode {
                    Mode::God => Some("option-open-display-god"),
                    _ if !draft.open => None,
                    Mode::Mephisto => Some("option-open-display-mephisto"),
                    _ => Some("option-open-display-description"),
                };
                if let Some(note) = note {
                    ui.label(RichText::new(self.t(note)).small().weak());
                }
                if self.show_seed {
                    ui.separator();
                    ui.horizontal(|ui| {
                        let label = ui
                            .label(self.t("series-seed"))
                            .on_hover_text(self.t("seed-description"));
                        ui.text_edit_singleline(&mut draft.seed)
                            .labelled_by(label.id)
                            .on_hover_text(self.t("seed-description"));
                    });
                    let note = if seed.is_some() {
                        RichText::new(self.t("seed-description")).weak()
                    } else {
                        RichText::new(self.t("seed-invalid")).color(Color32::from_rgb(200, 40, 40))
                    };
                    ui.label(note.small());
                }
                ui.horizontal(|ui| {
                    start = ui
                        .add_enabled(seed.is_some(), egui::Button::new(self.t("button-start")))
                        .clicked();
                    cancel = ui.button(self.t("button-cancel")).clicked();
                });
            });
        if start && let Some(seed) = seed {
            let coach = self.session.coach_settings();
            self.session = Session::new(SessionConfig {
                seed,
                series: if draft.free {
                    SeriesLength::Free
                } else {
                    SeriesLength::Fixed(draft.games)
                },
                kontra_re: draft.kontra_re,
                opponents: match draft.mode {
                    Mode::Levels => Opponents::Levels(draft.levels),
                    Mode::Goldfish => Opponents::Goldfish,
                    Mode::Mephisto => Opponents::Mephisto,
                    Mode::God => Opponents::GodMode,
                    Mode::Experimental => Opponents::Experimental,
                    Mode::Neural => Opponents::Neural,
                },
                display: if draft.open && draft.mode != Mode::God {
                    TableDisplay::Open
                } else {
                    TableDisplay::Closed
                },
            });
            self.thinking = None;
            self.ready = None;
            self.coach_thinking = None;
            self.error = None;
            self.selected = CardSet::EMPTY;
            // A new series is a new learning-history attempt.
            self.learning_attempt = None;
            // Coach settings belong to the player, not to the series.
            let _ = self.session.execute(Command::SetCoach(coach));
            self.next_move_at = now + self.opponent_delay;
            self.new_series = None;
        } else if cancel || !open {
            self.new_series = None;
        } else {
            self.new_series = Some(draft);
        }
    }
}

/// The seed typed in the "new series" dialog: a whole number from 0 to
/// `u64::MAX`, surrounding spaces ignored.
fn parse_seed(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

/// Message id of a strength level's name.
fn level_message(level: Level) -> &'static str {
    match level {
        Level::Rookie => "level-rookie",
        Level::Easy => "level-easy",
        Level::Medium => "level-medium",
        Level::Strong => "level-strong",
    }
}

/// Shows coach lines in their styles.
/// `true` in a window narrower than [`WIDE_LAYOUT`]: smaller seats, hand
/// and own Skat keep the trick in view down to 960 × 640.
fn compact(ctx: &egui::Context) -> bool {
    ctx.content_rect().width() < WIDE_LAYOUT
}

/// `true` if the held reveal button shows the concealed Augen of `game`:
/// only while held and only in the game in which the hold began, so a new
/// game, series or practice attempt conceals them until the next press.
fn hold_reveals(hold: &mut Option<GameKey>, held: bool, game: GameKey) -> bool {
    if !held {
        *hold = None;
    } else if hold.is_none() {
        *hold = Some(game);
    }
    held && *hold == Some(game)
}

/// Message of a table mode without per-opponent levels.
fn mode_message(opponents: Opponents) -> &'static str {
    match opponents {
        Opponents::Levels(_) => "mode-levels",
        Opponents::Goldfish => "mode-goldfish",
        Opponents::Mephisto => "mode-mephisto",
        Opponents::GodMode => "mode-god",
        Opponents::Neural => "mode-neural",
        Opponents::Experimental => "mode-experimental",
    }
}

fn show_lines(ui: &mut egui::Ui, lines: &[coach::Line]) {
    for line in lines {
        let text = RichText::new(&line.text);
        match line.style {
            Style::Heading => {
                ui.add_space(4.0);
                ui.label(text.strong().size(15.0));
            }
            Style::Strong => {
                ui.label(text.strong());
            }
            Style::Normal => {
                ui.label(text);
            }
            Style::Weak => {
                ui.label(text.weak().small());
            }
        }
    }
}

/// Message id of a coach level.
fn coach_level_message(level: CoachLevel) -> &'static str {
    match level {
        CoachLevel::Off => "coach-level-off",
        CoachLevel::OnRequest => "coach-level-on-request",
        CoachLevel::Important => "coach-level-important",
        CoachLevel::EveryMove => "coach-level-every-move",
    }
}

/// Message id of a strength level's description.
fn level_description(level: Level) -> &'static str {
    match level {
        Level::Rookie => "level-rookie-description",
        Level::Easy => "level-easy-description",
        Level::Medium => "level-medium-description",
        Level::Strong => "level-strong-description",
    }
}

impl eframe::App for GrandCoachApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = ui.input(|i| i.time);
        // Suit names follow the shown faces (D-62).
        self.i18n.faces = Faces {
            german: self.deck == Deck::German,
            theme: self.theme(),
        };
        if self.training.active() {
            // The live table waits: not drawn, driven or changed.
            let ctx = ui.ctx().clone();
            match self
                .training
                .screen(ui, &self.i18n, self.deck, &mut self.learning)
            {
                Some(crate::training::TrainingRequest::Exit) => {
                    self.training.exit();
                    ctx.request_repaint();
                }
                Some(crate::training::TrainingRequest::History) => self.learning.show(),
                None => {}
            }
            self.learning.window(&ctx, &self.i18n);
            return;
        }
        // The opponents wait while the "new series" dialog is open.
        if self.new_series.is_none() && !self.recording.active() {
            self.drive_opponents(ui.ctx(), now);
            self.drive_coach(ui.ctx(), now);
        }
        if self.recording.active()
            && let Some(g) = self.session.guided()
        {
            self.send(Command::Guided(g.id, skat_app::GuidedAction::Return), now);
        }
        let view = self.session.view();
        // Names and portraits of this series' opponents (D-72).
        self.i18n.cast = Some(view.cast);
        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui, &view, now));
        let blocked = self.recording.active();
        if self.practice.is_some() {
            egui::Panel::top("practice")
                .frame(
                    egui::Frame::side_top_panel(ui.style()).fill(Color32::from_rgb(255, 236, 200)),
                )
                .show(ui, |ui| self.practice_bar(ui, &view));
        }
        ui.add_enabled_ui(!blocked, |ui| {
            egui::Panel::top("status").show(ui, |ui| self.status(ui, &view));
            let guided = self.session.guided().is_some();
            let coach = Self::coach_visible(&view);
            let wide = ui.available_width() >= WIDE_LAYOUT;
            let hand = |app: &mut Self, ui: &mut egui::Ui| {
                egui::Panel::bottom("hand").show(ui, |ui| {
                    ui.add_space(4.0);
                    app.actions(ui, &view, now);
                    ui.add_space(4.0);
                    app.hand(ui, &view, now);
                    ui.add_space(4.0);
                });
            };
            if guided {
                // The complete playable hand keeps the window width, even
                // at 960 px; history stays beside the table above it.
                hand(self, ui);
                egui::Panel::right("side")
                    .default_size(300.0)
                    .show(ui, |ui| self.side_panel(ui, &view));
                if coach {
                    egui::Panel::left("coach")
                        .default_size(300.0)
                        .min_size(280.0)
                        .show(ui, |ui| self.coach_panel(ui, &view, now));
                }
            } else if wide {
                // Coach left, table with the hand in the centre, last trick,
                // Augen and score sheet right (issue #134).
                if coach {
                    egui::Panel::left("coach")
                        .default_size(340.0)
                        .min_size(240.0)
                        .show(ui, |ui| self.coach_panel(ui, &view, now));
                }
                egui::Panel::right("side")
                    .default_size(300.0)
                    .min_size(220.0)
                    .show(ui, |ui| self.side_panel(ui, &view));
                hand(self, ui);
            } else {
                // A small window: the hand and the actions span the full
                // width below both panels.
                hand(self, ui);
                egui::Panel::right("side")
                    .default_size(230.0)
                    .min_size(200.0)
                    .max_size(250.0)
                    .show(ui, |ui| self.side_panel(ui, &view));
                if coach {
                    egui::Panel::left("coach")
                        .default_size(240.0)
                        .min_size(200.0)
                        .max_size(260.0)
                        .show(ui, |ui| self.coach_panel(ui, &view, now));
                }
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(ui.style()).fill(self.table_color()))
                .show(ui, |ui| self.table(ui, &view, now));
            let ctx = ui.ctx().clone();
            if !blocked {
                self.warning_window(&ctx, &view, now);
                self.quiz_window(&ctx, &view, now);
                self.new_series_window(&ctx, now);
            }
        });
        let ctx = ui.ctx().clone();
        let can_practice = self.practice.is_none();
        self.recording.budget = self.session.coach_budget();
        // A reopened game names the figures of its own series.
        if let Some(cast) = self.recording.replay_cast() {
            self.i18n.cast = Some(cast);
        }
        let started = self
            .recording
            .windows(&ctx, &self.i18n, self.deck, can_practice, &view);
        self.i18n.cast = Some(view.cast);
        if let Some(recording) = started {
            self.practice_request = Some(PracticeRequest::Start(recording));
        }
        self.learning.window(&ctx, &self.i18n);
        if self.practice_request.is_some() {
            self.apply_practice_request();
            ctx.request_repaint();
        }
        if now < self.show_trick_until {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                self.show_trick_until - now,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(player: PlayerId) -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO + trick_slot(player), CARD)
    }

    #[test]
    fn recording_keeps_outstanding_opponent_result_until_return_to_live_series() {
        use egui_kittest::{Harness, kittest::Queryable};
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = Cancel::new();
        let observed_cancel = cancel.clone();
        let mut h = Harness::builder()
            .with_size([1280.0, 820.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 7,
                        series: SeriesLength::Fixed(3),
                        kontra_re: false,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        ..SessionConfig::default()
                    },
                );
                assert!(app.session.opponent_to_move());
                app.thinking = Some(Thinking {
                    result: receiver,
                    cancel,
                });
                app.next_move_at = 1e9;
                app.recording.show_dialog(false, &app.session);
                app
            });
        let before = h.state().session.view();
        let mut control = h.state().session.clone();
        let mv = control.opponent_job().unwrap().run(None);
        h.run_steps(4); // decision still outstanding
        assert!(h.state().thinking.is_some());
        sender.send(mv).unwrap(); // completed while inspecting a recording
        h.run_steps(4);
        assert!(h.state().thinking.is_some());
        assert!(h.state().ready.is_none());
        assert!(!observed_cancel.is_cancelled());
        assert_eq!(h.state().session.view(), before);
        h.get_by_role_and_label(egui::Role::Button, "Abbrechen")
            .click();
        h.run_steps(4);
        assert!(h.state().ready.is_some());
        assert_eq!(h.state().session.view(), before);
        let ctx = h.ctx.clone();
        h.state_mut().drive_opponents(&ctx, 1e9);
        control.finish_opponent(mv).unwrap();
        assert_eq!(h.state().session.view(), control.view());
    }

    #[test]
    fn recording_retains_outstanding_coach_analysis_without_restarting_it() {
        use egui_kittest::{Harness, kittest::Queryable};
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = Cancel::new();
        let observed_cancel = cancel.clone();
        let mut h = Harness::builder()
            .with_size([1280.0, 820.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 7,
                        series: SeriesLength::Fixed(3),
                        kontra_re: false,
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        ..SessionConfig::default()
                    },
                );
                app.session.run_opponents();
                app.session.set_coach_budget(skat_app::Budget {
                    samples: 1,
                    nodes_per_task: 10,
                });
                app.session.execute(Command::RequestAdvice).unwrap();
                let job = app
                    .session
                    .coach_job()
                    .expect("human decision requested analysis");
                app.coach_thinking = Some(CoachThinking {
                    job,
                    result: receiver,
                    cancel,
                });
                app.recording.show_dialog(false, &app.session);
                app
            });
        let before = h.state().session.view();
        let job = h.state().coach_thinking.as_ref().unwrap().job.clone();
        h.run_steps(4);
        assert!(!observed_cancel.is_cancelled());
        assert!(
            h.state()
                .coach_thinking
                .as_ref()
                .unwrap()
                .job
                .same_decision(&job)
        );
        let result = job.run(None);
        let mut control = h.state().session.clone();
        control.finish_coach(result.clone()).unwrap();
        sender.send(result).unwrap();
        h.run_steps(4);
        assert_eq!(h.state().session.view(), before);
        assert!(!observed_cancel.is_cancelled());
        h.get_by_role_and_label(egui::Role::Button, "Abbrechen")
            .click();
        h.run_steps(4);
        assert_eq!(h.state().session.view(), control.view());
    }

    fn easy_series() -> SessionConfig {
        SessionConfig {
            seed: 7,
            series: SeriesLength::Fixed(3),
            kontra_re: true,
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        }
    }

    #[test]
    fn series_practice_and_retry_record_under_their_own_learning_attempts() {
        use skat_app::{
            AssistanceLevel, AttemptKind, LessonFamily, QuestionId, QuestionKind, RetrievalAction,
            RetrievalEvent,
        };
        let ctx = egui::Context::default();
        let mut app = GrandCoachApp::new(&ctx, easy_series());
        let opportunity = |trick| {
            [AppEvent::Retrieval(RetrievalEvent {
                id: QuestionId {
                    series: 1,
                    game: 1,
                    trick,
                }
                .into(),
                question: QuestionKind::TrumpfOutside,
                action: RetrievalAction::Opportunity,
                assistance: AssistanceLevel::AnswerBeforeReveal,
                assisted: false,
                lesson: Some(LessonFamily::FocusedRetrieval.current()),
            })]
        };
        app.record_learning(&opportunity(3));
        let recording = app.session.export_current_recording_privileged().unwrap();
        app.start_practice(recording).unwrap();
        // The same question ID in practice and in a retry: new attempts.
        app.record_learning(&opportunity(3));
        app.restart_practice();
        app.record_learning(&opportunity(3));
        app.record_learning(&opportunity(3));
        app.return_to_series();
        // Back in the series: its own attempt again, so this is a duplicate.
        app.record_learning(&opportunity(3));
        app.record_learning(&opportunity(6));
        let h = app.learning_history();
        let kinds: Vec<_> = h.attempts().iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            [
                AttemptKind::Series,
                AttemptKind::Practice { attempt: 1 },
                AttemptKind::Practice { attempt: 2 },
            ]
        );
        let per_attempt: Vec<_> = h
            .attempts()
            .iter()
            .map(|a| h.events().iter().filter(|e| e.attempt == a.id).count())
            .collect();
        assert_eq!(per_attempt, [2, 1, 1]);
        // Without a file nothing is written anywhere.
        assert!(app.learning_file().is_none());
        // A new series clears the attempt, as the new-series dialog does.
        app.learning_attempt = None;
        app.record_learning(&opportunity(3));
        assert_eq!(app.learning_history().attempts().len(), 4);
    }

    #[test]
    fn practice_retains_live_opponent_work_and_disconnects_stale_practice_output() {
        use egui_kittest::{Harness, kittest::Queryable};
        let (live_sender, live_receiver) = std::sync::mpsc::channel();
        let live_cancel = Cancel::new();
        let observed_live = live_cancel.clone();
        let mut h = Harness::builder()
            .with_size([1280.0, 820.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(&cc.egui_ctx, easy_series());
                assert!(app.session.opponent_to_move());
                app.thinking = Some(Thinking {
                    result: live_receiver,
                    cancel: live_cancel,
                });
                app.next_move_at = 1e9;
                app.selected = CardSet::EMPTY;
                app.jump_bid = Some(23);
                app
            });
        let before = h.state().session.view();
        let mut control = h.state().session.clone();
        let live_move = control.opponent_job().unwrap().run(None);
        // Same game and position as the live table: before bidding.
        let recording = h
            .state()
            .session
            .export_current_recording_privileged()
            .unwrap();
        h.state_mut().start_practice(recording).unwrap();
        assert!(h.state().practicing());
        assert!(h.state().session.is_practice());
        assert!(h.state().thinking.is_none() && h.state().jump_bid.is_none());
        // A practice opponent decision that will be outstanding at return.
        let (practice_sender, practice_receiver) = std::sync::mpsc::channel();
        let practice_cancel = Cancel::new();
        let observed_practice = practice_cancel.clone();
        let practice_move = h.state().session.opponent_job().unwrap().run(None);
        h.state_mut().thinking = Some(Thinking {
            result: practice_receiver,
            cancel: practice_cancel,
        });
        h.state_mut().next_move_at = 1e9;
        // The live move completes while practising: neither table takes it.
        live_sender.send(live_move).unwrap();
        h.run_steps(4);
        h.get_by_label_contains("ÜBUNG");
        assert!(h.state().session.is_practice());
        assert_eq!(
            h.state_mut().session.finish_opponent(live_move),
            Err(AppError::StaleOpponentMove)
        );
        assert!(!observed_live.is_cancelled());
        assert_eq!(h.state().live_session().view(), before);
        h.get_by_role_and_label(egui::Role::Button, "Zurück zur Serie")
            .click();
        h.run_steps(2);
        assert!(!h.state().practicing());
        // Practice work was cancelled and its channel is gone.
        assert!(observed_practice.is_cancelled());
        assert!(practice_sender.send(practice_move).is_err());
        assert_eq!(
            h.state_mut().session.finish_opponent(practice_move),
            Err(AppError::StaleOpponentMove)
        );
        assert_eq!(h.state().jump_bid, Some(23));
        assert_eq!(h.state().session.view(), before);
        // The retained live move was received (not lost or cancelled first)
        // and is applied exactly as without practice.
        assert!(h.state().ready.is_some());
        let ctx = h.ctx.clone();
        h.state_mut().drive_opponents(&ctx, 1e9);
        control.finish_opponent(live_move).unwrap();
        assert_eq!(h.state().session.view(), control.view());
    }

    #[test]
    fn practice_retains_live_coach_analysis_and_restarts_isolate_attempts() {
        use egui_kittest::{Harness, kittest::Queryable};
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = Cancel::new();
        let observed_cancel = cancel.clone();
        let mut h = Harness::builder()
            .with_size([1280.0, 820.0])
            .build_eframe(move |cc| {
                let mut app = GrandCoachApp::new(&cc.egui_ctx, easy_series());
                app.session.run_opponents();
                app.session.set_coach_budget(skat_app::Budget {
                    samples: 1,
                    nodes_per_task: 10,
                });
                app.session.execute(Command::RequestAdvice).unwrap();
                let job = app.session.coach_job().expect("requested analysis");
                app.coach_thinking = Some(CoachThinking {
                    job,
                    result: receiver,
                    cancel,
                });
                app.opponent_delay = 0.0;
                app
            });
        let before = h.state().session.view();
        let job = h.state().coach_thinking.as_ref().unwrap().job.clone();
        let recording = h
            .state()
            .session
            .export_current_recording_privileged()
            .unwrap();
        h.state_mut().start_practice(recording).unwrap();
        assert_eq!(h.state().practice_attempt(), Some(1));
        // Practice spawns its own work; the live analysis keeps running.
        for _ in 0..200 {
            if !h.state().session.opponent_to_move() {
                break;
            }
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!observed_cancel.is_cancelled());
        let practice_view = h.state().session.view();
        assert!(practice_view.practice);
        // A new attempt is fresh: before bidding, attempt 2, settings kept.
        h.get_by_role_and_label(egui::Role::Button, "Neuer Versuch")
            .click();
        h.run_steps(1);
        assert_eq!(h.state().practice_attempt(), Some(2));
        assert!(h.state().session.view().bidding.len() <= 1);
        assert_eq!(h.state().session.coach_settings(), before.coach.settings);
        // The live result arrives while practising; it is delivered on return.
        let result = job.run(None);
        let mut control = h.state().live_session().clone();
        control.finish_coach(result.clone()).unwrap();
        sender.send(result.clone()).unwrap();
        h.run_steps(2);
        assert_eq!(
            h.state_mut().session.finish_coach(result),
            Err(AppError::StaleCoachResult)
        );
        assert_eq!(h.state().live_session().view(), before);
        h.get_by_role_and_label(egui::Role::Button, "Zurück zur Serie")
            .click();
        h.run_steps(4);
        // Received and applied: identical to delivering it without practice.
        assert!(h.state().coach_thinking.is_none());
        assert_eq!(h.state().session.view(), control.view());
    }

    #[test]
    fn a_held_reveal_ends_with_release_and_with_the_game() {
        let game = |number| GameKey {
            seed: 3,
            number,
            practice: None,
        };
        let mut hold = None;
        assert!(!hold_reveals(&mut hold, false, game(1)));
        assert!(hold_reveals(&mut hold, true, game(1)));
        assert!(hold_reveals(&mut hold, true, game(1)));
        // The next game starts while the button is still held: concealed
        // until it is released and pressed again.
        assert!(!hold_reveals(&mut hold, true, game(2)));
        assert!(!hold_reveals(&mut hold, true, game(2)));
        assert!(!hold_reveals(&mut hold, false, game(2)));
        assert!(hold_reveals(&mut hold, true, game(2)));
        // A practice attempt of the same game number is another game.
        let practice = GameKey {
            practice: Some(1),
            ..game(2)
        };
        assert!(!hold_reveals(&mut hold, true, practice));
        assert!(!hold_reveals(&mut hold, false, practice));
        assert_eq!(hold, None);
    }

    #[test]
    fn seeds_are_whole_numbers_within_u64() {
        assert_eq!(parse_seed(" 0 "), Some(0));
        assert_eq!(parse_seed("18446744073709551615"), Some(u64::MAX));
        for invalid in ["", " -1", "18446744073709551616", "1.5", "abc", "1 2"] {
            assert_eq!(parse_seed(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn trick_cards_lie_towards_their_players() {
        let (human, left, right) = (rect(PlayerId::HUMAN), rect(PlayerId(1)), rect(PlayerId(2)));
        assert!(left.max.x < human.min.x && human.max.x < right.min.x);
        assert!(human.min.y > left.min.y && human.min.y > right.min.y);
        assert_eq!(left.min.y, right.min.y);
        assert_eq!(human.center().x, trick_area().x / 2.0);
    }

    #[test]
    fn trick_cards_and_names_fit_without_overlap() {
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, trick_area());
        let with_name = |p| {
            let r = rect(p);
            egui::Rect::from_min_max(r.min, r.max + Vec2::new(0.0, TRICK_LABEL_HEIGHT))
        };
        let all = PlayerId::ALL.map(with_name);
        for (i, a) in all.iter().enumerate() {
            assert!(area.contains_rect(*a), "{i} leaves the area");
            for b in &all[i + 1..] {
                assert!(!a.intersects(*b), "{a:?} overlaps {b:?}");
            }
        }
    }
}
