// SPDX-License-Identifier: EUPL-1.2

//! The learning history window (M8 foundation, D-70): records the
//! retrieval events of the running session or practice attempt, keeps them
//! in one local file and shows descriptive counts in German and English.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText};
use skat_app::{
    AttemptId, AttemptKind, LearningContext, LearningError, LearningFileError, LearningHistory,
    LearningTopic, Observations, RetrievalEvent,
};

use crate::i18n::I18n;

/// File name of the learning history inside the data directory.
pub const HISTORY_FILE: &str = "learning-history.json";

/// The default data directory: `GRAND_COACH_DATA` if set; on Windows
/// `%APPDATA%\Grand-Coach`; elsewhere `$XDG_DATA_HOME/grand-coach` or
/// `~/.local/share/grand-coach`. `None` if none can be determined; the
/// history then stays in memory.
pub fn data_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if let Some(dir) = var("GRAND_COACH_DATA") {
        return Some(dir.into());
    }
    if cfg!(windows) {
        return var("APPDATA").map(|d| PathBuf::from(d).join("Grand-Coach"));
    }
    var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
        .or_else(|| var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .map(|d| d.join("grand-coach"))
}

/// The history, its file and what happened to it.
#[derive(Default)]
pub(crate) struct LearningUi {
    history: LearningHistory,
    /// Where the history is saved; `None`: memory only.
    path: Option<PathBuf>,
    /// The file could not be read; it is kept unchanged and never saved
    /// over until the learner sets it aside.
    damaged: Option<LearningFileError>,
    /// The last save failed.
    save_error: Option<LearningFileError>,
    /// Where a damaged file was moved.
    set_aside: Option<PathBuf>,
    open: bool,
    confirm_reset: bool,
}

impl LearningUi {
    /// Uses the history file at `path`: an existing valid file is loaded, a
    /// missing one starts empty, a damaged or unsupported one is kept and
    /// reported.
    pub(crate) fn use_file(&mut self, path: PathBuf) {
        self.damaged = None;
        self.save_error = None;
        self.set_aside = None;
        match LearningHistory::open_local(&path) {
            Ok(history) => self.history = history,
            Err(LearningFileError::Missing) => self.history = LearningHistory::new(),
            Err(error) => {
                self.history = LearningHistory::new();
                self.damaged = Some(error);
            }
        }
        self.path = Some(path);
    }

    pub(crate) fn history(&self) -> &LearningHistory {
        &self.history
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn show(&mut self) {
        self.open = true;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Records `events` of the attempt in `attempt`, starting one if needed
    /// (also after a reset, which forgets every attempt).
    pub(crate) fn record(
        &mut self,
        attempt: &mut Option<AttemptId>,
        kind: AttemptKind,
        events: &[RetrievalEvent],
    ) {
        if events.is_empty() {
            return;
        }
        let mut result = match *attempt {
            Some(id) => self.history.record(id, events),
            None => Err(LearningError::UnknownAttempt),
        };
        if result == Err(LearningError::UnknownAttempt) {
            result = self.history.begin_attempt(kind).and_then(|id| {
                *attempt = Some(id);
                self.history.record(id, events)
            });
        }
        if result.is_ok_and(|r| r.added > 0) {
            self.save();
        }
    }

    fn save(&mut self) {
        if self.damaged.is_some() {
            return;
        }
        if let Some(path) = &self.path {
            self.save_error = self.history.save_local(path).err();
        }
    }

    /// Deliberate reset after confirmation.
    fn reset(&mut self) {
        self.history.reset();
        self.save();
    }

    /// Moves the damaged file aside and saves the history to the path.
    fn recover(&mut self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        match LearningHistory::set_aside(&path) {
            Ok(aside) => self.set_aside = Some(aside),
            // Already gone (removed outside the app): nothing to keep.
            Err(LearningFileError::Missing) => {}
            Err(error) => {
                // Still damaged and unchanged; report and keep refusing to save.
                self.save_error = Some(error);
                return;
            }
        }
        self.damaged = None;
        self.save_error = None;
        self.save();
    }

    pub(crate) fn window(&mut self, ctx: &egui::Context, i: &I18n) {
        if !self.open {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(i.t("learning-title"))
            .id(egui::Id::unique("learning_history"))
            .open(&mut open)
            .default_pos(egui::pos2(60.0, 80.0))
            .default_width(720.0)
            .max_width((ctx.content_rect().width() - 80.0).max(320.0))
            .max_height((ctx.content_rect().height() - 100.0).max(300.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.storage(ui, i);
                    ui.separator();
                    self.overview(ui, i);
                    ui.separator();
                    self.reset_controls(ui, i);
                    ui.separator();
                    if ui.button(i.t("learning-close")).clicked() {
                        close = true;
                    }
                });
            });
        if !open || close {
            self.open = false;
            self.confirm_reset = false;
        }
    }

    fn storage(&mut self, ui: &mut egui::Ui, i: &I18n) {
        let warn = Color32::from_rgb(170, 40, 30);
        match (&self.path, self.damaged) {
            (None, _) => {
                ui.label(i.t("learning-memory-only"));
            }
            (Some(path), Some(error)) => {
                ui.label(
                    RichText::new(i.t(file_error_message(error)))
                        .color(warn)
                        .strong(),
                );
                ui.label(i.tf(
                    "learning-damaged-kept",
                    &[("path", path.display().to_string().into())],
                ));
                if ui.button(i.t("learning-set-aside")).clicked() {
                    self.recover();
                }
            }
            (Some(path), None) => {
                ui.label(i.tf(
                    "learning-file",
                    &[("path", path.display().to_string().into())],
                ));
                ui.label(i.t("learning-retention"));
            }
        }
        if let Some(aside) = &self.set_aside {
            ui.label(i.tf(
                "learning-set-aside-done",
                &[("path", aside.display().to_string().into())],
            ));
        }
        if let Some(error) = self.save_error {
            ui.label(
                RichText::new(i.tf(
                    "learning-save-failed",
                    &[("reason", i.t(file_error_message(error)).into())],
                ))
                .color(warn),
            );
        }
    }

    fn overview(&self, ui: &mut egui::Ui, i: &I18n) {
        let summary = self.history.summary();
        ui.label(RichText::new(i.t("learning-descriptive")).strong());
        match summary.observations {
            Observations::None => {
                ui.label(i.t("learning-empty"));
            }
            Observations::Limited => {
                ui.label(i.tf(
                    "learning-limited",
                    &[("count", summary.totals.opportunities.into())],
                ));
            }
            Observations::Several => {
                ui.label(i.tf(
                    "learning-observed",
                    &[
                        ("count", summary.totals.opportunities.into()),
                        ("attempts", summary.attempts.into()),
                    ],
                ));
            }
        }
        if summary.full {
            ui.label(RichText::new(i.t("learning-full")).color(Color32::from_rgb(170, 40, 30)));
        }
        if summary.rows.is_empty() {
            return;
        }
        egui::Grid::new("learning_rows")
            .striped(true)
            .num_columns(7)
            .show(ui, |ui| {
                for id in [
                    "learning-col-topic",
                    "learning-col-context",
                    "learning-col-opportunities",
                    "learning-col-answers",
                    "learning-col-unaided",
                    "learning-col-help",
                    "learning-col-skips",
                ] {
                    ui.label(RichText::new(i.t(id)).strong());
                }
                ui.end_row();
                for row in &summary.rows {
                    let c = row.counts;
                    ui.label(i.t(topic_message(row.topic)));
                    ui.label(i.t(context_message(row.context)));
                    ui.label(c.opportunities.to_string());
                    ui.label(i.tf(
                        "learning-of-exact",
                        &[("count", c.answers.into()), ("exact", c.correct.into())],
                    ));
                    ui.label(i.tf(
                        "learning-of-exact",
                        &[
                            ("count", c.unaided_answers.into()),
                            ("exact", c.unaided_correct.into()),
                        ],
                    ));
                    ui.label(c.help.to_string());
                    ui.label(c.skips.to_string());
                    ui.end_row();
                }
            });
        ui.label(i.t("learning-legend"));
    }

    fn reset_controls(&mut self, ui: &mut egui::Ui, i: &I18n) {
        if !self.confirm_reset {
            if ui
                .add_enabled(
                    !self.history.is_empty(),
                    egui::Button::new(i.t("learning-reset")),
                )
                .clicked()
            {
                self.confirm_reset = true;
            }
            return;
        }
        ui.label(RichText::new(i.t("learning-reset-confirm")).strong());
        ui.horizontal(|ui| {
            if ui.button(i.t("learning-reset-yes")).clicked() {
                self.reset();
                self.confirm_reset = false;
            }
            if ui.button(i.t("learning-reset-no")).clicked() {
                self.confirm_reset = false;
            }
        });
    }
}

fn file_error_message(error: LearningFileError) -> &'static str {
    match error {
        LearningFileError::InvalidJson | LearningFileError::Invalid(_) => "learning-error-invalid",
        LearningFileError::UnsupportedFormat => "learning-error-format",
        LearningFileError::TooLarge => "learning-error-large",
        LearningFileError::Missing | LearningFileError::Io => "learning-error-io",
    }
}

fn topic_message(topic: LearningTopic) -> &'static str {
    match topic {
        LearningTopic::TrumpfOutside => "learning-topic-trumpf",
        LearningTopic::ProvenVoids => "learning-topic-voids",
        LearningTopic::AugenWon => "learning-topic-augen",
        LearningTopic::ClassPlayed => "learning-topic-played",
    }
}

fn context_message(context: LearningContext) -> &'static str {
    match context {
        LearningContext::Series => "learning-context-series",
        LearningContext::Practice => "learning-context-practice",
        LearningContext::PracticeRetry => "learning-context-retry",
    }
}
