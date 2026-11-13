// SPDX-License-Identifier: EUPL-1.2

//! Detached read-only recording windows. No game commands; the viewer can
//! request a separate practice attempt from the app and, on demand, one
//! comment on a recorded own decision (D-67), computed off the UI thread.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use eframe::egui::{self, RichText, Vec2};
use skat_app::{
    Budget, Cancel, Card, GameRecording, PlayerId, PrivilegedRecordingPosition,
    REPLAY_COACH_TIME_LIMIT, REPLAY_REVIEW_DECISION_LIMIT, REPLAY_REVIEW_TIME_LIMIT, RecordingEnd,
    RecordingFileError, ReplayCoach, ReplayCoachResult, ReplayPerspective, ReplayReviewResult,
    ReplayStatus, ReviewStatus, ReviewUnavailable, Session, TableView,
};

use crate::{
    cards::{self, Deck},
    coach::{self, Line, Style},
    i18n::I18n,
    text,
};

#[derive(Default)]
pub(crate) struct RecordingUi {
    pub dialog: Option<FileDialog>,
    pub replay: Option<Replay>,
    /// Budget of replay comments opened from now on (the coach budget).
    pub budget: Budget,
}

/// The one running replay comment. Dropping it cancels the analysis and
/// disconnects its channel, so a late result cannot arrive.
struct ReplayThinking {
    result: Receiver<ReplayCoachResult>,
    cancel: Cancel,
}

impl Drop for ReplayThinking {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// The one running decision of the recording review (D-69). Dropping it
/// cancels that analysis and disconnects its channel.
struct ReviewThinking {
    result: Receiver<ReplayReviewResult>,
    cancel: Cancel,
}

impl Drop for ReviewThinking {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

pub(crate) struct FileDialog {
    save: bool,
    path: String,
    selected: usize,
    sources: Vec<GameRecording>,
    disclose: bool,
    error: Option<RecordingFileError>,
}

pub(crate) struct Replay {
    recording: GameRecording,
    position: PrivilegedRecordingPosition,
    boundaries: Vec<usize>,
    selected: usize,
    reveal: bool,
    /// The last practice start was rejected; the viewer stays open.
    practice_error: bool,
    /// On-demand comments on recorded own decisions (D-67).
    coach: ReplayCoach,
    thinking: Option<ReplayThinking>,
    /// The running review decision and when the review (re)started.
    reviewing: Option<ReviewThinking>,
    review_started: Option<Instant>,
}

/// A replay-coach control used in this frame.
enum CoachRequest {
    Select(Option<usize>),
    Request,
    Perspective(ReplayPerspective),
    Cancel,
    /// Start, continue or repeat the recording review.
    Review,
    CancelReview,
}

impl RecordingUi {
    pub fn active(&self) -> bool {
        self.dialog.is_some() || self.replay.is_some()
    }

    pub fn show_dialog(&mut self, save: bool, session: &Session) {
        let mut sources = Vec::new();
        if let Some(current) = session.export_current_recording_privileged() {
            sources.push(current);
        }
        sources.extend_from_slice(session.completed_recordings_privileged());
        self.dialog = Some(FileDialog {
            save,
            sources,
            selected: 0,
            path: String::new(),
            disclose: false,
            error: None,
        });
    }

    /// Draws the windows. Returns the recording of a requested practice
    /// start ([`skat_app::Session::start_practice`] validates it again);
    /// `can_practice` is `false` while an attempt is running.
    pub fn windows(
        &mut self,
        ctx: &egui::Context,
        i: &I18n,
        deck: Deck,
        can_practice: bool,
        live: &TableView,
    ) -> Option<GameRecording> {
        let practice = self.replay_window(ctx, i, deck, can_practice, live);
        let Some(mut dialog) = self.dialog.take() else {
            return practice;
        };
        let mut open = true;
        let mut done = false;
        let title = i.t(if dialog.save {
            "record-save"
        } else {
            "record-open"
        });
        egui::Window::new(title)
            .id(egui::Id::unique("record_file"))
            .open(&mut open)
            .default_pos(egui::pos2(80.0, 100.0))
            .default_width(520.0)
            .max_width((ctx.content_rect().width() - 120.0).max(320.0))
            .show(ctx, |ui| {
                ui.label(i.t("record-file-help"));
                if dialog.save {
                    egui::ComboBox::from_label(i.t("record-source"))
                        .selected_text(source_label(i, &dialog, dialog.selected))
                        .show_ui(ui, |ui| {
                            for n in 0..dialog.sources.len() {
                                let label = source_label(i, &dialog, n);
                                ui.selectable_value(&mut dialog.selected, n, label);
                            }
                        });
                    ui.label(i.t("record-export-warning"));
                    ui.checkbox(&mut dialog.disclose, i.t("record-export-consent"));
                }
                ui.horizontal(|ui| {
                    let label = ui.label(i.t("record-path"));
                    ui.text_edit_singleline(&mut dialog.path)
                        .labelled_by(label.id);
                });
                if let Some(error) = dialog.error {
                    ui.colored_label(egui::Color32::from_rgb(200, 40, 40), i.t(error_key(error)));
                }
                ui.horizontal(|ui| {
                    let enabled = !dialog.path.is_empty()
                        && (!dialog.save || (dialog.disclose && !dialog.sources.is_empty()));
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(i.t(if dialog.save {
                                "record-save-file"
                            } else {
                                "record-open-file"
                            })),
                        )
                        .clicked()
                    {
                        let result = if dialog.save {
                            dialog.sources[dialog.selected].save_local_new(&dialog.path)
                        } else {
                            GameRecording::open_local(&dialog.path).map(|recording| {
                                // open_local has validated the whole file. Publish only now.
                                let boundaries =
                                    recording.trick_boundaries().expect("validated file");
                                let selected = boundaries.len() - 1;
                                let position = recording
                                    .reconstruct_privileged(boundaries[selected])
                                    .expect("validated file");
                                // A new recording always starts unselected
                                // in the historical perspective.
                                let coach = ReplayCoach::open(&recording, self.budget)
                                    .expect("validated file");
                                self.replay = Some(Replay {
                                    recording,
                                    position,
                                    boundaries,
                                    selected,
                                    reveal: false,
                                    practice_error: false,
                                    coach,
                                    thinking: None,
                                    reviewing: None,
                                    review_started: None,
                                });
                            })
                        };
                        match result {
                            Ok(()) => done = true,
                            Err(error) => dialog.error = Some(error),
                        }
                    }
                    if ui.button(i.t("button-cancel")).clicked() {
                        done = true;
                    }
                });
            });
        if open && !done {
            self.dialog = Some(dialog);
        }
        practice
    }

    /// A requested practice start was rejected: keep the viewer and say so.
    pub fn practice_failed(&mut self) {
        if let Some(replay) = &mut self.replay {
            replay.practice_error = true;
        }
    }

    /// Closes every recording window (a practice attempt has started).
    pub fn close(&mut self) {
        self.dialog = None;
        self.replay = None;
    }

    /// `true` while a replay comment is being computed (tests).
    pub fn replay_computing(&self) -> bool {
        self.replay.as_ref().is_some_and(|r| r.thinking.is_some())
    }

    /// `true` while the recording review runs (tests).
    pub fn review_running(&self) -> bool {
        self.replay.as_ref().is_some_and(|r| r.reviewing.is_some())
    }

    /// The replay coach of the open recording (tests, screenshots).
    pub fn replay_coach_mut(&mut self) -> Option<&mut ReplayCoach> {
        self.replay.as_mut().map(|r| &mut r.coach)
    }

    fn replay_window(
        &mut self,
        ctx: &egui::Context,
        i: &I18n,
        deck: Deck,
        can_practice: bool,
        live: &TableView,
    ) -> Option<GameRecording> {
        let replay = self.replay.as_mut()?;
        // The focus selects moments only; it is no analysis input (D-69).
        replay.coach.set_review_focus(live.coach.settings.focus);
        replay.poll(ctx);
        let mut open = true;
        let mut close = false;
        let mut practice = None;
        let mut request = None;
        egui::Window::new(i.t("record-title"))
            .id(egui::Id::unique("record_view"))
            .open(&mut open)
            .default_pos(egui::pos2(40.0, 60.0))
            .default_width(760.0)
            .max_width((ctx.content_rect().width() - 80.0).max(320.0))
            .default_height(600.0)
            .max_height((ctx.content_rect().height() - 100.0).max(300.0))
            .show(ctx, |ui| {
                ui.label(RichText::new(i.t("record-isolation")).strong());
                ui.label(i.tf(
                    "record-identity",
                    &[
                        ("number", replay.recording.context.number.into()),
                        ("seed", replay.recording.seed.to_string().into()),
                        (
                            "version",
                            replay.recording.context.application_version.clone().into(),
                        ),
                    ],
                ));
                ui.label(i.t(match replay.recording.end {
                    RecordingEnd::InProgress => "record-incomplete",
                    RecordingEnd::Eingepasst => "record-all-pass",
                    RecordingEnd::Finished(_) => "record-finished",
                }));
                ui.label(i.t("record-practice-help"));
                if replay.practice_error {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 40, 40),
                        i.t("record-practice-error"),
                    );
                }
                ui.horizontal(|ui| {
                    let start = ui
                        .add_enabled(
                            can_practice,
                            egui::Button::new(RichText::new(i.t("record-practice-start")).strong()),
                        )
                        .on_disabled_hover_text(i.t("record-practice-unavailable"));
                    if start.clicked() {
                        practice = Some(replay.recording.clone());
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let last = replay.boundaries.len() - 1;
                    let mut next = replay.selected;
                    for (key, target, enabled) in [
                        ("record-first", 0, next > 0),
                        ("record-previous", next.saturating_sub(1), next > 0),
                        ("record-next", (next + 1).min(last), next < last),
                        ("record-last", last, next < last),
                    ] {
                        if ui
                            .add_enabled(enabled, egui::Button::new(i.t(key)))
                            .clicked()
                        {
                            next = target;
                        }
                    }
                    if next != replay.selected {
                        replay.position = replay
                            .recording
                            .reconstruct_privileged(replay.boundaries[next])
                            .expect("immutable validated record");
                        replay.selected = next;
                    }
                    ui.label(i.tf(
                        "record-boundary",
                        &[("count", replay.position.applied_actions.into())],
                    ));
                });
                ui.checkbox(&mut replay.reveal, i.t("record-reveal"));
                if replay.reveal {
                    ui.label(RichText::new(i.t("record-privileged")).strong());
                    ui.label(RichText::new(i.t("record-reveal-note")).small());
                }
                egui::ScrollArea::vertical()
                    .id_salt("record_scroll")
                    .max_height((ctx.content_rect().height() - 340.0).max(120.0))
                    .show(ui, |ui| {
                        request = review_section(ui, i, deck, &replay.coach, live);
                        ui.separator();
                        if let Some(r) = coach_section(ui, i, deck, &replay.coach, live) {
                            request = Some(r);
                        }
                        ui.separator();
                        let state = &replay.position.state;
                        let public = &state.seats[0];
                        ui.label(text::phase(i, public.phase));
                        if let Some(contract) = public.contract {
                            ui.label(text::contract(i, contract));
                        }
                        if let Some(level) = text::kontra(i, public.kontra) {
                            ui.label(level);
                        }
                        if let Some(result) = &public.result {
                            let player = PlayerId::on_seat(
                                result.outcome.declarer,
                                replay.recording.context.dealer,
                            );
                            for line in text::result_lines(i, player, result) {
                                ui.label(line);
                            }
                        }
                        ui.collapsing(i.t("record-bidding"), |ui| {
                            for bid in &public.bidding {
                                ui.label(format!(
                                    "{}: {}",
                                    text::seat(i, bid.seat),
                                    text::call(i, bid.call)
                                ));
                            }
                        });
                        if replay.reveal {
                            ui.heading(i.t("record-hands"));
                            for seat in skat_app::Seat::ALL {
                                ui.label(text::seat(i, seat));
                                card_row(ui, i, deck, state.hands[seat.index()]);
                            }
                            ui.label(i.t("label-skat"));
                            card_row(ui, i, deck, state.skat);
                            ui.collapsing(i.t("record-deal"), |ui| {
                                for seat in skat_app::Seat::ALL {
                                    ui.label(text::seat(i, seat));
                                    card_row(ui, i, deck, state.original_hands[seat.index()]);
                                }
                                ui.label(i.t("label-skat"));
                                card_row(ui, i, deck, state.original_skat);
                            });
                        }
                        ui.heading(i.t("record-tricks"));
                        if public.tricks.is_empty() {
                            ui.label(i.t("record-no-tricks"));
                        }
                        for (n, trick) in public.tricks.iter().enumerate() {
                            ui.label(i.tf(
                                "record-trick",
                                &[
                                    ("number", (n + 1).into()),
                                    ("leader", text::seat(i, trick.leader).into()),
                                    ("winner", text::seat(i, trick.winner).into()),
                                ],
                            ));
                            card_row(ui, i, deck, trick.cards);
                        }
                        if !public.current_trick.is_empty() {
                            ui.label(i.t("record-current-trick"));
                            card_row(
                                ui,
                                i,
                                deck,
                                public.current_trick.cards.into_iter().flatten(),
                            );
                        }
                    });
                if ui.button(i.t("record-close")).clicked() {
                    close = true;
                }
            });
        if let Some(request) = request {
            replay.apply(request, ctx);
        }
        if !open || close {
            // Dropping the replay cancels its running comment.
            self.replay = None;
        }
        practice
    }
}

impl Replay {
    /// Delivers a finished comment or review decision; the coach rejects
    /// anything stale. The review then starts its next decision, or ends at
    /// [`REPLAY_REVIEW_TIME_LIMIT`].
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(reviewing) = &self.reviewing {
            match reviewing.result.try_recv() {
                Ok(result) => {
                    self.reviewing = None;
                    let _ = self.coach.finish_review(result);
                    if self
                        .review_started
                        .is_some_and(|t| t.elapsed() >= REPLAY_REVIEW_TIME_LIMIT)
                    {
                        self.coach.review_time_limit();
                    } else {
                        self.start_review(ctx);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.reviewing = None;
                    self.coach.cancel_review();
                }
            }
        }
        let Some(thinking) = &self.thinking else {
            return;
        };
        match thinking.result.try_recv() {
            Ok(result) => {
                self.thinking = None;
                let _ = self.coach.finish(result);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.thinking = None;
                self.coach.cancel();
            }
        }
    }

    /// Applies a control. Every single-comment change drops the running
    /// comment first; selecting a decision does not stop the review.
    fn apply(&mut self, request: CoachRequest, ctx: &egui::Context) {
        match request {
            CoachRequest::Review => {
                if self.thinking.is_none() && self.coach.request_review() {
                    self.review_started = Some(Instant::now());
                    self.start_review(ctx);
                }
                return;
            }
            CoachRequest::CancelReview => {
                self.reviewing = None;
                self.coach.cancel_review();
                return;
            }
            _ => {}
        }
        self.thinking = None;
        match request {
            CoachRequest::Select(index) => self.coach.select(index),
            CoachRequest::Cancel => self.coach.cancel(),
            CoachRequest::Perspective(p) => {
                self.coach.set_perspective(p);
                self.start(ctx);
            }
            CoachRequest::Request => self.start(ctx),
            CoachRequest::Review | CoachRequest::CancelReview => {}
        }
    }

    /// Starts the next review decision on a background thread, bounded by
    /// the budget, [`REPLAY_REVIEW_DECISION_LIMIT`] and what is left of
    /// [`REPLAY_REVIEW_TIME_LIMIT`]. Ends quietly when none is left.
    fn start_review(&mut self, ctx: &egui::Context) {
        let Some(job) = self.coach.review_job() else {
            return;
        };
        let started = self.review_started.unwrap_or_else(Instant::now);
        let deadline =
            (Instant::now() + REPLAY_REVIEW_DECISION_LIMIT).min(started + REPLAY_REVIEW_TIME_LIMIT);
        let (sender, result) = std::sync::mpsc::channel();
        let cancel = Cancel::with_deadline(deadline);
        let (stop, repaint) = (cancel.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = sender.send(job.run(Some(&stop)));
            repaint.request_repaint();
        });
        self.reviewing = Some(ReviewThinking { result, cancel });
    }

    /// Starts the one replay analysis on a background thread, bounded by
    /// its budget and [`REPLAY_COACH_TIME_LIMIT`].
    fn start(&mut self, ctx: &egui::Context) {
        let Some(job) = self.coach.request() else {
            return;
        };
        let (sender, result) = std::sync::mpsc::channel();
        let cancel = Cancel::with_deadline(Instant::now() + REPLAY_COACH_TIME_LIMIT);
        let (stop, repaint) = (cancel.clone(), ctx.clone());
        std::thread::spawn(move || {
            // The receiver is gone if the request was replaced meanwhile.
            let _ = sender.send(job.run(Some(&stop)));
            repaint.request_repaint();
        });
        self.thinking = Some(ReplayThinking { result, cancel });
    }
}

/// The recording review (D-69): request, progress and cancel, scope, and
/// zero to three moments, each navigable to its decision.
fn review_section(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    coach: &ReplayCoach,
    live: &TableView,
) -> Option<CoachRequest> {
    let mut request = None;
    ui.heading(i.t("replay-review-title"));
    let status = coach.review_status();
    if let ReviewStatus::Unavailable(reason) = status {
        ui.label(i.t(match reason {
            ReviewUnavailable::Incomplete => "replay-review-incomplete",
            ReviewUnavailable::NoDecisions => "replay-review-no-decisions",
        }));
        return None;
    }
    ui.label(RichText::new(i.t("replay-review-help")).small());
    let single_busy = coach.computing();
    let start = |ui: &mut egui::Ui, key: &str| {
        ui.add_enabled(!single_busy, egui::Button::new(i.t(key)))
            .on_disabled_hover_text(i.t("replay-review-single-busy"))
            .clicked()
    };
    match status {
        ReviewStatus::Unavailable(_) => {}
        ReviewStatus::Idle { eligible } => {
            ui.label(i.tf(
                "replay-review-eligible",
                &[
                    ("count", eligible.into()),
                    ("total", REPLAY_REVIEW_TIME_LIMIT.as_secs().into()),
                    ("each", REPLAY_REVIEW_DECISION_LIMIT.as_secs().into()),
                ],
            ));
            if start(ui, "replay-review-request") {
                request = Some(CoachRequest::Review);
            }
        }
        ReviewStatus::Running {
            attempted,
            eligible,
        } => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(i.tf(
                    "replay-review-running",
                    &[
                        ("current", (attempted + 1).min(eligible).into()),
                        ("eligible", eligible.into()),
                    ],
                ));
                if ui.button(i.t("replay-review-cancel")).clicked() {
                    request = Some(CoachRequest::CancelReview);
                }
            });
        }
        ReviewStatus::Done(summary) => {
            show_lines(ui, &coach::review_summary(i, &summary));
            let key = if summary.exhaustive() {
                "replay-review-again"
            } else {
                "replay-review-continue"
            };
            if start(ui, key) {
                request = Some(CoachRequest::Review);
            }
            let figures = live.coach.settings.show_figures;
            for (n, m) in summary.moments.iter().enumerate() {
                let text = coach::review_moment(i, live, m, figures);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    show_lines(ui, std::slice::from_ref(&text.title));
                    situation(ui, i, deck, &text.situation, ("review", n));
                    show_lines(ui, &text.summary);
                    egui::CollapsingHeader::new(&text.lesson_title)
                        .id_salt(("review_lesson", n))
                        .show(ui, |ui| show_lines(ui, &text.lesson));
                    egui::CollapsingHeader::new(i.t("coach-details"))
                        .id_salt(("review_details", n))
                        .show(ui, |ui| show_lines(ui, &text.details));
                    if ui.button(i.t("replay-review-show")).clicked() {
                        request = Some(CoachRequest::Select(Some(m.decision)));
                    }
                });
            }
        }
    }
    request
}

/// The replay-coach section: decision selector, persistent perspective
/// label, controls, the table before the action and the comment.
fn coach_section(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    coach: &ReplayCoach,
    live: &TableView,
) -> Option<CoachRequest> {
    let mut request = None;
    let view = coach.view();
    ui.heading(i.t("replay-coach-title"));
    if view.status == ReplayStatus::NoDecisions {
        ui.label(i.t("replay-coach-no-decisions"));
        return None;
    }
    ui.label(RichText::new(i.t("replay-coach-help")).small());
    let label = |n: Option<usize>| match n {
        Some(n) => coach::recorded_decision(i, &view.decisions[n]),
        None => i.t("replay-coach-none-selected"),
    };
    let mut selected = view.selected;
    egui::ComboBox::from_label(i.t("replay-coach-decision"))
        .selected_text(label(selected))
        .width(360.0)
        .show_ui(ui, |ui| {
            for n in 0..view.decisions.len() {
                ui.selectable_value(&mut selected, Some(n), label(Some(n)));
            }
        });
    if selected != view.selected {
        request = Some(CoachRequest::Select(selected));
    }
    let Some(context) = &view.context else {
        return request;
    };
    let banner = coach::replay_perspective(i, view.perspective);
    let color = match view.perspective {
        ReplayPerspective::Historical => egui::Color32::from_rgb(30, 90, 160),
        ReplayPerspective::Hindsight => egui::Color32::from_rgb(170, 70, 0),
    };
    egui::Frame::group(ui.style())
        .stroke(egui::Stroke::new(2.0, color))
        .show(ui, |ui| {
            ui.colored_label(color, RichText::new(banner.text).strong());
        });
    let computing = view.status == ReplayStatus::Computing;
    ui.horizontal_wrapped(|ui| {
        let again = matches!(
            view.status,
            ReplayStatus::Ready(_) | ReplayStatus::Unavailable(_)
        );
        let key = if again {
            "replay-coach-retry"
        } else {
            "replay-coach-request"
        };
        let review = coach.review_running();
        if ui
            .add_enabled(!computing && !review, egui::Button::new(i.t(key)))
            .on_disabled_hover_text(i.t("replay-review-busy"))
            .clicked()
        {
            request = Some(CoachRequest::Request);
        }
        let (key, other) = match view.perspective {
            ReplayPerspective::Historical => {
                ("replay-coach-hindsight", ReplayPerspective::Hindsight)
            }
            ReplayPerspective::Hindsight => {
                ("replay-coach-historical", ReplayPerspective::Historical)
            }
        };
        if ui
            .add_enabled(!review, egui::Button::new(i.t(key)))
            .on_disabled_hover_text(i.t("replay-review-busy"))
            .clicked()
        {
            request = Some(CoachRequest::Perspective(other));
        }
        if computing && ui.button(i.t("replay-coach-cancel")).clicked() {
            request = Some(CoachRequest::Cancel);
        }
    });
    let figures = live.coach.settings.show_figures;
    match &view.status {
        ReplayStatus::Idle => {
            ui.label(RichText::new(i.t("replay-coach-idle")).weak());
        }
        ReplayStatus::Computing => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(i.t("replay-coach-computing"));
            });
        }
        ReplayStatus::Unavailable(reason) => {
            ui.label(coach::unavailable(i, *reason));
        }
        ReplayStatus::Ready(comment) => {
            let text = coach::replay_comment(i, live, context, comment, figures);
            show_lines(ui, &text.summary);
            if !text.lesson_title.is_empty() {
                egui::CollapsingHeader::new(&text.lesson_title)
                    .id_salt("replay_lesson")
                    .show(ui, |ui| show_lines(ui, &text.lesson));
            }
            egui::CollapsingHeader::new(i.t("coach-details"))
                .id_salt("replay_details")
                .show(ui, |ui| show_lines(ui, &text.details));
        }
        ReplayStatus::NoDecisions | ReplayStatus::NotSelected => {}
    }
    ui.label(RichText::new(i.t("replay-coach-before")).strong());
    situation(ui, i, deck, &coach::replay_situation(i, context), "single");
    request
}

/// The table before the recorded action: facts, the hand with the chosen
/// cards marked and the trick in progress. Display only.
fn situation(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    s: &coach::SituationText,
    salt: impl std::hash::Hash + std::fmt::Debug,
) {
    show_lines(ui, &s.lines);
    ui.label(RichText::new(&s.hand_label).small());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for &card in &s.hand {
            let response = card_image(ui, i, deck, card);
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
    if !s.trick_label.is_empty() {
        ui.label(RichText::new(&s.trick_label).small());
        ui.horizontal(|ui| {
            for (player, card) in &s.trick {
                ui.vertical(|ui| {
                    card_image(ui, i, deck, *card);
                    ui.label(RichText::new(player).small());
                });
            }
        });
    }
    egui::CollapsingHeader::new(&s.history_title)
        .id_salt(("replay_history", salt))
        .show(ui, |ui| show_lines(ui, &s.history));
}

fn show_lines(ui: &mut egui::Ui, lines: &[Line]) {
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

pub(crate) fn card_image(ui: &mut egui::Ui, i: &I18n, deck: Deck, card: Card) -> egui::Response {
    ui.add(
        egui::Image::new(cards::image(&cards::face_path(deck, card, i.faces.theme)))
            .fit_to_exact_size(Vec2::new(44.0, 66.0)),
    )
    .on_hover_text(text::card(i, card))
}

fn source_label(i: &I18n, dialog: &FileDialog, n: usize) -> String {
    let Some(recording) = dialog.sources.get(n) else {
        return i.t("record-no-game");
    };
    if n == 0 {
        i.t("record-current")
    } else {
        i.tf(
            "record-completed",
            &[("number", recording.context.number.into())],
        )
    }
}

pub(crate) fn card_row(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    cards: impl IntoIterator<Item = Card>,
) {
    ui.horizontal_wrapped(|ui| {
        for card in cards {
            card_image(ui, i, deck, card);
        }
    });
}

fn error_key(error: RecordingFileError) -> &'static str {
    match error {
        RecordingFileError::InvalidJson => "record-error-json",
        RecordingFileError::UnsupportedFormat => "record-error-format",
        RecordingFileError::UnsupportedEngine => "record-error-engine",
        RecordingFileError::InvalidRecording(_) => "record-error-history",
        RecordingFileError::TooLarge => "record-error-large",
        RecordingFileError::Missing => "record-error-missing",
        RecordingFileError::AlreadyExists => "record-error-exists",
        RecordingFileError::Io => "record-error-io",
    }
}
