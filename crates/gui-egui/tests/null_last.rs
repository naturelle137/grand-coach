// SPDX-License-Identifier: EUPL-1.2

//! Ordinary seed6/game3/trick2 route. Fixture widgets are separate component tests.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod support;
use eframe::egui::Role;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::*;
use support::null_click as click;
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_ouvert_last_ordinary_widgets() {
    for language in Language::ALL {
        let config = SessionConfig {
            seed: 6,
            series: SeriesLength::Fixed(3),
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        };
        let mut h = Harness::builder()
            .with_size([1440.0, 1000.0])
            .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
        enter_null_with_ouvert(&mut h, language, true);
        let i = I18n::new(language);
        assert!(h.state().session().view().contract.unwrap().ouvert);
        click(&mut h, &i.t("guided-reveal"));
        h.get_by_label(&i.t("guided-rule-null"));
        h.state_mut().opponent_delay = 1000.0;
        click(&mut h, &card_name(&i, Card::from_index(16).unwrap()));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_last_ordinary_widgets_bilingual_sizes_all_cards_retry_reveal_return() {
    for language in Language::ALL {
        for narrow in [false, true] {
            for n in [16, 18, 20] {
                let config = SessionConfig {
                    seed: 6,
                    series: SeriesLength::Fixed(3),
                    opponents: Opponents::Levels([Level::Easy; 2]),
                    ..SessionConfig::default()
                };
                let mut h = Harness::builder()
                    .with_size(if narrow {
                        [960.0, 640.0]
                    } else {
                        [1440.0, 1000.0]
                    })
                    .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
                enter_null(&mut h, language);
                let i = I18n::new(language);
                let before = h.state().session().view();
                let history = h.state().learning_history().clone();
                let recording = h.state().session().export_current_recording_privileged();
                let rule = i.tf(
                    "guided-rule-card",
                    &[("card", card_name(&i, Card::from_index(20).unwrap()).into())],
                );
                assert!(h.query_by_label(&rule).is_none());
                assert!(h.query_by_label(&i.t("guided-rule-null")).is_none());
                assert!(h.query_by_label(&i.t("guided-memory")).is_none());
                h.get_by_label(&i.t("guided-null-role"));
                click(&mut h, &seat_name(&i, Seat::Mittelhand));
                h.get_by_label(&i.t("guided-wrong"));
                assert!(h.state().session().guided().unwrap().rule.is_none());
                click(&mut h, &i.t("guided-retry"));
                click(&mut h, &seat_name(&i, Seat::Vorhand));
                click(&mut h, &seat_name(&i, Seat::Hinterhand));
                assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
                click(&mut h, &i.t("guided-retry"));
                click(&mut h, &i.t("guided-none"));
                h.get_by_label(&i.t("guided-null-attempt"));
                let chosen = Card::from_index(n).unwrap();
                click(
                    &mut h,
                    &i.tf("guided-consider", &[("card", card_name(&i, chosen).into())]),
                );
                h.get_by_label(&rule);
                h.get_by_label(&i.t("guided-rule-null"));
                assert_eq!(
                    h.state().session().view().current_trick,
                    before.current_trick
                );
                click(&mut h, &i.t("guided-return"));
                let after = h.state().session().view();
                // Ordinary asynchronous coach results may finish during clicks;
                // every game field and the actual recording must stay unchanged.
                let mut expected = before.clone();
                expected.coach = after.coach.clone();
                assert_eq!(after, expected);
                assert_eq!(
                    h.state().session().export_current_recording_privileged(),
                    recording
                );
                click(&mut h, &i.t("guided-start"));
                click(&mut h, &i.t("guided-reveal"));
                for a in &before.legal_actions {
                    let Action::Play(c) = a else { panic!("card") };
                    assert!(
                        !h.get_by_role_and_label(Role::Button, &card_name(&i, *c))
                            .accesskit_node()
                            .is_disabled()
                    );
                }
                click(&mut h, &i.t("guided-memory"));
                let memory = h.state().session().guided().unwrap().memory.unwrap();
                assert!(memory.skat.is_none() && memory.trumpf.is_none());
                click(&mut h, &i.t("guided-memory-hide"));
                h.state_mut().opponent_delay = 1000.0;
                click(&mut h, &card_name(&i, chosen));
                let g = h.state().session().guided().unwrap();
                assert_eq!(g.step, GuidedStep::Observe);
                assert!(g.observation.is_none() && g.memory.is_none());
                assert_eq!(
                    h.state()
                        .session()
                        .view()
                        .last_trick
                        .unwrap()
                        .cards
                        .into_iter()
                        .map(|p| p.card.index())
                        .collect::<Vec<_>>(),
                    [17, 21, n]
                );
                click(&mut h, &i.t("guided-void-yes"));
                assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
                click(&mut h, &i.t("guided-retry"));
                click(&mut h, &i.t("guided-void-no"));
                h.get_by_label(&i.tf(
                    "guided-winner",
                    &[("seat", seat_name(&i, Seat::Vorhand).into())],
                ));
                assert_eq!(
                    h.state()
                        .session()
                        .guided()
                        .unwrap()
                        .observation
                        .unwrap()
                        .winner,
                    Some(Seat::Vorhand)
                );
                click(&mut h, &i.t("guided-return"));
                assert!(h.state().session().guided().is_none());
                assert_eq!(h.state().learning_history(), &history);
            }
        }
    }
}

#[path = "../examples/support/null_widgets.rs"]
mod fixture_widgets;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn null_last_admitted_fixture_components_bilingual_both_sizes() {
    for name in fixture_widgets::names() {
        fixture_widgets::render(&name, None);
    }
}
