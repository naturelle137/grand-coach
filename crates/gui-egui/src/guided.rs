// SPDX-License-Identifier: EUPL-1.2

//! One bilingual prompt at a time. Answers never act on the table.
use crate::{coach, i18n::I18n, text};
use eframe::egui;
use skat_app::coach_model::Source;
use skat_app::{
    Action, GuidedAction, GuidedAnswer, GuidedBranch, GuidedGameBranch, GuidedKeep, GuidedPreRule,
    GuidedStep, GuidedView, Phase, Seat, TableView,
};

pub(crate) fn panel(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
) -> Option<GuidedAction> {
    let mut action = None;
    ui.heading(i.t("guided-title"));
    if let Some(r) = g.reizen {
        return reizen(ui, i, table, g, r);
    }
    if let Some(p) = &g.preplay {
        return preplay(ui, i, table, g, p);
    }
    let prompt = match g.step {
        GuidedStep::Orient => "guided-orient",
        GuidedStep::Check => "guided-check",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::UnderPartner) => "guided-under",
        GuidedStep::Narrow | GuidedStep::Threshold => "guided-win",
        GuidedStep::Act => "guided-act",
        GuidedStep::Waiting => "guided-waiting",
        GuidedStep::Observe => "guided-observe",
        GuidedStep::Complete => "guided-complete",
        GuidedStep::Unsupported
            if table.phase == Phase::Ansage
                || table
                    .legal_actions
                    .iter()
                    .any(|a| matches!(a, Action::Kontra | Action::KeinKontra | Action::Re)) =>
        {
            "guided-unsupported-ansage"
        }
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

/// Reizen: public bid, role, threshold attempt, then Easy's actual call.
fn reizen(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
    r: skat_app::GuidedReizen,
) -> Option<GuidedAction> {
    let mut action = None;
    ui.label(if r.current == 0 {
        i.t("guided-bid-current-none")
    } else {
        i.tf("guided-bid-current", &[("bid", r.current.into())])
    });
    let prompt = match (g.step, r.holds, r.stake) {
        (GuidedStep::Orient, ..) => i.t("guided-bid-orient"),
        (GuidedStep::Threshold, Some(true), Some(v)) => {
            i.tf("guided-bid-threshold-hold", &[("value", v.into())])
        }
        (GuidedStep::Threshold, _, Some(v)) => {
            i.tf("guided-bid-threshold-ask", &[("value", v.into())])
        }
        (GuidedStep::Threshold, ..) => {
            i.tf("guided-bid-threshold-none", &[("bid", r.current.into())])
        }
        (GuidedStep::Act, ..) => i.t("guided-bid-act"),
        _ => i.t("guided-unsupported"),
    };
    ui.label(prompt);
    if g.correct == Some(false) {
        ui.label(i.t("guided-wrong"));
        if ui.button(i.t("guided-retry")).clicked() {
            action = Some(GuidedAction::Retry);
        }
    }
    let answers: &[(&str, GuidedAnswer)] = match g.step {
        GuidedStep::Orient => &[
            ("guided-bid-holds", GuidedAnswer::Holds(true)),
            ("guided-bid-asks", GuidedAnswer::Holds(false)),
        ],
        GuidedStep::Threshold => &[
            ("guided-bid-within-yes", GuidedAnswer::Within(true)),
            ("guided-bid-within-no", GuidedAnswer::Within(false)),
        ],
        _ => &[],
    };
    for &(key, answer) in answers {
        if ui.button(i.t(key)).clicked() {
            action = Some(GuidedAction::Answer(answer));
        }
    }
    if let Some(rule) = &g.bid_rule {
        ui.strong(i.tf(
            "guided-bid-rule",
            &[("action", coach::action(i, table, rule.action).into())],
        ));
        let branch = match rule.branch {
            GuidedGameBranch::Suit => Some("guided-bid-branch-suit"),
            GuidedGameBranch::Grand => Some("guided-bid-branch-grand"),
            GuidedGameBranch::Null => Some("guided-bid-branch-null"),
            GuidedGameBranch::Weak => None,
        };
        if let Some(key) = branch {
            ui.label(i.tf(
                "guided-bid-game",
                &[
                    ("game", text::game_type(i, rule.game).into()),
                    ("ceiling", rule.ceiling.into()),
                ],
            ));
            ui.label(i.t(key));
        } else {
            ui.label(i.t("guided-bid-weak"));
        }
        ui.label(match rule.stake {
            Some(v) => i.tf(
                if v <= rule.ceiling {
                    "guided-bid-within"
                } else {
                    "guided-bid-above"
                },
                &[("value", v.into()), ("ceiling", rule.ceiling.into())],
            ),
            None => i.t("guided-bid-no-value"),
        });
        ui.label(i.t("guided-bid-caveat"));
    }
    if matches!(g.step, GuidedStep::Orient | GuidedStep::Threshold) {
        if ui.button(i.t("guided-unknown")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
        if ui.button(i.t("guided-reveal")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
    }
    if ui.button(i.t("guided-return")).clicked() {
        action = Some(GuidedAction::Return);
    }
    action
}

/// Skataufnahme/Drücken: candidate branch, threshold or card attempt, then
/// Easy's actual Hand/pickup or discard.
fn preplay(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
    p: &skat_app::GuidedPreplay,
) -> Option<GuidedAction> {
    let mut action = None;
    let pickup = p.phase == Phase::SkatAufnahme;
    let game = p.game.map(|game| text::game_type(i, game));
    let prompt = match (g.step, &game) {
        (GuidedStep::Orient, _) if pickup => i.t("guided-pre-orient-pickup"),
        (GuidedStep::Orient, _) => i.t("guided-pre-orient-discard"),
        (GuidedStep::Threshold, Some(game)) if pickup => i.tf(
            "guided-pre-threshold-pickup",
            &[
                ("game", game.clone().into()),
                ("threshold", p.threshold.unwrap_or_default().into()),
            ],
        ),
        (GuidedStep::Threshold, Some(game)) => i.tf(
            "guided-pre-threshold-discard",
            &[("game", game.clone().into())],
        ),
        (GuidedStep::Act, _) if pickup => i.t("guided-pre-act-pickup"),
        (GuidedStep::Act, _) => i.t("guided-pre-act-discard"),
        _ => i.t("guided-unsupported"),
    };
    ui.label(prompt);
    if g.correct == Some(false) {
        ui.label(i.t("guided-wrong"));
        if ui.button(i.t("guided-retry")).clicked() {
            action = Some(GuidedAction::Retry);
        }
    }
    let answers: Vec<(String, GuidedAnswer)> = match g.step {
        GuidedStep::Orient => [
            ("guided-pre-grand", GuidedGameBranch::Grand),
            ("guided-pre-suit", GuidedGameBranch::Suit),
            ("guided-pre-null", GuidedGameBranch::Null),
            ("guided-pre-weak", GuidedGameBranch::Weak),
        ]
        .map(|(key, b)| (i.t(key), GuidedAnswer::Game(b)))
        .to_vec(),
        GuidedStep::Threshold if pickup => vec![
            (i.t("guided-pre-hand-yes"), GuidedAnswer::Within(true)),
            (i.t("guided-pre-hand-no"), GuidedAnswer::Within(false)),
        ],
        GuidedStep::Threshold => p
            .cards
            .iter()
            .map(|&c| {
                (
                    i.tf("guided-pre-consider", &[("card", text::card(i, c).into())]),
                    GuidedAnswer::Card(c),
                )
            })
            .collect(),
        _ => vec![],
    };
    for (label, answer) in answers {
        if ui.button(label).clicked() {
            action = Some(GuidedAction::Answer(answer));
        }
    }
    if let Some(rule) = &g.pre_rule {
        ui.strong(i.tf(
            "guided-bid-rule",
            &[("action", coach::action(i, table, rule.action()).into())],
        ));
        if rule.fallback() {
            ui.label(i.t("guided-pre-fallback"));
        }
        ui.label(i.tf(
            "guided-pre-game",
            &[("game", text::game_type(i, rule.game()).into())],
        ));
        ui.label(i.t(match rule.branch() {
            GuidedGameBranch::Grand => "guided-pre-branch-grand",
            GuidedGameBranch::Suit => "guided-pre-branch-suit",
            GuidedGameBranch::Null => "guided-pre-branch-null",
            GuidedGameBranch::Weak => "guided-pre-branch-weak",
        }));
        match rule {
            GuidedPreRule::Pickup(r) if r.branch == GuidedGameBranch::Null => {
                ui.label(i.t("guided-pre-strength-null"));
            }
            GuidedPreRule::Pickup(r) => {
                ui.label(i.tf(
                    if r.strength >= r.threshold {
                        "guided-pre-strength-hand"
                    } else {
                        "guided-pre-strength-pickup"
                    },
                    &[
                        ("strength", r.strength.into()),
                        ("threshold", r.threshold.into()),
                    ],
                ));
                ui.label(i.t("guided-pre-strength-count"));
            }
            GuidedPreRule::Discard(r) => {
                ui.label(i.t(if r.branch == GuidedGameBranch::Null {
                    "guided-pre-keeps-null"
                } else {
                    "guided-pre-keeps"
                }));
                let mut put = skat_app::CardSet::EMPTY;
                for (card, keep) in r.reasons {
                    put.insert(card);
                    let name = ("card", text::card(i, card).into());
                    ui.label(match keep {
                        GuidedKeep::Side { suit } => {
                            i.tf("guided-pre-keep-side", &[name, ("length", suit.into())])
                        }
                        GuidedKeep::Null => i.tf("guided-pre-keep-null", &[name]),
                        GuidedKeep::Trumpf => i.tf("guided-pre-keep-trumpf", &[name]),
                        GuidedKeep::Ass => i.tf("guided-pre-keep-ass", &[name]),
                        GuidedKeep::GuardedTen => i.tf("guided-pre-keep-ten", &[name]),
                    });
                }
                let ties: Vec<_> = (r.eligible - put)
                    .iter()
                    .map(|c| text::card(i, c))
                    .collect();
                if !ties.is_empty() {
                    ui.label(i.tf("guided-pre-tie", &[("cards", ties.join(", ").into())]));
                }
            }
        }
        ui.label(i.t(if pickup {
            "guided-pre-caveat-pickup"
        } else {
            "guided-pre-caveat-discard"
        }));
    }
    if matches!(g.step, GuidedStep::Orient | GuidedStep::Threshold) {
        if ui.button(i.t("guided-unknown")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
        if ui.button(i.t("guided-reveal")).clicked() {
            action = Some(GuidedAction::Reveal);
        }
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
