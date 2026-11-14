// SPDX-License-Identifier: EUPL-1.2

//! Actual seed-1020 second-game jump, shared by regression and screenshots.
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language};
use skat_app::coach_model::LessonContent;
use skat_app::{Action, Budget, Level, Opponents, SeriesLength, SessionConfig};

pub fn config() -> SessionConfig {
    SessionConfig {
        seed: 1020,
        series: SeriesLength::Fixed(3),
        kontra_re: true,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    }
}

fn click(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}

fn settle(h: &mut Harness<'_, GrandCoachApp>) {
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

pub fn jump(h: &mut Harness<'_, GrandCoachApp>) {
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    // Finish game 1 as Grand Hand through the real widgets, no injected state.
    for _ in 0..100 {
        settle(h);
        let v = h.state().session().view();
        if v.game_number == 2 {
            break;
        }
        let legal = &v.legal_actions;
        let label = if v.can_next_game {
            "Nächstes Spiel".to_owned()
        } else if legal.contains(&Action::Hold) {
            format!("Ja, {} halten", v.bid)
        } else if let Some(Action::Bid(b)) = legal.iter().find(|a| matches!(a, Action::Bid(_))) {
            format!("{b} sagen")
        } else if legal.contains(&Action::Hand) {
            "Hand spielen".to_owned()
        } else if legal.iter().any(|a| matches!(a, Action::Announce(_))) {
            "Ansagen".to_owned()
        } else if legal.contains(&Action::KeinRe) {
            "Kein Re".to_owned()
        } else if let Some(Action::Play(card)) = legal.first() {
            let suit = ["Kreuz", "Pik", "Herz", "Karo"][usize::from(card.suit().index())];
            let rank = [
                "Sieben", "Acht", "Neun", "Zehn", "Bube", "Dame", "König", "Ass",
            ][usize::from(card.rank().index())];
            format!("{suit}-{rank}")
        } else {
            panic!("unexpected first-game actions: {legal:?}");
        };
        click(h, &label);
    }
    assert_eq!(h.state().session().view().game_number, 2);
    click(h, "20 sagen");
    settle(h);
    let before = h.state().session().view();
    assert_eq!(before.bid, 20);
    assert!(before.legal_actions.contains(&Action::Bid(22)));
    assert!(before.legal_actions.contains(&Action::Bid(35)));
    h.get_by_role(Role::ComboBox).click();
    h.run_steps(4);
    h.get_by_label("35").click();
    h.run_steps(4);
    click(h, "Springen");
    settle(h);
    for _ in 0..5000 {
        h.run_steps(1);
        let v = h.state().session().view();
        if let Some(last) = v.coach.last_move
            && matches!(
                last.lesson,
                Some(LessonContent::GameValue {
                    value: 22,
                    bidding: true,
                    ..
                })
            )
        {
            assert_eq!(v.bid, 35);
            assert_eq!(last.action, Action::Bid(35));
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("no historical game-value lesson after the actual jump");
}

pub fn assert_copy(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    h.state_mut().set_language(language);
    h.run_steps(4);
    let (context, jump) = match language {
        Language::De => (
            "Lage vor deiner Entscheidung",
            "Bei einem höheren Reizsprung gilt dessen Wert",
        ),
        Language::En => (
            "position before your decision",
            "A higher jump bid commits you to its value",
        ),
    };
    h.get_by_label_contains(context);
    h.get_by_label_contains(jump);
}
