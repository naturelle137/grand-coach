// SPDX-License-Identifier: EUPL-1.2

//! One bilingual prompt at a time. Answers never act on the table.
use crate::{coach, i18n::I18n, text};
use eframe::egui;
use skat_app::coach_model::Source;
use skat_app::{
    GameType, GuidedAction, GuidedAnnounceRule, GuidedAnswer, GuidedBranch, GuidedEarlyCondition,
    GuidedGameBranch, GuidedKeep, GuidedKontra, GuidedKontraCall, GuidedKontraExclusion,
    GuidedKontraObservation, GuidedPreRule, GuidedStep, GuidedView, Phase, Seat, TableView,
};

/// Render one active guided prompt from the application's permitted models.
/// Returns a user response; the caller applies it to the owning session.
pub fn panel(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
) -> Option<GuidedAction> {
    // Keep free return reachable while the Null or lead explanation scrolls,
    // including at the minimum supported window height. Earlier families keep
    // their existing layout and controls.
    if (g.feed_partner.is_some()
        || g.lead.is_some()
        || g.task == Some(GuidedBranch::WinEarly)
        || g.early_condition.is_some()
        || table.contract.is_some_and(|c| c.game == GameType::Null))
        && g.kontra.is_none()
        && g.reizen.is_none()
        && g.preplay.is_none()
    {
        let leave = ui.button(i.t("guided-return")).clicked();
        let scroll = egui::ScrollArea::vertical().id_salt("null_dialogue");
        // A new lead question starts at the top. Retaining the candidate
        // control's scroll offset could hide the next prompt at 960x640.
        let scroll = if g.feed_partner.is_some()
            || g.lead.is_some()
            || g.task == Some(GuidedBranch::WinEarly)
        {
            scroll.id_salt(("lead_dialogue", g.step as u8))
        } else {
            scroll
        };
        let response = scroll.show(ui, |ui| content(ui, i, table, g, false)).inner;
        if leave {
            Some(GuidedAction::Return)
        } else {
            response
        }
    } else {
        content(ui, i, table, g, true)
    }
}

fn content(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
    return_button: bool,
) -> Option<GuidedAction> {
    let mut action = None;
    ui.heading(i.t("guided-title"));
    if let Some(r) = g.reizen {
        return reizen(ui, i, table, g, r);
    }
    if let Some(p) = &g.preplay {
        return preplay(ui, i, table, g, p);
    }
    if let Some(k) = g.kontra {
        return kontra(ui, i, table, g, k);
    }
    if matches!(
        g.step,
        GuidedStep::Orient | GuidedStep::Check | GuidedStep::Narrow | GuidedStep::Act
    ) && table.contract.is_some_and(|c| {
        !c.ouvert
            && (c.game == GameType::Grand
                || (matches!(c.game, GameType::Farbe(_)) && g.feed_partner.is_some()))
    }) && table.current_trick.len() == 2
        && table.declarer != Some(skat_app::PlayerId::HUMAN)
        && let Some(partner) = table
            .players
            .iter()
            .find(|p| p.player != skat_app::PlayerId::HUMAN && Some(p.player) != table.declarer)
    {
        ui.label(i.tf(
            "guided-feed-role",
            &[("seat", text::seat(i, partner.seat).into())],
        ));
    }
    let null = table
        .contract
        .is_some_and(|c| c.game == GameType::Null && (!c.ouvert || g.lead.is_some()));
    let null_defender = null && table.declarer != Some(skat_app::PlayerId::HUMAN);
    if null_defender
        && matches!(
            g.step,
            GuidedStep::Orient | GuidedStep::Check | GuidedStep::Narrow | GuidedStep::Act
        )
    {
        ui.label(i.t(if g.lead.is_some() {
            "guided-null-defender-lead-role"
        } else if table.current_trick.len() == 2 {
            "guided-null-defender-last-role"
        } else {
            "guided-null-defender-role"
        }));
    }
    if null
        && matches!(
            g.step,
            GuidedStep::Orient | GuidedStep::Check | GuidedStep::Narrow | GuidedStep::Act
        )
        && (table.current_trick.len() == 2
            || (table.current_trick.len() == 1 && g.step == GuidedStep::Orient))
        && table.declarer == Some(skat_app::PlayerId::HUMAN)
    {
        ui.label(i.t(if table.current_trick.len() == 1 {
            "guided-null-middle-role"
        } else {
            "guided-null-role"
        }));
    }
    let prompt = match g.step {
        GuidedStep::Orient if g.lead.is_some() => "guided-lead-orient",
        GuidedStep::Check if g.lead.is_some() && null => "guided-null-lead-candidates",
        GuidedStep::Check if g.lead_trumpf => "guided-draw-trumpf",
        GuidedStep::Check if g.lead.is_some() => "guided-lead-master",
        GuidedStep::Narrow if g.lead.is_some() && null => "guided-null-lead-attempt",
        GuidedStep::Narrow if g.lead_trumpf => "guided-draw-trumpf-attempt",
        GuidedStep::Narrow if g.lead.is_some() => "guided-lead-attempt",
        GuidedStep::Observe if g.lead_trumpf => "guided-draw-trumpf-observe",
        GuidedStep::Observe if g.lead.is_some() => "guided-lead-observe",
        GuidedStep::Orient => "guided-orient",
        GuidedStep::Check => "guided-check",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::UnderPartner) => "guided-under",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::FeedPartner) => "guided-feed-attempt",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::WinEarly) => "guided-early-attempt",
        GuidedStep::Narrow if g.task == Some(GuidedBranch::Null) => "guided-null-attempt",
        GuidedStep::Narrow | GuidedStep::Threshold => "guided-win",
        GuidedStep::Act => "guided-act",
        GuidedStep::Waiting => "guided-waiting",
        GuidedStep::Observe => "guided-observe",
        GuidedStep::Complete => "guided-complete",
        GuidedStep::Unsupported => match g.kontra_exclusion {
            Some(GuidedKontraExclusion::OuvertRound) => "guided-unsupported-kontra-ouvert-round",
            Some(GuidedKontraExclusion::Ouvert) => "guided-unsupported-kontra-ouvert",
            Some(GuidedKontraExclusion::Restricted) => "guided-unsupported-kontra",
            None => "guided-unsupported",
        },
    };
    ui.label(i.t(prompt));
    if g.correct == Some(false) {
        ui.label(i.t("guided-wrong"));
        if ui.button(i.t("guided-retry")).clicked() {
            action = Some(GuidedAction::Retry);
        }
    }
    if let Some(seat) = g.lead
        && g.step == GuidedStep::Orient
    {
        for (key, declarer) in [
            ("guided-lead-declarer", true),
            ("guided-lead-defender", false),
        ] {
            if ui.button(i.t(key)).clicked() {
                action = Some(GuidedAction::Answer(GuidedAnswer::Lead { seat, declarer }));
            }
        }
    }
    if g.lead.is_some() && matches!(g.step, GuidedStep::Check | GuidedStep::Narrow) {
        ui.label(i.t(if null {
            "guided-null-lead-ranks"
        } else if g.lead_trumpf {
            "guided-draw-trumpf-definition"
        } else if matches!(table.contract.map(|c| c.game), Some(GameType::Farbe(_))) {
            "guided-lead-suit-definition"
        } else {
            "guided-lead-definition"
        }));
        ui.label(i.t(if null {
            if null_defender {
                "guided-null-defender-lead-limit"
            } else {
                "guided-null-lead-limit"
            }
        } else if g.lead_trumpf {
            "guided-draw-trumpf-risk"
        } else {
            "guided-lead-risk"
        }));
    }
    if g.lead.is_none() && matches!(g.step, GuidedStep::Orient | GuidedStep::Check) {
        for seat in Seat::ALL {
            if ui.button(text::seat(i, seat)).clicked() {
                action = Some(GuidedAction::Answer(GuidedAnswer::Seat(Some(seat))));
            }
        }
        if g.step == GuidedStep::Check && ui.button(i.t("guided-none")).clicked() {
            action = Some(GuidedAction::Answer(GuidedAnswer::Seat(None)));
        }
    }
    if g.step == GuidedStep::Narrow || (g.lead.is_some() && g.step == GuidedStep::Check) {
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
        let lead_rule = matches!(
            rule.branch,
            GuidedBranch::LeadMaster | GuidedBranch::DrawTrumpf
        );
        if !lead_rule {
            ui.strong(i.tf(
                "guided-rule-card",
                &[("card", text::card(i, rule.card).into())],
            ));
        }
        let key = if rule.branch == GuidedBranch::DrawTrumpf {
            "guided-rule-draw-trumpf"
        } else if rule.branch == GuidedBranch::LeadMaster {
            "guided-rule-lead"
        } else if rule.branch == GuidedBranch::UnderPartner {
            "guided-rule-under"
        } else if rule.branch == GuidedBranch::FeedPartner && !rule.last {
            "guided-rule-feed-early"
        } else if rule.branch == GuidedBranch::FeedPartner {
            "guided-rule-feed"
        } else if rule.branch == GuidedBranch::WinEarly {
            "guided-rule-early"
        } else if rule.branch == GuidedBranch::Null {
            if g.lead.is_some() {
                "guided-rule-null-lead"
            } else if null_defender && rule.last {
                "guided-rule-null-defender-last"
            } else if null_defender {
                "guided-rule-null-defender"
            } else if rule.last {
                "guided-rule-null"
            } else {
                "guided-rule-null-middle"
            }
        } else {
            "guided-rule-win"
        };
        if lead_rule {
            ui.strong(i.tf(key, &[("card", text::card(i, rule.card).into())]));
            let cards = rule
                .candidates
                .iter()
                .map(|&c| text::card(i, c))
                .collect::<Vec<_>>()
                .join(", ");
            ui.label(i.tf(
                if rule.branch == GuidedBranch::DrawTrumpf {
                    "guided-draw-trumpf-candidates-risk"
                } else {
                    "guided-lead-candidates-risk"
                },
                &[("cards", cards.into())],
            ));
        } else {
            ui.label(i.t(key));
        }
        if let Some(condition) = g.early_condition {
            ui.label(i.t(match condition {
                GuidedEarlyCondition::Trumpf
                    if matches!(table.contract.map(|c| c.game), Some(GameType::Farbe(_))) =>
                {
                    "guided-suit-early-trumpf"
                }
                GuidedEarlyCondition::FollowClassMaster
                    if matches!(table.contract.map(|c| c.game), Some(GameType::Farbe(_))) =>
                {
                    "guided-suit-early-master"
                }
                GuidedEarlyCondition::Trumpf => "guided-early-trumpf",
                GuidedEarlyCondition::FollowClassMaster => "guided-early-master",
            }));
        }
        let details = |ui: &mut egui::Ui| {
            let cards: Vec<_> = rule.candidates.iter().map(|&c| text::card(i, c)).collect();
            ui.label(i.tf("guided-candidates", &[("cards", cards.join(", ").into())]));
            if rule.branch == GuidedBranch::Null && g.lead.is_none() && !null_defender {
                let winners: Vec<_> = rule.winners.iter().map(|&c| text::card(i, c)).collect();
                ui.label(if winners.is_empty() {
                    i.t(if rule.last {
                        "guided-null-no-winners"
                    } else {
                        "guided-null-middle-no-winners"
                    })
                } else {
                    i.tf(
                        if rule.last {
                            "guided-null-winners"
                        } else {
                            "guided-null-middle-winners"
                        },
                        &[("cards", winners.join(", ").into())],
                    )
                });
            }
        };
        if rule.branch == GuidedBranch::DrawTrumpf {
            ui.collapsing(i.t("guided-lead-details"), |ui| {
                ui.label(i.t("guided-draw-trumpf-conservative"));
                ui.label(i.t("guided-lead-empty"));
            });
        } else if rule.branch == GuidedBranch::LeadMaster {
            // Choice, candidates and caveat above; further detail stays collapsed.
            ui.collapsing(i.t("guided-lead-details"), |ui| {
                ui.label(i.t("guided-lead-conservative"));
                ui.label(i.t("guided-lead-empty"));
            });
        } else if rule.branch == GuidedBranch::Null && g.lead.is_some() {
            ui.collapsing(i.t("guided-null-lead-details"), |ui| {
                ui.label(i.t("guided-null-lead-order"));
                ui.label(i.t("guided-null-lead-empty"));
                details(ui);
            });
        } else if rule.branch == GuidedBranch::FeedPartner && !rule.last {
            ui.collapsing(i.t("guided-feed-early-details"), |ui| {
                ui.label(i.t("guided-feed-early-order"));
                details(ui);
            });
        } else if rule.branch == GuidedBranch::Null && !rule.last {
            ui.collapsing(i.t("guided-null-middle-details"), |ui| {
                ui.label(i.t("guided-null-middle-order"));
                details(ui);
            });
        } else {
            details(ui);
        }
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
    if !g.lead_replies.is_empty() {
        ui.label(
            g.lead_replies
                .iter()
                .map(|&s| source_line(i, s))
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    if g.lead.is_none()
        && matches!(g.step, GuidedStep::Observe | GuidedStep::Complete)
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
        let deduction = i.t(if o.proves_void && null {
            "guided-null-proves-void"
        } else if o.proves_void {
            "guided-proves-void"
        } else {
            "guided-only-card"
        });
        if g.lead.is_some() {
            let winner = o
                .winner
                .map(|seat| i.tf("guided-winner", &[("seat", text::seat(i, seat).into())]))
                .unwrap_or_default();
            ui.label(format!("{deduction} {winner}"));
        } else {
            ui.label(deduction);
            if let Some(seat) = o.winner {
                ui.label(i.tf("guided-winner", &[("seat", text::seat(i, seat).into())]));
            }
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
    figures_panel(ui, i, table, g);
    if return_button && ui.button(i.t("guided-return")).clicked() {
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

/// Kontra/Re: public contract and factor, factor orientation, Easy's
/// condition attempt, then its actual call and the actual next public step.
fn kontra(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
    k: GuidedKontra,
) -> Option<GuidedAction> {
    let mut action = None;
    let re = k.call == GuidedKontraCall::Re;
    // The original decision's public context; after the learner acted the
    // actual next step replaces it, so a stale level is never shown.
    if !matches!(g.step, GuidedStep::Waiting | GuidedStep::Complete) {
        let game = ("game", text::game_type(i, k.game).into());
        ui.label(i.tf(
            if re {
                "guided-kontra-public-re"
            } else {
                "guided-kontra-public-kontra"
            },
            &[game],
        ));
    }
    if k.ouvert_round {
        ui.label(i.t("guided-kontra-ouvert-before"));
    } else if table.contract.is_some_and(|c| c.ouvert) {
        ui.label(i.t("guided-kontra-ouvert-view"));
    }
    let pick = |kontra: &'static str, re_key: &'static str| if re { re_key } else { kontra };
    // Grand: only the four Buben are Trumpf. Null: no Trumpf, so the generic
    // rule supplies no assessment and no count or threshold is shown.
    let (grand, null) = (k.game == GameType::Grand, k.game == GameType::Null);
    let by_game = |farbe: (&'static str, &'static str),
                   g: (&'static str, &'static str),
                   n: (&'static str, &'static str)| {
        let (kontra, re_key) = if null {
            n
        } else if grand {
            g
        } else {
            farbe
        };
        pick(kontra, re_key)
    };
    let figures = [
        ("factor", k.factor.unwrap_or_default().into()),
        ("threshold", k.threshold.unwrap_or_default().into()),
    ];
    let prompt = match g.step {
        GuidedStep::Orient => i.t(pick(
            "guided-kontra-orient-kontra",
            "guided-kontra-orient-re",
        )),
        GuidedStep::Threshold if k.ouvert_round => i.t("guided-kontra-ouvert-question"),
        GuidedStep::Threshold => i.tf(
            by_game(
                (
                    "guided-kontra-threshold-kontra",
                    "guided-kontra-threshold-re",
                ),
                (
                    "guided-kontra-threshold-kontra-grand",
                    "guided-kontra-threshold-re-grand",
                ),
                (
                    "guided-kontra-threshold-kontra-null",
                    "guided-kontra-threshold-re-null",
                ),
            ),
            &figures,
        ),
        GuidedStep::Act if k.ouvert_round => i.t("guided-kontra-ouvert-act"),
        GuidedStep::Act => i.t(pick("guided-kontra-act-kontra", "guided-kontra-act-re")),
        GuidedStep::Waiting => i.t("guided-kontra-waiting"),
        GuidedStep::Complete => i.t("guided-kontra-complete"),
        _ => i.t("guided-unsupported-kontra"),
    };
    ui.label(prompt);
    if g.correct == Some(false) {
        ui.label(i.t("guided-wrong"));
        if ui.button(i.t("guided-retry")).clicked() {
            action = Some(GuidedAction::Retry);
        }
    }
    let answers: Vec<(String, GuidedAnswer)> = match g.step {
        GuidedStep::Orient => [1, 2, 4]
            .map(|f| {
                (
                    i.tf("guided-kontra-factor-button", &[("factor", f.into())]),
                    GuidedAnswer::Factor(f),
                )
            })
            .to_vec(),
        GuidedStep::Threshold if k.ouvert_round => vec![
            (i.t("guided-kontra-ouvert-yes"), GuidedAnswer::Applies(true)),
            (i.t("guided-kontra-ouvert-no"), GuidedAnswer::Applies(false)),
        ],
        GuidedStep::Threshold if null => vec![
            (
                i.t("guided-kontra-applies-yes"),
                GuidedAnswer::Applies(true),
            ),
            (
                i.t("guided-kontra-applies-no"),
                GuidedAnswer::Applies(false),
            ),
        ],
        GuidedStep::Threshold => vec![
            (i.t("guided-kontra-yes"), GuidedAnswer::Within(true)),
            (i.t("guided-kontra-no"), GuidedAnswer::Within(false)),
        ],
        _ => vec![],
    };
    for (label, answer) in answers {
        if ui.button(label).clicked() {
            action = Some(GuidedAction::Answer(answer));
        }
    }
    if let Some(rule) = &g.kontra_rule {
        ui.strong(i.tf(
            "guided-bid-rule",
            &[("action", coach::action(i, table, rule.action).into())],
        ));
        if re && !null {
            ui.label(match rule.skat.len() {
                0 => i.t("guided-ann-skat-hand"),
                _ => {
                    let cards: Vec<_> = rule.skat.iter().map(|c| text::card(i, c)).collect();
                    i.tf(
                        "guided-ann-skat-known",
                        &[("cards", cards.join(", ").into())],
                    )
                }
            });
        }
        let key = match (re, rule.wants) {
            _ if null => pick(
                "guided-kontra-count-kontra-null",
                "guided-kontra-count-re-null",
            ),
            (false, false) if grand => "guided-kontra-count-kontra-grand",
            (false, true) => "guided-kontra-count-kontra-yes",
            (false, false) => "guided-kontra-count-kontra-no",
            (true, true) => "guided-kontra-count-re-yes",
            (true, false) => "guided-kontra-count-re-no",
        };
        if k.ouvert_round {
            ui.label(i.t("guided-kontra-ouvert-rule"));
        } else {
            ui.label(i.tf(
                key,
                &[
                    ("count", rule.count.into()),
                    ("threshold", rule.threshold.into()),
                ],
            ));
        }
        if let Some(card) = &rule.card {
            ui.label(i.tf(
                "guided-kontra-card",
                &[("card", text::card(i, card.card).into())],
            ));
        }
        ui.label(i.t(by_game(
            ("guided-kontra-fact-factor", "guided-pre-strength-count"),
            ("guided-kontra-fact-factor", "guided-pre-strength-count"),
            ("guided-kontra-fact-factor", "guided-kontra-fact-factor"),
        )));
        if !k.ouvert_round {
            ui.label(i.t(by_game(
                ("guided-kontra-caveat-kontra", "guided-kontra-caveat-re"),
                (
                    "guided-kontra-caveat-kontra-grand",
                    "guided-kontra-caveat-re",
                ),
                ("guided-kontra-caveat-null", "guided-kontra-caveat-null"),
            )));
        }
    }
    if let Some(o) = g.kontra_observation {
        ui.label(observation_line(i, o));
        if let GuidedKontraObservation::Declined {
            winner: Some(seat), ..
        } = o
        {
            ui.label(i.tf("guided-winner", &[("seat", text::seat(i, seat).into())]));
        }
    }
    if matches!(g.step, GuidedStep::Orient | GuidedStep::Threshold) {
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
    figures_panel(ui, i, table, g);
    if ui.button(i.t("guided-return")).clicked() {
        action = Some(GuidedAction::Return);
    }
    action
}

/// Separately requested model comparison of the original decision.
fn figures_panel(ui: &mut egui::Ui, i: &I18n, table: &TableView, g: &GuidedView) {
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
}

fn observation_line(i: &I18n, o: GuidedKontraObservation) -> String {
    match o {
        GuidedKontraObservation::DeclinedCall => i.t("guided-kontra-ouvert-declined"),
        GuidedKontraObservation::Reply { seat, re, level } => i.tf(
            if re {
                "guided-kontra-reply-re"
            } else {
                "guided-kontra-reply-kein-re"
            },
            &[
                ("seat", text::seat(i, seat).into()),
                ("factor", level.factor().into()),
            ],
        ),
        GuidedKontraObservation::Resumed { seat, card, level } => i.tf(
            "guided-kontra-resumed",
            &[
                ("seat", text::seat(i, seat).into()),
                ("card", text::card(i, card).into()),
                ("factor", level.factor().into()),
            ],
        ),
        GuidedKontraObservation::Declined {
            card, trick, level, ..
        } => i.tf(
            "guided-kontra-declined",
            &[
                ("card", text::card(i, card).into()),
                ("trick", (trick + 1).into()),
                ("factor", level.factor().into()),
            ],
        ),
    }
}

/// Skataufnahme/Drücken/Ansage: candidate branch, threshold, card or value
/// attempt, then Easy's actual Hand/pickup, discard or announcement.
fn preplay(
    ui: &mut egui::Ui,
    i: &I18n,
    table: &TableView,
    g: &GuidedView,
    p: &skat_app::GuidedPreplay,
) -> Option<GuidedAction> {
    let mut action = None;
    let pickup = p.phase == Phase::SkatAufnahme;
    let ansage = p.phase == Phase::Ansage;
    if let Some(bid) = p.bid {
        // Public facts only: the bid and whether the Skat is known.
        ui.label(i.tf(
            if p.hand_game == Some(true) {
                "guided-ann-public-hand"
            } else {
                "guided-ann-public-pickup"
            },
            &[("bid", bid.into())],
        ));
    }
    let game = p.game.map(|game| text::game_type(i, game));
    let prompt = match (g.step, &game) {
        (GuidedStep::Orient, _) if ansage => i.t("guided-ann-orient"),
        (GuidedStep::Threshold, Some(game)) if ansage => i.tf(
            "guided-ann-threshold",
            &[
                ("game", game.clone().into()),
                ("bid", p.bid.unwrap_or_default().into()),
            ],
        ),
        (GuidedStep::Act, _) if ansage => i.t("guided-ann-act"),
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
        GuidedStep::Threshold if ansage => vec![
            (i.t("guided-ann-fits-yes"), GuidedAnswer::Within(true)),
            (i.t("guided-ann-fits-no"), GuidedAnswer::Within(false)),
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
            if ansage {
                "guided-ann-preferred"
            } else {
                "guided-pre-game"
            },
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
            GuidedPreRule::Announce(r) => announce_answer(ui, i, r),
        }
        ui.label(i.t(if pickup {
            "guided-pre-caveat-pickup"
        } else if ansage {
            "guided-ann-caveat"
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

/// Easy's announcement: preferred value against the bid, then the fitting
/// candidates with the strength tie-break, the Null ouvert condition and
/// every fallback, all from the shared rule.
fn announce_answer(ui: &mut egui::Ui, i: &I18n, r: &GuidedAnnounceRule) {
    ui.label(match r.skat.len() {
        0 => i.t("guided-ann-skat-hand"),
        _ => {
            let cards: Vec<_> = r.skat.iter().map(|c| text::card(i, c)).collect();
            i.tf(
                "guided-ann-skat-known",
                &[("cards", cards.join(", ").into())],
            )
        }
    });
    let values = [("value", r.value.into()), ("bid", r.bid.into())];
    ui.label(i.tf(
        if r.fits {
            "guided-ann-fits"
        } else {
            "guided-ann-short"
        },
        &values,
    ));
    if r.no_fit {
        ui.label(i.tf("guided-ann-no-fit", &[("bid", r.bid.into())]));
    } else if !r.fits {
        let fitting: Vec<_> = r
            .candidates
            .iter()
            .filter(|c| c.fits)
            .map(|c| {
                i.tf(
                    "guided-ann-candidate",
                    &[
                        ("game", text::game_type(i, c.game).into()),
                        ("value", c.value.into()),
                        ("strength", c.strength.into()),
                    ],
                )
            })
            .collect();
        ui.label(i.tf(
            "guided-ann-candidates",
            &[("games", fitting.join(", ").into())],
        ));
        let tied: Vec<_> = r
            .tied()
            .into_iter()
            .map(|g| text::game_type(i, g))
            .collect();
        ui.label(if tied.is_empty() {
            i.tf(
                "guided-ann-strongest",
                &[("game", text::game_type(i, r.game).into())],
            )
        } else {
            i.tf(
                "guided-ann-tie",
                &[
                    ("games", tied.join(", ").into()),
                    ("game", text::game_type(i, r.game).into()),
                ],
            )
        });
    }
    match r.null_risks {
        Some(0) => {
            ui.label(i.t("guided-ann-ouvert"));
        }
        Some(n) => {
            ui.label(i.tf("guided-ann-null-plain", &[("count", n.into())]));
        }
        None => (),
    }
    if r.plain {
        ui.label(i.t("guided-ann-plain"));
    }
}

fn source_line(i: &I18n, source: Source) -> String {
    match source {
        Source::Lead { index, seat } => i.tf(
            "guided-lead-source",
            &[
                ("trick", (index + 1).into()),
                ("seat", text::seat(i, seat).into()),
            ],
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;
    use egui_kittest::{Harness, kittest::Queryable};
    use skat_app::{Action, Announcement, CardSet, GameType, GuidedAnnounceCandidate, Suit};

    /// Review #152: the revealed no-fit answer (Herz Hand cards at bid 77,
    /// see `guided_announce.rs`) names only the known-card comparison and the
    /// Grand fallback; a hidden Skat with SJ makes the actual Grand 96.
    #[test]
    fn no_fit_answer_does_not_claim_a_certain_overbid_in_de_en() {
        let candidate = |game, value, strength| GuidedAnnounceCandidate {
            game,
            value,
            strength,
            fits: false,
        };
        let rule = GuidedAnnounceRule {
            action: Action::Announce(Announcement::plain(GameType::Grand)),
            bid: 77,
            hand_game: true,
            skat: CardSet::EMPTY,
            preferred: GameType::Farbe(Suit::Herz),
            branch: GuidedGameBranch::Suit,
            value: 30,
            fits: false,
            candidates: [
                candidate(GameType::Farbe(Suit::Kreuz), 36, 61),
                candidate(GameType::Farbe(Suit::Pik), 33, 61),
                candidate(GameType::Farbe(Suit::Herz), 30, 106),
                candidate(GameType::Farbe(Suit::Karo), 27, 57),
                candidate(GameType::Grand, 72, 54),
            ],
            game: GameType::Grand,
            null_risks: None,
            ouvert: false,
            no_fit: true,
            plain: false,
            fallback: false,
        };
        for language in Language::ALL {
            let i = I18n::new(language);
            let text = i.tf("guided-ann-no-fit", &[("bid", 77.into())]);
            let ui_i = I18n::new(language);
            let h = Harness::new_ui(move |ui| announce_answer(ui, &ui_i, &rule));
            assert!(h.query_by_label(&text).is_some(), "{language:?}");
            for certain in ["ist überreizt", "is overbid"] {
                assert!(!text.contains(certain), "{language:?}: {text}");
            }
            assert!(text.contains("77") && text.contains("Grand"), "{text}");
        }
    }
}
