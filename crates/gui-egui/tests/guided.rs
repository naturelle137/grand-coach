// SPDX-License-Identifier: EUPL-1.2

//! Real bilingual egui widgets for the guided card-play slice.
#[path = "support/guided.rs"]
mod support;
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{GuidedStep, Level, Opponents, SeriesLength, SessionConfig};
use support::*;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_widgets_allow_wrong_retry_reveal_memory_any_card_observation_and_return_in_de_en() {
    for language in Language::ALL {
        let config = SessionConfig {
            seed: 21,
            series: SeriesLength::Free,
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        };
        let mut h = Harness::builder()
            .with_size([1600.0, 1200.0])
            .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
        enter(&mut h);
        h.state_mut().set_language(language);
        h.run_steps(4);
        let i = I18n::new(language);
        let before = h.state().session().view();
        let history_before = h.state().learning_history().clone();
        let original_id = h.state().session().guided().unwrap().id;
        let en = language == Language::En;
        let t = |de, english| if en { english } else { de };
        let file = std::env::temp_dir().join(format!(
            "guided-replay-{}-{language:?}.json",
            std::process::id()
        ));
        h.state()
            .session()
            .export_current_recording_privileged()
            .unwrap()
            .save_local_new(&file)
            .unwrap();
        click(&mut h, t("Spiel", "Game"));
        click(&mut h, t("Aufnahme öffnen …", "Open recording …"));
        let path_label = t("Dateipfad:", "File path:");
        h.get_by_role_and_label(Role::TextInput, path_label).focus();
        h.run_steps(1);
        h.key_press_modifiers(eframe::egui::Modifiers::COMMAND, eframe::egui::Key::A);
        h.get_by_role_and_label(Role::TextInput, path_label)
            .type_text(file.to_str().unwrap());
        h.run_steps(4);
        click(&mut h, t("Datei öffnen", "Open file"));
        assert!(h.state().session().guided().is_none());
        // The new detached window needs its sizing animation to settle
        // before clicking its bottom control (normal tests interact first).
        h.run_steps(20);
        click(&mut h, t("Aufnahme schließen", "Close recording"));
        h.run_steps(12);
        assert!(h.state_mut().replay_coach_mut().is_none());
        click(&mut h, &i.t("guided-start"));
        assert_ne!(
            h.state()
                .session()
                .guided()
                .expect("re-entry after replay")
                .id,
            original_id
        );
        assert_eq!(
            h.state().session().view().current_trick,
            before.current_trick
        );
        std::fs::remove_file(file).unwrap();
        assert!(
            h.query_by_role_and_label(Role::Button, &i.t("guided-memory"))
                .is_none()
        );
        // A deliberately wrong answer: the player's own seat cannot lead the
        // current trick before they have played their card.
        let own = before.players[0].seat;
        let label = match own {
            skat_app::Seat::Vorhand => i.t("seat-vorhand"),
            skat_app::Seat::Mittelhand => i.t("seat-mittelhand"),
            skat_app::Seat::Hinterhand => i.t("seat-hinterhand"),
        };
        click(&mut h, &label);
        assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &i.t("guided-reveal"));
        assert_eq!(h.state().session().guided().unwrap().step, GuidedStep::Act);
        click(&mut h, &i.t("guided-memory"));
        assert!(h.state().session().guided().unwrap().memory.is_some());
        click(&mut h, &i.t("guided-memory-hide"));
        assert_eq!(
            h.state().session().view().legal_actions,
            before.legal_actions
        );
        let label = coach::action(&i, &before, *before.legal_actions.last().unwrap());
        click(&mut h, &label);
        settle(&mut h);
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        click(&mut h, &i.t("guided-unknown"));
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Complete);
        assert!(g.observation.is_some());
        let original_rule = g.rule;
        h.state_mut().set_coach(skat_app::CoachSettings {
            show_figures: true,
            ..skat_app::CoachSettings::default()
        });
        h.run_steps(4);
        assert!(!h.state().session().guided().unwrap().figures_requested);
        click(&mut h, &i.t("guided-figures"));
        assert!(h.state().session().guided().unwrap().figures_requested);
        assert_eq!(h.state().session().guided().unwrap().rule, original_rule);
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
        // Guided events are not silently recorded as M8 retrieval attempts.
        assert_eq!(h.state().learning_history(), &history_before);
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_reizen_widgets_allow_wrong_retry_reveal_free_call_skip_and_stale_in_de_en() {
    for language in Language::ALL {
        let config = SessionConfig {
            seed: 21,
            series: SeriesLength::Free,
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        };
        let mut h = Harness::builder()
            .with_size([1600.0, 1200.0])
            .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
        let i = I18n::new(language);
        enter_reizen(&mut h, language);
        let before = h.state().session().view();
        let history_before = h.state().learning_history().clone();
        // Initial prompt: public bid and the role question only.
        let first = h.state().session().guided().unwrap();
        assert!(first.bid_rule.is_none());
        assert_eq!(first.reizen.unwrap().current, before.bid);
        assert_eq!(first.reizen.unwrap().holds, None);
        for absent in ["guided-bid-within-yes", "guided-memory", "guided-figures"] {
            assert!(
                h.query_by_role_and_label(Role::Button, &i.t(absent))
                    .is_none()
            );
        }
        // Skip, then re-enter with a new identity.
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
        let after = h.state().session().view();
        assert_eq!(
            (&after.bidding, &after.legal_actions, &after.hand),
            (&before.bidding, &before.legal_actions, &before.hand)
        );
        click(&mut h, &i.t("guided-start"));
        let g = h.state().session().guided().unwrap();
        assert_ne!(g.id, first.id);
        let holds = before.legal_actions.contains(&skat_app::Action::Hold);
        let (wrong, right) = if holds {
            ("guided-bid-asks", "guided-bid-holds")
        } else {
            ("guided-bid-holds", "guided-bid-asks")
        };
        click(&mut h, &i.t(wrong));
        assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &i.t(right));
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Threshold);
        assert_eq!(g.reizen.unwrap().holds, Some(holds));
        assert!(g.bid_rule.is_none());
        click(&mut h, &i.t("guided-unknown"));
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Act);
        let rule = g.bid_rule.unwrap();
        assert_eq!(
            h.state().session().view().legal_actions,
            before.legal_actions
        );
        // Choose freely: a different legal call than the rule's.
        let other = if rule.action == skat_app::Action::Pass {
            before.legal_actions[0]
        } else {
            skat_app::Action::Pass
        };
        let label = match other {
            skat_app::Action::Hold => i.tf("button-hold", &[("value", before.bid.into())]),
            skat_app::Action::Bid(value) => i.tf("button-bid", &[("value", value.into())]),
            _ => i.t("button-pass"),
        };
        click(&mut h, &label);
        assert!(h.state().session().guided().is_none());
        // The free call was actually played; the old dialogue is gone.
        let human = before.players[0].player;
        let calls = h.state().session().view().bidding;
        assert!(calls.len() > before.bidding.len());
        let expected = match other {
            skat_app::Action::Hold => skat_app::Call::Hold(before.bid),
            skat_app::Action::Bid(value) => skat_app::Call::Bid(value),
            _ => skat_app::Call::Pass,
        };
        assert_eq!(calls[before.bidding.len()], (human, expected));
        assert_eq!(h.state().learning_history(), &history_before);
    }
}
