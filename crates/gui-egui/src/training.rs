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
    focus_error: bool,
    validation_chooser: bool,
    validation_demo: bool,
    prior_exposure: skat_app::PriorExposure,
    own_words: String,
    clarity: String,
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

    /// Prepared launcher opens confirmation only, never a reserved view.
    pub(crate) fn prepare_validation(&mut self) {
        self.open = true;
        if self.session.is_none() {
            self.validation_chooser = true;
        }
    }

    /// Leaves the training; its state is dropped.
    pub(crate) fn exit(&mut self) {
        *self = TrainingUi {
            assistance: self.assistance,
            ..TrainingUi::default()
        };
    }

    pub(crate) fn view(&self) -> Option<TrainingView> {
        self.session
            .as_ref()
            .filter(|s| s.focus_view().is_none())
            .map(TrainingSession::view)
    }

    /// Gallery-only shared form on an ordinary public task, never a reserved view.
    pub(crate) fn preview_validation_controls(&mut self) {
        if self
            .focus_view()
            .is_some_and(|v| v.entry.validation.is_none())
        {
            self.validation_demo = true;
        }
    }

    pub(crate) fn focus_view(&self) -> Option<skat_app::FocusView> {
        self.session.as_ref().and_then(TrainingSession::focus_view)
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
        let focus = self.focus_view();
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
                        if let Some(v) = &focus
                            && v.entry.validation.is_some()
                            && !v.settled
                            && !self.focus_error
                        {
                            let command =
                                self.validation_submission(skat_app::ValidationResponse::Stop);
                            if let Some(session) = &mut self.session {
                                self.focus_error = !learning.run_focus(session, command);
                            }
                        }
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(v) = &focus {
                    if self.focus_error && v.entry.validation.is_some() {
                        ui.label(i.t("validation-save-stop"));
                        return;
                    }
                    if let Some(command) = self.focus_task(ui, i, deck, v)
                        && let Some(session) = &mut self.session
                    {
                        let old = session.focus_view().map(|v| v.entry.attempt);
                        self.focus_error = !learning.run_focus(session, command);
                        if old != session.focus_view().map(|v| v.entry.attempt) {
                            self.prior_exposure = skat_app::PriorExposure::Unknown;
                            self.own_words.clear();
                            self.clarity.clear();
                        }
                        self.value = 0;
                    }
                    if v.entry.validation.is_none() && ui.button(i.t("focus-topics")).clicked() {
                        self.session = None;
                    }
                    if self.focus_error {
                        ui.label(i.t("focus-unavailable"));
                    }
                    return;
                }
                match &view {
                    None => {
                        if self.validation_chooser {
                            ui.heading(i.t("validation-title"));
                            ui.label(i.t("validation-intro"));
                            ui.label(i.tf(
                                "validation-build",
                                &[("build", env!("GRAND_COACH_BUILD_COMMIT").into())],
                            ));
                            for (set, key) in [
                                (
                                    skat_app::practice_catalogue::ValidationSet::Initial,
                                    "validation-initial",
                                ),
                                (
                                    skat_app::practice_catalogue::ValidationSet::Delayed,
                                    "validation-delayed",
                                ),
                            ] {
                                if ui.button(i.t(key)).clicked() {
                                    self.session = learning.start_validation(set);
                                    self.focus_error = self.session.is_none();
                                    self.prior_exposure = skat_app::PriorExposure::Unknown;
                                    self.own_words.clear();
                                    self.clarity.clear();
                                }
                            }
                            if self.focus_error {
                                ui.label(i.t("validation-save-stop"));
                            }
                            if ui.button(i.t("focus-topics")).clicked() {
                                self.validation_chooser = false;
                            }
                            return;
                        }
                        for family in FAMILIES {
                            if ui.button(i.t(family_message(family))).clicked() {
                                self.session = learning.start_focus(family, self.assistance);
                                self.focus_error = self.session.is_none();
                            }
                        }
                        if self.focus_error {
                            ui.label(i.t("focus-unavailable"));
                        }
                        ui.label(i.t("focus-limits"));
                        if ui.button(i.t("validation-entry")).clicked() {
                            self.validation_chooser = true;
                        }
                        ui.separator();
                        if let Some(topic) = self.chooser(ui, i) {
                            self.start(topic, learning);
                        }
                    }
                    Some(v) => {
                        command = self.task(ui, i, deck, v);
                    }
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

use skat_app::practice_catalogue::{
    Mechanism, MechanismKind, PracticeFamily, PracticeFeedback, PracticeQuestion,
};
use skat_app::{FocusAction, FocusAnswer, FocusCommand, FocusView};

const FAMILIES: [PracticeFamily; 4] = [
    PracticeFamily::RemainingTrumpf,
    PracticeFamily::ProvenVoid,
    PracticeFamily::SideCardVulnerability,
    PracticeFamily::PartnerOvertaking,
];

fn family_message(family: PracticeFamily) -> &'static str {
    match family {
        PracticeFamily::RemainingTrumpf => "focus-trumpf",
        PracticeFamily::ProvenVoid => "focus-void",
        PracticeFamily::SideCardVulnerability => "focus-side",
        PracticeFamily::PartnerOvertaking => "focus-partner",
    }
}

fn answer_message(answer: FocusAnswer) -> &'static str {
    match answer {
        FocusAnswer::Count(_) => "focus-count",
        FocusAnswer::Possible => "focus-possible",
        FocusAnswer::Certain => "focus-certain",
        FocusAnswer::Absent => "focus-absent",
    }
}

impl TrainingUi {
    fn focus_task(
        &mut self,
        ui: &mut egui::Ui,
        i: &I18n,
        deck: Deck,
        v: &FocusView,
    ) -> Option<FocusCommand> {
        let mut command = None;
        ui.heading(i.t(family_message(v.family)));
        if self.validation_demo {
            ui.label(i.t("validation-demo"));
        }
        ui.label(i.tf(
            if v.entry.validation.is_some() {
                "validation-position"
            } else {
                "focus-position"
            },
            &[("number", v.number.into())],
        ));
        if let Some(data) = &v.entry.validation {
            ui.heading(i.t("validation-title"));
            ui.label(format!("{}@{}", v.entry.task.id, v.entry.task.version));
            ui.label(i.tf("validation-build", &[("build", data.build.clone().into())]));
            ui.label(i.t("validation-exposure-note"));
        }
        ui.label(i.t(
            if v.entry.validation.is_some() && v.entry.eligible && !v.entry.assisted {
                "validation-first"
            } else if v.entry.validation.is_none() && v.entry.eligible && !v.entry.assisted {
                "focus-unseen"
            } else {
                "focus-practice"
            },
        ));
        if v.entry.validation.is_some() && matches!(v.entry.action, FocusAction::Stopped) {
            ui.label(i.t("validation-stopped"));
            return None;
        }
        let position = &v.position;
        if let Some(contract) = position.contract {
            ui.label(i.tf(
                "training-contract",
                &[
                    ("game", text::contract(i, contract).into()),
                    ("bid", position.bid.into()),
                    (
                        "declarer",
                        text::seat(i, position.declarer.unwrap_or(position.seat)).into(),
                    ),
                ],
            ));
        }
        ui.label(i.tf(
            if position.declarer == Some(position.seat) {
                "training-role-declarer"
            } else {
                "training-role-defender"
            },
            &[
                ("seat", text::seat(i, position.seat).into()),
                (
                    "declarer",
                    text::seat(i, position.declarer.unwrap_or(position.seat)).into(),
                ),
            ],
        ));
        for (n, trick) in position.tricks.iter().enumerate() {
            ui.label(i.tf(
                "training-trick",
                &[
                    ("number", (n + 1).into()),
                    ("leader", text::seat(i, trick.seat_of(0)).into()),
                    ("winner", text::seat(i, trick.winner).into()),
                ],
            ));
            crate::recording::card_row(ui, i, deck, trick.cards);
        }
        ui.label(RichText::new(i.t("training-table")).strong());
        if position.current_trick.cards.iter().all(Option::is_none) {
            ui.label(i.t("training-you-lead"));
        } else {
            if let Some(leader) = position.current_trick.leader {
                ui.label(i.tf("focus-leader", &[("seat", text::seat(i, leader).into())]));
            }
            crate::recording::card_row(
                ui,
                i,
                deck,
                position.current_trick.cards.iter().flatten().copied(),
            );
        }
        ui.label(RichText::new(i.t("training-hand")).strong());
        crate::recording::card_row(ui, i, deck, position.hand.iter());
        ui.label(i.t("training-skat-unknown"));
        ui.separator();
        ui.label(RichText::new(i.t("training-question")).strong());
        match v.entry.question {
            PracticeQuestion::Count(q) => {
                ui.label(assist::question(i, q));
            }
            PracticeQuestion::Consequence {
                action: skat_app::Action::Play(card),
                kind,
            } => {
                ui.label(i.tf(
                    match kind {
                        MechanismKind::Abstich => "focus-question-abstich",
                        MechanismKind::Overtake => "focus-question-overtake",
                    },
                    &[("card", text::card(i, card).into())],
                ));
                ui.label(i.t("focus-judgement-note"));
            }
            _ => {
                ui.label(i.t("focus-unavailable"));
            }
        }
        if let Some(feedback) = &v.feedback {
            ui.label(
                RichText::new(i.t(if v.settled {
                    "training-result"
                } else {
                    "retrieval-help"
                }))
                .strong(),
            );
            if v.settled {
                ui.label(i.t(match &v.entry.action {
                    FocusAction::Answer { expected: None, .. } => "focus-unscored",
                    FocusAction::Answer {
                        given,
                        expected: Some(expected),
                        ..
                    } if given == expected => "focus-correct",
                    FocusAction::Answer { .. } => "focus-incorrect",
                    FocusAction::Unclear => "validation-unclear-result",
                    _ => "focus-skipped",
                }));
            }
            if let Some(expected) = skat_app::focus_expected(feedback) {
                match expected {
                    FocusAnswer::Count(_) => {}
                    _ => {
                        ui.label(i.t(answer_message(expected)));
                    }
                }
            } else {
                ui.label(i.t("focus-unavailable"));
            }
            match feedback {
                PracticeFeedback::Count(Some(d)) => {
                    for line in assist::derivation_lines(i, *d) {
                        ui.label(line);
                    }
                    if let Derivation::ProvenVoids { seats, .. } = d {
                        let names: Vec<_> = Seat::ALL
                            .into_iter()
                            .filter(|seat| seats[seat.index()])
                            .map(|seat| text::seat(i, seat))
                            .collect();
                        if !names.is_empty() {
                            ui.label(
                                i.tf("training-void-seats", &[("seats", names.join(", ").into())]),
                            );
                        }
                    }
                }
                PracticeFeedback::Count(None) => {}
                PracticeFeedback::Consequences(ms) => {
                    for m in ms {
                        match m {
                            Mechanism::Abstich(a) => {
                                ui.label(i.tf(
                                    "focus-abstich-evidence",
                                    &[
                                        ("seat", text::seat(i, a.seat).into()),
                                        ("suit", text::suit(i, a.card.suit()).into()),
                                        ("cards", a.suit_unseen.len().into()),
                                        ("trumps", a.trumpf_unseen.len().into()),
                                    ],
                                ));
                            }
                            Mechanism::Overtake(o) => {
                                ui.label(
                                    i.tf(
                                        "focus-overtake-evidence",
                                        &[
                                            ("seat", text::seat(i, o.partner).into()),
                                            ("cards", o.higher.len().into()),
                                            (
                                                "trumping",
                                                i.t(if o.trumping {
                                                    "focus-trumping-possible"
                                                } else {
                                                    "focus-no-trumping"
                                                })
                                                .into(),
                                            ),
                                        ],
                                    ),
                                );
                            }
                        }
                    }
                    ui.label(i.t("focus-boundary-note"));
                }
            }
        }
        if v.feedback.is_none() && matches!(v.entry.action, FocusAction::Unclear) {
            ui.label(i.t("validation-unclear-result"));
        }
        if v.entry.assisted {
            ui.label(i.t("training-was-assisted"));
        }
        if (v.entry.validation.is_some() || self.validation_demo) && !v.settled {
            ui.separator();
            if !self.validation_demo && ui.button(i.t("validation-unexpected-aid")).clicked() {
                command =
                    Some(self.validation_submission(skat_app::ValidationResponse::UnexpectedAid));
            }
            ui.label(i.t("validation-prior"));
            ui.horizontal_wrapped(|ui| {
                for (value, key) in [
                    (skat_app::PriorExposure::None, "validation-none"),
                    (skat_app::PriorExposure::Seen, "validation-seen"),
                    (skat_app::PriorExposure::Unknown, "validation-unknown"),
                ] {
                    ui.radio_value(&mut self.prior_exposure, value, i.t(key));
                }
            });
            let label = ui.label(i.t("validation-own-words"));
            ui.add(
                egui::TextEdit::multiline(&mut self.own_words)
                    .char_limit(1024)
                    .desired_rows(2)
                    .hint_text(i.t("validation-own-words")),
            )
            .labelled_by(label.id);
            let label = ui.label(i.t("validation-clarity"));
            ui.add(
                egui::TextEdit::multiline(&mut self.clarity)
                    .char_limit(1024)
                    .desired_rows(2)
                    .hint_text(i.t("validation-clarity")),
            )
            .labelled_by(label.id);
            if ui
                .add_enabled(
                    !self.validation_demo,
                    egui::Button::new(i.t("validation-unclear")),
                )
                .clicked()
            {
                command = Some(self.validation_submission(skat_app::ValidationResponse::Unclear));
            }
        }
        if v.settled {
            ui.horizontal_wrapped(|ui| {
                if ui.button(i.t("training-retry")).clicked() {
                    command = Some(FocusCommand::Retry);
                }
                if (v.entry.validation.is_none() || v.number < 4)
                    && ui.button(i.t("training-next")).clicked()
                {
                    command = Some(FocusCommand::Next);
                }
            });
            if v.entry.validation.is_some() && v.number == 4 {
                ui.label(i.t("validation-complete"));
            }
        } else {
            let mut level = v.entry.assistance;
            ui.horizontal_wrapped(|ui| {
                for (l, key) in FOCUS_LEVELS {
                    ui.radio_value(&mut level, l, i.t(key));
                }
            });
            if level != v.entry.assistance {
                command = Some(FocusCommand::Assistance(level));
            }
            ui.horizontal_wrapped(|ui| {
                match v.entry.question {
                    PracticeQuestion::Count(_) => {
                        ui.label(i.tf("quiz-value", &[("value", self.value.into())]));
                        for (key, delta) in [
                            ("quiz-minus-ten", -10),
                            ("quiz-minus-one", -1),
                            ("quiz-plus-one", 1),
                            ("quiz-plus-ten", 10),
                        ] {
                            if ui.button(i.t(key)).clicked() {
                                self.value = (i16::from(self.value) + delta).clamp(0, 32) as u8;
                            }
                        }
                        if ui.button(i.t("quiz-submit")).clicked() {
                            command = Some(FocusCommand::Answer(FocusAnswer::Count(self.value)));
                        }
                    }
                    _ => {
                        for answer in [
                            FocusAnswer::Possible,
                            FocusAnswer::Certain,
                            FocusAnswer::Absent,
                        ] {
                            if ui.button(i.t(answer_message(answer))).clicked() {
                                command = Some(FocusCommand::Answer(answer));
                            }
                        }
                    }
                }
                if ui.button(i.t("retrieval-help")).clicked() {
                    command = Some(FocusCommand::Help);
                }
                if ui.button(i.t("quiz-skip")).clicked() {
                    command = Some(FocusCommand::Skip);
                }
            });
        }
        if v.entry.validation.is_some() {
            command = command.map(|c| match c {
                FocusCommand::Answer(answer) => {
                    self.validation_submission(skat_app::ValidationResponse::Answer(answer))
                }
                FocusCommand::Skip => {
                    self.validation_submission(skat_app::ValidationResponse::Skip)
                }
                other => other,
            });
        }
        command
    }

    fn validation_submission(&self, response: skat_app::ValidationResponse) -> FocusCommand {
        FocusCommand::SubmitValidation {
            response,
            exposure: self.prior_exposure,
            own_words: self.own_words.clone(),
            clarity: self.clarity.clone(),
            submitted_at: unix_time(),
        }
    }
}

pub(crate) fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

const FOCUS_LEVELS: [(AssistanceLevel, &str); 3] = [
    (AssistanceLevel::VisibleAid, "focus-visible"),
    (AssistanceLevel::RevealOnRequest, "focus-request"),
    (AssistanceLevel::AnswerBeforeReveal, "retrieval-answer"),
];
