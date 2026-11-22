// SPDX-License-Identifier: EUPL-1.2
//! Real bilingual Ouvert declarer WinLast widget flow.

#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

#[test]
fn suit_ouvert_last_real_widgets_bilingual() {
    for language in Language::ALL {
        let i = I18n::new(language);
        let mut h = Harness::builder()
            .with_size([1440.0, 1000.0])
            .build_eframe(|cc| {
                GrandCoachApp::new(
                    &cc.egui_ctx,
                    SessionConfig {
                        seed: 0,
                        series: SeriesLength::Fixed(1),
                        opponents: Opponents::Levels([Level::Easy; 2]),
                        kontra_re: false,
                        ..SessionConfig::default()
                    },
                )
            });
        h.state_mut().set_language(language);
        h.state_mut().opponent_delay = 0.0;
        h.state_mut().set_coach_budget(Budget {
            samples: 1,
            nodes_per_task: 1,
        });
        for _ in 0..16 {
            support::settle(&mut h);
            let v = h.state().session().view();
            assert!(v.legal_actions.contains(&Action::Hold));
            support::click(&mut h, &i.tf("button-hold", &[("value", v.bid.into())]));
        }
        support::settle(&mut h);
        support::click(&mut h, &i.t("button-hand"));
        support::settle(&mut h);
        h.get_by_role_and_label(
            Role::RadioButton,
            &support::game_name(&i, GameType::Farbe(Suit::Kreuz)),
        )
        .click();
        h.run_steps(4);
        h.get_by_role_and_label(Role::CheckBox, &i.t("contract-ouvert"))
            .click();
        h.run_steps(4);
        support::click(&mut h, &i.t("button-announce"));
        for (suit, rank) in [
            (Suit::Kreuz, Rank::Zehn),
            (Suit::Karo, Rank::Sieben),
            (Suit::Karo, Rank::Neun),
            (Suit::Herz, Rank::Sieben),
            (Suit::Pik, Rank::Dame),
            (Suit::Kreuz, Rank::Bube),
            (Suit::Kreuz, Rank::Dame),
        ] {
            support::settle(&mut h);
            let card = Card::new(suit, rank);
            assert!(
                h.state()
                    .session()
                    .view()
                    .legal_actions
                    .contains(&Action::Play(card))
            );
            support::click(&mut h, &support::card_name(&i, card));
        }
        support::settle(&mut h);
        let v = h.state().session().view();
        assert_eq!(v.current_trick.len(), 2);
        assert_eq!(v.declarer, Some(PlayerId::HUMAN));
        support::click(&mut h, &i.t("guided-start"));
        support::null_click(&mut h, &i.t("guided-unknown"));
        assert_eq!(
            h.state().session().guided().unwrap().rule.unwrap().branch,
            GuidedBranch::WinLast
        );
        let card = match v.legal_actions[0] {
            Action::Play(card) => card,
            _ => panic!("expected card"),
        };
        support::null_click(&mut h, &support::card_name(&i, card));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        support::null_click(&mut h, &i.t("guided-unknown"));
        assert!(
            h.state()
                .session()
                .guided()
                .unwrap()
                .observation
                .unwrap()
                .winner
                .is_some()
        );
        support::null_click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
    }
}
