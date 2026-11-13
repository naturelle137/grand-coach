// SPDX-License-Identifier: EUPL-1.2

//! One bilingual prompt at a time. Answers never act on the table.
use crate::{coach, i18n::I18n, text};
use eframe::egui;
use skat_app::coach_model::Source;
use skat_app::{GuidedAction, GuidedAnswer, GuidedBranch, GuidedStep, GuidedView, Seat, TableView};

pub(crate) fn panel(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
) -> Option<GuidedAction> {
    let mut action = None;
    ui.heading(i.t("guided-title"));
    let prompt = match g.step {
        GuidedStep::Orient => "guided-orient",
        GuidedStep::Check => "guided-check",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::UnderPartner) => "guided-under",
        GuidedStep::Narrow => "guided-win",
        GuidedStep::Act => "guided-act",
        GuidedStep::Waiting => "guided-waiting",
        GuidedStep::Observe => "guided-observe",
        GuidedStep::Complete => "guided-complete",
        GuidedStep::Unsupported => "guided-unsupported",
    };
    ui.label(i.t(prompt));
    if g.correct == Some(false) {
        ui.label(i.t("guided-wrong"));
        if ui.button(i.t("guided-retry")).clicked() {
            action = Some(GuidedAction::Retry);
        }
    }
    if matches!(g.step, GuidedStep::Orient | GuidedStep::Check) {
        for seat in Seat::ALL {
            if ui.button(text::seat(i, seat)).clicked() {
                action = Some(GuidedAction::Answer(GuidedAnswer::Seat(Some(seat))));
            }
        }
        if g.step == GuidedStep::Check && ui.button(i.t("guided-none")).clicked() {
            action = Some(GuidedAction::Answer(GuidedAnswer::Seat(None)));
        }
    }
    if g.step == GuidedStep::Narrow {
        for &card in &g.legal_cards {
            if ui
                .button(i.tf("guided-consider", &[("card", text::card(i, card).into())]))
                .clicked()
            {
                action = Some(GuidedAction::Answer(GuidedAnswer::Card(card)));
            }
        }
    }
    if let Some(source) = g
        .evidence
        .filter(|_| g.correct == Some(false) && g.step == GuidedStep::Orient)
    {
        ui.label(source_line(i, source));
    }
    if g.step == GuidedStep::Act
        && let Some(rule) = &g.rule
    {
        ui.strong(i.tf(
            "guided-rule-card",
            &[("card", text::card(i, rule.card).into())],
        ));
        let key = if rule.branch == GuidedBranch::UnderPartner {
            "guided-rule-under"
        } else {
            "guided-rule-win"
        };
        ui.label(i.t(key));
        let cards: Vec<_> = rule.candidates.iter().map(|&c| text::card(i, c)).collect();
        ui.label(i.tf("guided-candidates", &[("cards", cards.join(", ").into())]));
        let open = g.memory.is_some();
        if ui
            .button(i.t(if open {
                "guided-memory-hide"
            } else {
                "guided-memory"
            }))
            .clicked()
        {
            action = Some(GuidedAction::Memory(!open));
        }
    }
    if let Some(m) = &g.memory {
        ui.group(|ui| {
            ui.label(i.tf(
                "guided-memory-counts",
                &[
                    ("own", m.own.len().into()),
                    ("played", m.played.len().into()),
                    ("unseen", m.unseen.len().into()),
                ],
            ));
            ui.label(match m.skat {
                Some(s) => i.tf("guided-known-skat", &[("count", s.len().into())]),
                None => i.t("guided-unknown-skat"),
            });
            if let Some(t) = m.trumpf {
                ui.label(i.tf("guided-trumpf", &[("count", t.outside.into())]));
            }
        });
    }
    if matches!(g.step, GuidedStep::Observe | GuidedStep::Complete)
        && let Some(s) = g.observed_source
    {
        ui.label(source_line(i, s));
    }
    if g.step == GuidedStep::Observe {
        for (key, answer) in [("guided-void-yes", true), ("guided-void-no", false)] {
            if ui.button(i.t(key)).clicked() {
                action = Some(GuidedAction::Answer(GuidedAnswer::Void(answer)));
            }
        }
    }
    if let Some(o) = &g.observation {
        ui.label(i.t(if o.proves_void {
            "guided-proves-void"
        } else {
            "guided-only-card"
        }));
        if let Some(seat) = o.winner {
            ui.label(i.tf("guided-winner", &[("seat", text::seat(i, seat).into())]));
        }
    }
    if matches!(
        g.step,
        GuidedStep::Orient | GuidedStep::Check | GuidedStep::Narrow | GuidedStep::Observe
    ) {
        if ui.button(i.t("guided-unknown")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
        if ui.button(i.t("guided-reveal")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
    }
    if g.step == GuidedStep::Complete
        && table.coach.settings.show_figures
        && ui.button(i.t("guided-figures")).clicked()
    {
        action = Some(GuidedAction::Figures);
    }
    if g.figures_requested {
        ui.separator();
        ui.strong(i.t("guided-model-title"));
        egui::ScrollArea::vertical()
            .id_salt("guided-model")
            .max_height(220.0)
            .show(ui, |ui| {
                if let Some(h) = &g.figures {
                    ui.label(i.tf(
                        "guided-budget",
                        &[
                            ("samples", g.budget.map_or(0, |b| b.samples).into()),
                            ("nodes", g.budget.map_or(0, |b| b.nodes_per_task).into()),
                            ("game", g.id.game.into()),
                            ("actions", g.id.actions.into()),
                        ],
                    ));
                    ui.label(i.t(if g.basis == skat_app::CoachBasis::RealDeal {
                        "coach-basis-real-deal"
                    } else {
                        "guided-basis-own"
                    }));
                    for line in coach::hints(i, table, h, true) {
                        ui.label(line.text);
                    }
                } else {
                    ui.label(i.t("guided-model-unavailable"));
                }
            });
    }
    if ui.button(i.t("guided-return")).clicked() {
        action = Some(GuidedAction::Return);
    }
    action
}

fn source_line(i: &I18n, source: Source) -> String {
    match source {
        Source::Trick {
            index, seat, card, ..
        } => i.tf(
            "guided-source",
            &[
                ("trick", (index + 1).into()),
                ("seat", text::seat(i, seat).into()),
                ("card", text::card(i, card).into()),
            ],
        ),
        _ => String::new(),
    }
}
