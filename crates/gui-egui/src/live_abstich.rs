// SPDX-License-Identifier: EUPL-1.2

//! Presentation only: the app owns candidate identity and shared evidence.
use crate::{i18n::I18n, text};
use eframe::egui;
use skat_app::{
    Command, FocusAnswer, LiveAbstichAction, LiveAbstichView, MechanismResponse, PlayerId,
    TableView,
};

pub(crate) fn response_key(r: MechanismResponse) -> &'static str {
    match r {
        MechanismResponse::Correct => "live-response-correct",
        MechanismResponse::OverstatedCertainty => "live-response-overstated",
        MechanismResponse::DeniedPossibility => "live-response-denied",
        MechanismResponse::UnderstatedCertainty => "live-response-understated",
        MechanismResponse::Skipped => "live-response-skipped",
        _ => "live-response-unsupported",
    }
}

pub(crate) fn panel(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    v: &LiveAbstichView,
) -> Option<Command> {
    let mut command = None;
    ui.heading(i.t("live-abstich-title"));
    ui.label(i.t("live-abstich-note"));
    if v.unsupported {
        ui.label(i.t("live-abstich-unsupported"));
    }
    if !v.unsupported || v.id.is_some() {
        ui.label(i.t("live-abstich-select"));
        ui.horizontal_wrapped(|ui| {
            for &card in &v.candidates {
                if ui
                    .selectable_label(v.id.is_some_and(|id| id.card == card), text::card(i, card))
                    .clicked()
                {
                    command = Some(Command::SelectLiveAbstich(v.boundary, card));
                }
            }
        });
    }
    if let Some(id) = v.id
        && let Some(seat) = id.opponent
    {
        let player = PlayerId::on_seat(seat, table.dealer);
        ui.label(i.tf(
            "live-abstich-question",
            &[
                ("card", text::card(i, id.card).into()),
                ("player", text::player(i, player).into()),
            ],
        ));
        ui.label(i.t("live-abstich-definition"));
    }
    if let Some(r) = v.response {
        ui.label(i.t(response_key(r)));
    }
    if let Some(e) = v.evidence {
        ui.label(i.t(match e.certainty {
            skat_app::coach_model::Certainty::Possible => "focus-possible",
            skat_app::coach_model::Certainty::Certain => "focus-certain",
        }));
        ui.label(i.tf(
            "focus-abstich-evidence",
            &[
                ("seat", text::seat(i, e.seat).into()),
                ("suit", text::suit(i, e.card.suit()).into()),
                ("cards", e.suit_unseen.len().into()),
                ("trumps", e.trumpf_unseen.len().into()),
            ],
        ));
        ui.label(i.t("focus-boundary-note"));
    }
    if v.assisted {
        ui.label(i.t("live-abstich-assisted"));
    }
    if let Some(id) =
        v.id.filter(|id| id.opponent.is_some() && v.response.is_none())
    {
        for (key, answer) in [
            ("focus-possible", FocusAnswer::Possible),
            ("focus-certain", FocusAnswer::Certain),
            ("focus-absent", FocusAnswer::Absent),
        ] {
            if ui.button(i.t(key)).clicked() {
                command = Some(Command::LiveAbstich(id, LiveAbstichAction::Answer(answer)));
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button(i.t("live-abstich-help")).clicked() {
                command = Some(Command::LiveAbstich(id, LiveAbstichAction::Help));
            }
            if ui.button(i.t("quiz-skip")).clicked() {
                command = Some(Command::LiveAbstich(id, LiveAbstichAction::Skip));
            }
        });
    }
    command
}

pub(crate) fn summary(ui: &mut egui::Ui, i: &I18n, rows: &[(bool, skat_app::MechanismCounts); 2]) {
    ui.separator();
    ui.heading(i.t("live-abstich-history"));
    ui.label(i.t("live-abstich-selection-bias"));
    ui.label(i.t("live-abstich-source"));
    if rows.iter().all(|(_, c)| c.presented == 0) {
        ui.label(i.t("live-abstich-empty"));
    }
    for (assisted, c) in rows.iter().filter(|(_, c)| c.presented > 0) {
        ui.strong(i.t(if *assisted {
            "live-abstich-assisted"
        } else {
            "focus-unaided"
        }));
        ui.label(i.tf(
            "live-abstich-totals",
            &[
                ("presented", c.presented.into()),
                ("answered", c.answered.into()),
                ("scored", c.scored.into()),
            ],
        ));
        ui.label(i.tf(
            "live-abstich-categories",
            &[
                ("correct", c.correct.into()),
                ("overstated", c.overstated.into()),
                ("denied", c.denied.into()),
                ("understated", c.understated.into()),
            ],
        ));
        ui.label(i.tf(
            "live-abstich-remaining",
            &[
                ("skipped", c.skipped.into()),
                ("unanswered", c.unanswered.into()),
                ("unsupported", c.unsupported.into()),
            ],
        ));
    }
}
