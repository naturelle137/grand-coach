// SPDX-License-Identifier: EUPL-1.2

//! Read-only comparison of a finished practice attempt with the original
//! game (D-66). Own decisions only, from the permitted view then; it ranks
//! nothing and names no better line.

use eframe::egui::{self, RichText};
use skat_app::{ComparisonRow, DecisionContext, PracticeComparison};

use crate::{cards::Deck, coach, i18n::I18n, recording::situation};

/// Shows the window; returns `false` once it is closed.
pub(crate) fn window(
    ctx: &egui::Context,
    i: &I18n,
    deck: Deck,
    comparison: &PracticeComparison,
) -> bool {
    let mut open = true;
    egui::Window::new(i.t("compare-title"))
        .id(egui::Id::unique("practice_compare"))
        .open(&mut open)
        .default_pos(egui::pos2(60.0, 80.0))
        .default_width(900.0)
        .max_width((ctx.content_rect().width() - 80.0).max(320.0))
        .max_height((ctx.content_rect().height() - 100.0).max(300.0))
        .show(ctx, |ui| {
            ui.label(RichText::new(i.t("compare-isolation")).strong());
            ui.label(i.t("compare-no-ranking"));
            if comparison.diverged() == 0 {
                ui.label(i.t("compare-all-same"));
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (n, row) in comparison.rows.iter().enumerate() {
                    match row {
                        ComparisonRow::Identical { count } => {
                            ui.label(
                                RichText::new(
                                    i.tf("compare-identical", &[("count", (*count).into())]),
                                )
                                .weak(),
                            );
                        }
                        ComparisonRow::Diverged {
                            ordinal,
                            original,
                            attempt,
                        } => {
                            ui.separator();
                            ui.label(
                                RichText::new(
                                    i.tf("compare-decision", &[("number", (*ordinal).into())]),
                                )
                                .strong(),
                            );
                            ui.columns(2, |cols| {
                                side(
                                    &mut cols[0],
                                    i,
                                    deck,
                                    "compare-original",
                                    original.as_deref(),
                                    (n, 0),
                                );
                                side(
                                    &mut cols[1],
                                    i,
                                    deck,
                                    "compare-attempt",
                                    attempt.as_deref(),
                                    (n, 1),
                                );
                            });
                        }
                    }
                }
            });
        });
    open
}

fn side(
    ui: &mut egui::Ui,
    i: &I18n,
    deck: Deck,
    title: &str,
    context: Option<&DecisionContext>,
    salt: (usize, u8),
) {
    ui.label(RichText::new(i.t(title)).heading());
    match context {
        Some(c) => {
            ui.label(
                RichText::new(i.tf(
                    "compare-chose",
                    &[("action", coach::context_action(i, c).into())],
                ))
                .strong(),
            );
            situation(ui, i, deck, &coach::replay_situation(i, c), salt);
        }
        None => {
            ui.label(i.t("compare-none"));
        }
    }
}
