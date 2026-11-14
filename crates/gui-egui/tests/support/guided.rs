// SPDX-License-Identifier: EUPL-1.2

use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{Action, GuidedStep, Phase};

pub fn click(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}

pub fn settle(h: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..5000 {
        h.run_steps(1);
        if !h.state().session().view().opponent_to_move {
            h.run_steps(3);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not settle");
}

/// Ordinary app, real legal widget moves, no injected session or mock view.
pub fn enter(h: &mut Harness<'_, GrandCoachApp>) {
    let i = I18n::new(Language::De);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(skat_app::Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    for _ in 0..300 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Spiel && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            if g.step == GuidedStep::Orient {
                return;
            }
            click(h, &i.t("guided-return"));
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a supported guided branch");
}

/// Ordinary app up to the human's next Reizen decision; enters the dialogue
/// by its real button in `language`.
#[allow(dead_code)]
pub fn enter_reizen(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..300 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Reizen && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            assert_eq!(g.step, GuidedStep::Orient);
            assert!(g.reizen.is_some());
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human Reizen decision");
}

/// Ordinary app: the human holds or bids until declarer, picks up for
/// `Phase::Druecken`, and enters the dialogue at `phase` by its real button.
#[allow(dead_code)]
pub fn enter_preplay(h: &mut Harness<'_, GrandCoachApp>, language: Language, phase: Phase) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..600 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == phase && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            assert_eq!(g.step, GuidedStep::Orient);
            assert_eq!(g.preplay.expect("pre-play dialogue").phase, phase);
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let bid = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) => Some(*value),
            _ => None,
        });
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Hold) {
            i.tf("button-hold", &[("value", v.bid.into())])
        } else if let Some(value) = bid {
            i.tf("button-bid", &[("value", value.into())])
        } else if v.legal_actions.contains(&Action::TakeSkat) {
            i.t("button-take-skat")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human {phase:?} decision");
}
