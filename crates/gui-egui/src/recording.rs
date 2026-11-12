// SPDX-License-Identifier: EUPL-1.2

//! Detached read-only recording windows. No commands or analysis jobs.

use eframe::egui::{self, RichText, Vec2};
use skat_app::{
    Card, GameRecording, PlayerId, PrivilegedRecordingPosition, RecordingEnd, RecordingFileError,
    Session,
};

use crate::{
    cards::{self, Deck},
    i18n::I18n,
    text,
};

#[derive(Default)]
pub(crate) struct RecordingUi {
    pub dialog: Option<FileDialog>,
    pub replay: Option<Replay>,
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

    pub fn windows(&mut self, ctx: &egui::Context, i: &I18n, deck: Deck) {
        self.replay_window(ctx, i, deck);
        let Some(mut dialog) = self.dialog.take() else {
            return;
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
                                self.replay = Some(Replay {
                                    recording,
                                    position,
                                    boundaries,
                                    selected,
                                    reveal: false,
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
    }

    fn replay_window(&mut self, ctx: &egui::Context, i: &I18n, deck: Deck) {
        let Some(replay) = &mut self.replay else {
            return;
        };
        let mut open = true;
        let mut close = false;
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
                }
                egui::ScrollArea::vertical()
                    .id_salt("record_scroll")
                    .max_height((ctx.content_rect().height() - 340.0).max(120.0))
                    .show(ui, |ui| {
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
        if !open || close {
            self.replay = None;
        }
    }
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

fn card_row(ui: &mut egui::Ui, i: &I18n, deck: Deck, cards: impl IntoIterator<Item = Card>) {
    ui.horizontal_wrapped(|ui| {
        for card in cards {
            ui.add(
                egui::Image::new(cards::image(&cards::face_path(deck, card, i.faces.theme)))
                    .fit_to_exact_size(Vec2::new(44.0, 66.0)),
            )
            .on_hover_text(text::card(i, card));
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
