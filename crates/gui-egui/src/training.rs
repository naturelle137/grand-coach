// SPDX-License-Identifier: EUPL-1.2

//! The focused counting training screen (M8 foundation, D-71): replaces
//! the table while it is open, shows one fixed position from the intended
//! seat's view and records the training's events in the learning history.
//! The live session is neither changed nor driven meanwhile.

use eframe::egui::{self, Color32, RichText};
use skat_app::{
    AssistanceLevel, AttemptId, Derivation, Seat, TrainingCommand, TrainingFixture,
    TrainingSession, TrainingTopic, TrainingView,
};

use crate::assist;
use crate::cards::Deck;
use crate::i18n::I18n;
use crate::learning::LearningUi;
use crate::text;

/// The training screen and its state.
#[derive(Default)]
pub(crate) struct TrainingUi {
    open: bool,
    session: Option<TrainingSession>,
    /// Support chosen before a topic is started.
    assistance: AssistanceLevel,
    /// Number chosen for the answer.
    value: u8,
    /// Learning-history attempt of the current presentation.
    attempt: Option<AttemptId>,
    presented: Option<(TrainingFixture, u32)>,
}

/// What the screen asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrainingRequest {
    /// Leave the training and return to the waiting live table.
    Exit,
    /// Open the learning history window.
    History,
}

impl TrainingUi {
    pub(crate) fn active(&self) -> bool {
        self.open
    }

    /// Opens the topic choice.
    pub(crate) fn enter(&mut self) {
        self.open = true;
    }

    /// Leaves the training; its state is dropped.
    pub(crate) fn exit(&mut self) {
        *self = TrainingUi {
            assistance: self.assistance,
            ..TrainingUi::default()
        };
    }

    pub(crate) fn view(&self) -> Option<TrainingView> {
        self.session.as_ref().map(TrainingSession::view)
    }

    /// Runs `command` and records the resulting events, one learning
    /// attempt per presentation.
    fn run(&mut self, command: Option<TrainingCommand>, learning: &mut LearningUi) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Some(command) = command
            && session.execute(command).is_err()
        {
            return;
        }
        if !matches!(command, Some(TrainingCommand::Assistance(_)) | None) {
            self.value = 0;
        }
        let presented = session.presentation();
        if self.presented != Some(presented) {
            self.presented = Some(presented);
            self.attempt = None;
        }
        let events = session.take_events();
        learning.record(&mut self.attempt, session.attempt_kind(), &events);
    }

    fn start(&mut self, topic: TrainingTopic, learning: &mut LearningUi) {
        self.session = Some(TrainingSession::new(topic, self.assistance));
        self.run(None, learning);
    }

    /// Draws the screen in place of the table.
    pub(crate) fn screen(
        &mut self,
        ui: &mut egui::Ui,
        i: &I18n,
        deck: Deck,
        learning: &mut LearningUi,
    ) -> Option<TrainingRequest> {
        let mut request = None;
        let view = self.view();
        egui::Panel::top("training_banner")
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::from_rgb(214, 234, 248)))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let banner = match &view {
                        Some(v) => i.tf(
                            "training-banner",
                            &[
                                ("topic", i.t(topic_message(v.topic)).into()),
                                ("position", v.position.0.into()),
                                ("count", v.position.1.into()),
                                ("presentation", v.presentation.into()),
                            ],
                        ),
                        None => i.t("training-title"),
                    };
                    ui.label(RichText::new(banner).strong().color(BANNER_TEXT));
                    if ui.button(i.t("training-return")).clicked() {
                        request = Some(TrainingRequest::Exit);
                    }
                    if ui.button(i.t("learning-open")).clicked() {
                        request = Some(TrainingRequest::History);
                    }
                });
                ui.label(RichText::new(i.t("training-isolation")).color(BANNER_TEXT));
            });
        let mut command = None;
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match &view {
                None => {
                    if let Some(topic) = self.chooser(ui, i) {
                        self.start(topic, learning);
                    }
                }
                Some(v) => {
                    command = self.task(ui, i, deck, v);
                }
            });
        });
        if command.is_some() {
            self.run(command, learning);
        }
        request
    }

    /// The topic choice before a training starts.
    fn chooser(&mut self, ui: &mut egui::Ui, i: &I18n) -> Option<TrainingTopic> {
        let mut topic = None;
        ui.label(i.t("training-intro"));
        ui.add_space(6.0);
        ui.label(RichText::new(i.t("setting-retrieval-level")).strong());
        for (level, key) in LEVELS {
            ui.radio_value(&mut self.assistance, level, i.t(key));
        }
        ui.add_space(6.0);
        for t in TrainingTopic::ALL {
            ui.horizontal(|ui| {
                if ui.button(i.t(start_message(t))).clicked() {
                    topic = Some(t);
                }
                ui.label(RichText::new(i.t(topic_note(t))).weak());
            });
        }
        ui.add_space(6.0);
        ui.label(RichText::new(i.t("training-practice-only")).weak());
        topic
    }

    /// One task: the position on the left, the question on the right.
    fn task(
        &mut self,
        ui: &mut egui::Ui,
        i: &I18n,
        deck: Deck,
        v: &TrainingView,
    ) -> Option<TrainingCommand> {
        let mut command = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new(i.t("training-topic")).strong());
            for t in TrainingTopic::ALL {
                if ui
                    .selectable_label(v.topic == t, i.t(topic_message(t)))
                    .clicked()
                    && v.topic != t
                {
                    command = Some(TrainingCommand::Topic(t));
                }
            }
        });
        ui.separator();
        ui.columns(2, |columns| {
            situation(&mut columns[0], i, deck, v);
            if let Some(c) = self.question(&mut columns[1], i, v) {
                command = Some(c);
            }
        });
        command
    }

    fn question(
        &mut self,
        ui: &mut egui::Ui,
        i: &I18n,
        v: &TrainingView,
    ) -> Option<TrainingCommand> {
        let mut command = None;
        ui.label(RichText::new(i.t("training-question")).strong().size(15.0));
        ui.label(assist::question(i, v.question));
        ui.add_space(4.0);
        ui.label(RichText::new(i.t("setting-retrieval-level")).strong());
        let mut level = v.assistance;
        for (l, key) in LEVELS {
            ui.radio_value(&mut level, l, i.t(key));
        }
        if level != v.assistance {
            command = Some(TrainingCommand::Assistance(level));
        }
        ui.add_space(4.0);
        if let Some(feedback) = v.feedback {
            ui.label(RichText::new(i.t("training-result")).strong());
            for line in assist::feedback_lines(i, &feedback) {
                ui.label(line);
            }
            if let Some(line) = void_seats(i, v, feedback.derivation) {
                ui.label(line);
            }
            if v.assisted {
                ui.label(RichText::new(i.t("training-was-assisted")).weak());
            }
            ui.horizontal(|ui| {
                if ui.button(i.t("training-retry")).clicked() {
                    command = Some(TrainingCommand::Retry);
                }
                if ui.button(i.t("training-next")).clicked() {
                    command = Some(TrainingCommand::Next);
                }
            });
        } else {
            if let Some(help) = v.help {
                for line in assist::derivation_lines(i, help) {
                    ui.label(line);
                }
                if let Some(line) = void_seats(i, v, help) {
                    ui.label(line);
                }
                ui.label(RichText::new(i.t("retrieval-assisted")).weak());
            } else if ui.button(i.t("retrieval-help")).clicked() {
                command = Some(TrainingCommand::Help);
            }
            ui.add_space(4.0);
            ui.label(RichText::new(i.tf("quiz-value", &[("value", self.value.into())])).strong());
            let mut delta = 0i16;
            ui.horizontal(|ui| {
                for (id, step) in [
                    ("quiz-minus-ten", -10),
                    ("quiz-minus-one", -1),
                    ("quiz-plus-one", 1),
                    ("quiz-plus-ten", 10),
                ] {
                    if ui.button(i.t(id)).clicked() {
                        delta = step;
                    }
                }
            });
            self.value = (i16::from(self.value) + delta).clamp(0, 32) as u8;
            ui.horizontal(|ui| {
                if ui.button(i.t("quiz-submit")).clicked() {
                    command = Some(TrainingCommand::Answer(self.value));
                }
                if ui.button(i.t("quiz-skip")).clicked() {
                    command = Some(TrainingCommand::Skip);
                }
            });
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(i.tf(
                "training-fixture-note",
                &[("version", v.task.version.into())],
            ))
            .weak()
            .small(),
        );
        command
    }
}

/// Text on the light banner, readable in both themes.
const BANNER_TEXT: Color32 = Color32::from_rgb(16, 48, 80);

const LEVELS: [(AssistanceLevel, &str); 3] = [
    (AssistanceLevel::VisibleAid, "retrieval-visible"),
    (AssistanceLevel::RevealOnRequest, "retrieval-request"),
    (AssistanceLevel::AnswerBeforeReveal, "retrieval-answer"),
];

/// A seat with its role and whether it is the learner's.
fn seat_label(i: &I18n, v: &TrainingView, seat: Seat) -> String {
    let name = text::seat(i, seat);
    match (seat == v.seat, seat == v.declarer) {
        (true, true) => i.tf("training-seat-you-declarer", &[("seat", name.into())]),
        (true, false) => i.tf("training-seat-you", &[("seat", name.into())]),
        (false, true) => i.tf("training-seat-declarer", &[("seat", name.into())]),
        (false, false) => name,
    }
}

/// The seats with public proof of a void, named only from that proof.
fn void_seats(i: &I18n, v: &TrainingView, derivation: Derivation) -> Option<String> {
    let Derivation::ProvenVoids { seats, .. } = derivation else {
        return None;
    };
    let names: Vec<String> = Seat::ALL
        .into_iter()
        .filter(|s| seats[s.index()])
        .map(|s| seat_label(i, v, s))
        .collect();
    (!names.is_empty()).then(|| i.tf("training-void-seats", &[("seats", names.join(", ").into())]))
}

/// Contract, roles, past tricks, the trick on the table and the hand.
fn situation(ui: &mut egui::Ui, i: &I18n, deck: Deck, v: &TrainingView) {
    ui.label(
        RichText::new(i.tf(
            "training-contract",
            &[
                ("game", text::contract(i, v.contract).into()),
                ("bid", v.bid.into()),
                ("declarer", text::seat(i, v.declarer).into()),
            ],
        ))
        .strong(),
    );
    let seat = text::seat(i, v.seat);
    ui.label(if v.seat == v.declarer {
        i.tf("training-role-declarer", &[("seat", seat.into())])
    } else {
        i.tf(
            "training-role-defender",
            &[
                ("seat", seat.into()),
                ("declarer", text::seat(i, v.declarer).into()),
            ],
        )
    });
    ui.add_space(4.0);
    ui.label(RichText::new(i.t("training-tricks")).strong());
    for (n, trick) in v.tricks.iter().enumerate() {
        ui.label(
            RichText::new(i.tf(
                "training-trick",
                &[
                    ("number", (n + 1).into()),
                    ("leader", seat_label(i, v, trick.cards[0].0).into()),
                    ("winner", seat_label(i, v, trick.winner).into()),
                ],
            ))
            .small(),
        );
        played(ui, i, deck, v, &trick.cards);
    }
    ui.add_space(4.0);
    ui.label(RichText::new(i.t("training-table")).strong());
    if v.current_trick.is_empty() {
        ui.label(i.t("training-you-lead"));
    } else {
        played(ui, i, deck, v, &v.current_trick);
    }
    ui.label(RichText::new(i.t("training-your-turn")).weak());
    ui.add_space(4.0);
    ui.label(RichText::new(i.t("training-hand")).strong());
    crate::recording::card_row(ui, i, deck, v.hand.iter().copied());
    match &v.own_skat {
        Some(skat) => {
            ui.label(RichText::new(i.t("training-discard")).strong());
            crate::recording::card_row(ui, i, deck, skat.iter().copied());
        }
        None => {
            ui.label(i.t("training-skat-unknown"));
        }
    }
}

fn played(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    v: &TrainingView,
    cards: &[(Seat, skat_app::Card)],
) {
    ui.horizontal(|ui| {
        for &(seat, card) in cards {
            // Equal columns, so the cards line up across tricks.
            ui.allocate_ui_with_layout(
                egui::vec2(110.0, 100.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.set_width(110.0);
                    crate::recording::card_image(ui, i, deck, card);
                    ui.add(egui::Label::new(RichText::new(seat_label(i, v, seat)).small()).wrap());
                },
            );
        }
    });
}

fn topic_message(topic: TrainingTopic) -> &'static str {
    match topic {
        TrainingTopic::TrumpfOutside => "learning-topic-trumpf",
        TrainingTopic::ProvenVoids => "learning-topic-voids",
    }
}

fn start_message(topic: TrainingTopic) -> &'static str {
    match topic {
        TrainingTopic::TrumpfOutside => "training-start-trumpf",
        TrainingTopic::ProvenVoids => "training-start-voids",
    }
}

fn topic_note(topic: TrainingTopic) -> &'static str {
    match topic {
        TrainingTopic::TrumpfOutside => "training-note-trumpf",
        TrainingTopic::ProvenVoids => "training-note-voids",
    }
}
