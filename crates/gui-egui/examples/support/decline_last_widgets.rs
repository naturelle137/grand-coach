// SPDX-License-Identifier: EUPL-1.2

//! Ordinary seed-0 declarer routes through the last-seat decline dialogue.
use super::guided_support as support;
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;

pub fn names() -> Vec<String> {
    let mut names = Vec::new();
    for game in ["grand", "herz"] {
        for stage in ["orient", "narrow", "rule", "observed"] {
            for language in ["de", "en"] {
                for size in ["", "-narrow"] {
                    names.push(format!(
                        "guided-declarer-decline-{game}-{stage}-{language}{size}"
                    ));
                }
            }
        }
    }
    names
}

pub fn render(name: &str, out: &str) {
    let language = if name.contains("-en") {
        Language::En
    } else {
        Language::De
    };
    let game = if name.contains("-grand-") {
        GameType::Grand
    } else {
        GameType::Farbe(Suit::Herz)
    };
    let i = I18n::new(language);
    let size = if name.ends_with("-narrow") {
        [960.0, 640.0]
    } else {
        [1440.0, 1000.0]
    };
    let mut h = Harness::builder()
        .with_size(size)
        .wgpu()
        .build_eframe(|cc| {
            GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed: 0,
                    series: SeriesLength::Fixed(3),
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
    for _ in 0..7 {
        support::settle(&mut h);
        let bid = h.state().session().view().bid;
        support::click(&mut h, &i.tf("button-hold", &[("value", bid.into())]));
    }
    support::settle(&mut h);
    support::click(&mut h, &i.t("button-pass"));
    for index in [3, 4, 5, 12, 13, 16, 24, 26, 30, 28] {
        support::settle(&mut h);
        support::click(
            &mut h,
            &support::card_name(&i, Card::from_index(index).unwrap()),
        );
    }
    support::settle(&mut h);
    support::click(&mut h, &i.t("button-next-game"));
    for value in [24, 27] {
        support::settle(&mut h);
        support::click(&mut h, &i.tf("button-bid", &[("value", value.into())]));
    }
    support::settle(&mut h);
    support::click(&mut h, &i.t("button-hand"));
    support::settle(&mut h);
    h.get_by_role_and_label(Role::RadioButton, &support::game_name(&i, game))
        .click();
    h.run_steps(4);
    support::click(&mut h, &i.t("button-announce"));
    support::settle(&mut h);
    let v = h.state().session().view();
    assert_eq!(v.declarer, Some(PlayerId::HUMAN));
    assert_eq!(v.current_trick.len(), 2);
    assert_eq!(v.contract.unwrap().game, game);
    let expected_winner = v.current_trick[0].player.seat(v.dealer);
    support::click(&mut h, &i.t("guided-start"));
    if !name.contains("-orient-") {
        support::click(&mut h, &support::seat_name(&i, expected_winner));
        support::click(&mut h, &i.t("guided-none"));
        if name.contains("-narrow-") {
            h.run_steps(4);
            h.render()
                .unwrap()
                .save(format!("{out}/{name}.png"))
                .unwrap();
            return;
        }
        let card = Card::from_index(26).unwrap();
        support::click(
            &mut h,
            &i.tf(
                "guided-consider",
                &[("card", support::card_name(&i, card).into())],
            ),
        );
        let g = h.state().session().guided().unwrap();
        assert!(g.decline_last);
        let card = g.rule.unwrap().card;
        assert_eq!(card.index(), 26);
        h.get_by_label(&i.t("guided-rule-decline-last"));
        if name.contains("-observed-") {
            h.state_mut().opponent_delay = 1000.0;
            support::click(&mut h, &support::card_name(&i, card));
            assert_eq!(
                h.state().session().guided().unwrap().step,
                GuidedStep::Observe
            );
            support::click(&mut h, &i.t("guided-reveal"));
            assert_eq!(
                h.state()
                    .session()
                    .guided()
                    .unwrap()
                    .observation
                    .unwrap()
                    .winner,
                Some(expected_winner)
            );
        }
    }
    h.run_steps(4);
    h.render()
        .unwrap()
        .save(format!("{out}/{name}.png"))
        .unwrap();
}
