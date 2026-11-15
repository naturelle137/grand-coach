// SPDX-License-Identifier: EUPL-1.2

//! Ordinary seed0/game3/trick2 route, MH DECLARER only; no fixture injection.
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
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn grand_win_early_frozen_ordinary_widgets_bilingual_retry_consent_free_play_actual_winner() {
    for language in Language::ALL {
        for narrow in [false, true] {
            let config = SessionConfig {
                seed: 0,
                series: SeriesLength::Fixed(3),
                kontra_re: false,
                opponents: Opponents::Levels([Level::Easy; 2]),
                ..SessionConfig::default()
            };
            let mut h = Harness::builder()
                .with_size([1440.0, 1000.0])
                .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
            enter_early(&mut h, language);
            if narrow {
                h.set_size([960.0, 640.0].into());
                h.run_steps(4);
            }
            let i = I18n::new(language);
            let dj = Card::new(Suit::Karo, Rank::Bube);
            let s8 = Card::new(Suit::Pik, Rank::Acht);
            let before = h.state().session().view();
            let history = h.state().learning_history().clone();
            if !narrow {
                let id = h.state().session().guided().unwrap().id;
                let file = std::env::temp_dir().join(format!(
                    "grand-early-replay-{}-{language:?}.json",
                    std::process::id()
                ));
                h.state()
                    .session()
                    .export_current_recording_privileged()
                    .unwrap()
                    .save_local_new(&file)
                    .unwrap();
                let en = language == Language::En;
                let t = |de, en_text| if en { en_text } else { de };
                click(&mut h, t("Spiel", "Game"));
                click(&mut h, t("Aufnahme öffnen …", "Open recording …"));
                let label = t("Dateipfad:", "File path:");
                h.get_by_role_and_label(Role::TextInput, label).focus();
                h.run_steps(1);
                h.key_press_modifiers(eframe::egui::Modifiers::COMMAND, eframe::egui::Key::A);
                h.get_by_role_and_label(Role::TextInput, label)
                    .type_text(file.to_str().unwrap());
                h.run_steps(4);
                click(&mut h, t("Datei öffnen", "Open file"));
                assert!(h.state().session().guided().is_none());
                h.run_steps(20);
                click(&mut h, t("Aufnahme schließen", "Close recording"));
                h.run_steps(12);
                click(&mut h, &i.t("guided-start"));
                assert_ne!(h.state().session().guided().unwrap().id, id);
                assert_eq!(
                    h.state().session().view().legal_actions,
                    before.legal_actions
                );
                std::fs::remove_file(file).unwrap();
            }
            let rule_card = i.tf("guided-rule-card", &[("card", card_name(&i, dj).into())]);
            assert!(h.query_by_label(&rule_card).is_none());
            assert!(h.query_by_label(&i.t("guided-early-trumpf")).is_none());
            assert!(h.query_by_label(&i.t("guided-memory")).is_none());
            click(&mut h, &seat_name(&i, Seat::Mittelhand));
            assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
            click(&mut h, &i.t("guided-retry"));
            click(&mut h, &seat_name(&i, Seat::Vorhand));
            click(&mut h, &i.t("guided-none"));
            assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
            click(&mut h, &i.t("guided-retry"));
            click(&mut h, &seat_name(&i, Seat::Hinterhand));
            let consider = |c| i.tf("guided-consider", &[("card", card_name(&i, c).into())]);
            click(&mut h, &consider(s8));
            assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
            click(&mut h, &i.t("guided-retry"));
            // Attempt and deliberate reveal both exercised at the same turn.
            click(&mut h, &consider(dj));
            assert_eq!(h.state().session().guided().unwrap().rule.unwrap().card, dj);
            click(&mut h, &i.t("guided-return"));
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            click(&mut h, &i.t("guided-start"));
            click(
                &mut h,
                &i.t(if language == Language::De {
                    "guided-reveal"
                } else {
                    "guided-unknown"
                }),
            );
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.rule.unwrap().candidates, vec![dj]);
            assert_eq!(g.early_condition, Some(GuidedEarlyCondition::Trumpf));
            assert!(g.memory.is_none());
            h.get_by_label(&rule_card);
            h.get_by_label(&i.t("guided-early-trumpf"));
            // Card controls stay enabled, including the losing non-Easy S8.
            for action in &before.legal_actions {
                let Action::Play(c) = action else {
                    panic!("card")
                };
                assert!(
                    !h.get_by_role_and_label(Role::Button, &card_name(&i, *c))
                        .accesskit_node()
                        .is_disabled()
                );
            }
            click(&mut h, &i.t("guided-memory"));
            assert_eq!(
                h.state().session().guided().unwrap().memory.unwrap().skat,
                None
            );
            click(&mut h, &i.t("guided-memory-hide"));
            click(&mut h, &card_name(&i, s8));
            settle(&mut h);
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.step, GuidedStep::Observe);
            let source = g.observed_source.unwrap();
            assert_eq!(
                source,
                coach_model::Source::Trick {
                    index: 1,
                    seat: Seat::Hinterhand,
                    led: Card::new(Suit::Kreuz, Rank::Zehn),
                    card: Card::new(Suit::Kreuz, Rank::Acht),
                }
            );
            eprintln!("#161 ordinary {language:?} narrow={narrow} actual third={source:?}");
            assert!(g.memory.is_none() && g.observation.is_none());
            click(&mut h, &i.t("guided-void-no"));
            let o = h.state().session().guided().unwrap().observation.unwrap();
            assert!(!o.proves_void);
            assert_eq!(o.winner, Some(Seat::Vorhand));
            eprintln!("#161 actual completed winner={:?}", o.winner);
            click(&mut h, &i.t("guided-return"));
            assert!(h.state().session().guided().is_none());
            assert_eq!(h.state().learning_history(), &history);
        }
    }
}
