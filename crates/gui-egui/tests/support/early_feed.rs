// SPDX-License-Identifier: EUPL-1.2
//! Ordinary seed22/game3, real widgets; shared by regression and screenshots.
use super::guided_support::{card_name, click, seat_name, settle};
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::*;

/// 0 orientation, 1 attempted rule, 2 observed question, 3 completed feedback.
pub fn flow(h: &mut Harness<'_, GrandCoachApp>, language: Language, stage: u8, chosen: u8) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    let route: Vec<Command> =
        serde_json::from_str(include_str!("../fixtures/early-feed-route.json")).unwrap();
    for cmd in route {
        settle(h);
        let v = h.state().session().view();
        let label = match cmd {
            Command::NextGame => i.t("button-next-game"),
            Command::Act(a) => coach::action(&i, &v, a),
            _ => unreachable!(),
        };
        click(h, &label);
    }
    settle(h);
    let before = h.state().session().view();
    let history = h.state().learning_history().clone();
    assert_eq!(
        (
            before.game_number,
            before.tricks_played,
            before.players[0].seat
        ),
        (3, 0, Seat::Mittelhand)
    );
    assert_eq!(before.current_trick[0].card.index(), 15);
    assert_eq!(
        before.legal_actions,
        [9, 10, 13].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    click(h, &i.t("guided-start"));
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Orient);
    assert!(g.rule.is_none() && g.memory.is_none() && g.figures.is_none() && g.task.is_none());
    assert!(h.query_by_label(&i.t("guided-memory")).is_none());
    if stage == 0 {
        return;
    }
    click(h, &seat_name(&i, Seat::Mittelhand));
    assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
    click(h, &i.t("guided-retry"));
    click(h, &seat_name(&i, Seat::Vorhand));
    click(h, &i.t("guided-none"));
    assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
    click(h, &i.t("guided-retry"));
    click(h, &seat_name(&i, Seat::Hinterhand));
    assert_eq!(
        h.state().session().guided().unwrap().task,
        Some(GuidedBranch::FeedPartner)
    );
    assert!(h.state().session().guided().unwrap().rule.is_none());
    let card = Card::from_index(chosen).unwrap();
    if language == Language::De {
        click(
            h,
            &i.tf("guided-consider", &[("card", card_name(&i, card).into())]),
        );
    } else {
        click(h, &i.t("guided-unknown"));
    }
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Act);
    let r = g.rule.unwrap();
    assert_eq!(
        (r.card.index(), r.branch, r.last, r.table_master),
        (13, GuidedBranch::FeedPartner, false, true)
    );
    assert!(h.query_by_label(&i.t("guided-rule-feed-early")).is_some());
    assert!(h.query_by_label(&i.t("guided-rule-feed")).is_none());
    let details = i.t("guided-feed-early-details");
    h.get_by_role_and_label(eframe::egui::Role::DisclosureTriangle, &details)
        .click();
    h.run_steps(4);
    assert!(h.query_by_label(&i.t("guided-feed-early-order")).is_some());
    h.get_by_role_and_label(eframe::egui::Role::DisclosureTriangle, &details)
        .click();
    h.run_steps(4);
    click(h, &i.t("guided-memory"));
    assert!(h.state().session().guided().unwrap().memory.is_some());
    click(h, &i.t("guided-memory-hide"));
    assert_eq!(
        h.state().session().view().legal_actions,
        before.legal_actions
    );
    assert_eq!(
        h.state().session().view().current_trick,
        before.current_trick
    );
    assert_eq!(h.state().learning_history(), &history);
    if stage == 1 {
        return;
    }
    click(h, &card_name(&i, card));
    settle(h);
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Observe
    );
    assert!(h.state().session().guided().unwrap().observation.is_none());
    if stage == 2 {
        return;
    }
    click(h, &i.t("guided-unknown"));
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Complete);
    let o = g.observation.unwrap();
    let last = h.state().session().view().last_trick.unwrap();
    assert_eq!(
        last.cards
            .iter()
            .map(|p| p.card.index())
            .collect::<Vec<_>>(),
        vec![15, chosen, 28]
    );
    assert_eq!(o.winner, Some(Seat::Hinterhand));
    assert_eq!(
        o.source,
        skat_app::coach_model::Source::Trick {
            index: 0,
            seat: Seat::Hinterhand,
            led: Card::from_index(15).unwrap(),
            card: last.cards[2].card
        }
    );
    assert_eq!(h.state().learning_history(), &history);
}
