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

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_pickup_and_discard_widgets_hide_answers_until_reveal_and_stay_free_in_de_en() {
    use skat_app::{Action, GuidedPreRule, Phase};
    for language in Language::ALL {
        for phase in [Phase::SkatAufnahme, Phase::Druecken] {
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
            enter_preplay(&mut h, language, phase);
            let before = h.state().session().view();
            let history_before = h.state().learning_history().clone();
            // Initial prompt: only the branch question; no game, threshold or rule.
            let g = h.state().session().guided().unwrap();
            let p = g.preplay.clone().unwrap();
            assert_eq!((p.game, p.threshold, g.pre_rule), (None, None, None));
            for absent in [
                "guided-pre-hand-yes",
                "guided-pre-strength-count",
                "guided-memory",
                "guided-figures",
            ] {
                assert!(
                    h.query_by_role_and_label(Role::Button, &i.t(absent))
                        .is_none()
                );
            }
            // Skip, then re-enter with a new identity; the table is unchanged.
            click(&mut h, &i.t("guided-return"));
            assert!(h.state().session().guided().is_none());
            let after = h.state().session().view();
            assert_eq!(
                (&after.legal_actions, &after.hand),
                (&before.legal_actions, &before.hand)
            );
            click(&mut h, &i.t("guided-start"));
            assert_ne!(h.state().session().guided().unwrap().id, g.id);
            // Wrong branch, retry, then the deliberate reveal.
            let branches = [
                "guided-pre-grand",
                "guided-pre-suit",
                "guided-pre-null",
                "guided-pre-weak",
            ];
            click(&mut h, &i.t(branches[0]));
            let mut right = h.state().session().guided().unwrap().correct == Some(true)
                || h.state().session().guided().unwrap().step == GuidedStep::Threshold;
            for key in &branches[1..] {
                if right {
                    break;
                }
                click(&mut h, &i.t("guided-retry"));
                click(&mut h, &i.t(key));
                right = h.state().session().guided().unwrap().step == GuidedStep::Threshold;
            }
            assert!(right, "one branch is Easy's");
            let g = h.state().session().guided().unwrap();
            assert!(g.preplay.unwrap().game.is_some() && g.pre_rule.is_none());
            click(&mut h, &i.t("guided-reveal"));
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.step, GuidedStep::Act);
            let rule = g.pre_rule.unwrap();
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            // Choose freely: a different legal action than the rule's.
            match rule {
                GuidedPreRule::Pickup(r) => {
                    let (other, key) = if r.action == Action::Hand {
                        (Action::TakeSkat, "button-take-skat")
                    } else {
                        (Action::Hand, "button-hand")
                    };
                    click(&mut h, &i.t(key));
                    assert!(h.state().session().guided().is_none());
                    assert_eq!(
                        h.state().session().view().hand_game,
                        Some(other == Action::Hand)
                    );
                }
                GuidedPreRule::Discard(_) | GuidedPreRule::Announce(_) => {
                    click(&mut h, &i.t("guided-return"));
                    assert!(h.state().session().guided().is_none());
                    assert_eq!(h.state().session().view().phase, Phase::Druecken);
                }
            }
            assert_eq!(h.state().learning_history(), &history_before);
        }
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_ansage_widgets_hide_answers_until_reveal_and_stay_free_in_de_en() {
    use skat_app::{Action, Announcement, GameType, GuidedPreRule, Phase, Suit};
    // Seed 21: the preferred game reaches bid 22 (Hand and pickup); seed 7:
    // Hand at bid 36, Herz falls short and Kreuz is announced.
    for language in Language::ALL {
        for (seed, hand) in [(21, true), (21, false), (7, true)] {
            let config = SessionConfig {
                seed,
                series: SeriesLength::Free,
                opponents: Opponents::Levels([Level::Easy; 2]),
                ..SessionConfig::default()
            };
            let mut h = Harness::builder()
                .with_size([1600.0, 1200.0])
                .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
            let i = I18n::new(language);
            enter_ansage(&mut h, language, hand);
            let before = h.state().session().view();
            let history_before = h.state().learning_history().clone();
            // Initial prompt: public bid and Hand state only; no game or rule.
            let g = h.state().session().guided().unwrap();
            let p = g.preplay.clone().unwrap();
            assert_eq!((p.game, g.pre_rule), (None, None));
            assert_eq!((p.bid, p.hand_game), (Some(before.bid), Some(hand)));
            let public = if hand {
                "guided-ann-public-hand"
            } else {
                "guided-ann-public-pickup"
            };
            assert!(
                h.query_by_label(&i.tf(public, &[("bid", before.bid.into())]))
                    .is_some()
            );
            for absent in ["guided-ann-fits-yes", "guided-memory", "guided-figures"] {
                assert!(
                    h.query_by_role_and_label(Role::Button, &i.t(absent))
                        .is_none()
                );
            }
            assert!(h.query_by_label(&i.t("guided-ann-caveat")).is_none());
            // Skip, then re-enter with a new identity; the table is unchanged.
            click(&mut h, &i.t("guided-return"));
            assert!(h.state().session().guided().is_none());
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            click(&mut h, &i.t("guided-start"));
            assert_ne!(h.state().session().guided().unwrap().id, g.id);
            // Wrong branch, retry, until Easy's branch.
            let branches = [
                "guided-pre-grand",
                "guided-pre-suit",
                "guided-pre-null",
                "guided-pre-weak",
            ];
            let mut right = false;
            for (n, key) in branches.iter().enumerate() {
                if n > 0 {
                    click(&mut h, &i.t("guided-retry"));
                }
                click(&mut h, &i.t(key));
                right = h.state().session().guided().unwrap().step == GuidedStep::Threshold;
                if right {
                    break;
                }
            }
            assert!(right, "one branch is Easy's");
            let g = h.state().session().guided().unwrap();
            assert!(g.preplay.unwrap().game.is_some() && g.pre_rule.is_none());
            if hand {
                // Value attempt: a wrong answer can be retried; the right one reveals.
                click(&mut h, &i.t("guided-ann-fits-yes"));
                let g = h.state().session().guided().unwrap();
                if g.step != GuidedStep::Act {
                    assert_eq!(g.correct, Some(false));
                    assert!(g.pre_rule.is_none());
                    click(&mut h, &i.t("guided-retry"));
                    click(&mut h, &i.t("guided-ann-fits-no"));
                }
                assert_eq!(h.state().session().guided().unwrap().correct, Some(true));
            } else {
                click(&mut h, &i.t("guided-unknown"));
            }
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.step, GuidedStep::Act);
            let Some(GuidedPreRule::Announce(rule)) = g.pre_rule else {
                panic!("announce rule");
            };
            assert_eq!(rule.hand_game, hand);
            assert_eq!(
                rule.action,
                Action::Announce(Announcement {
                    ouvert: rule.ouvert,
                    ..Announcement::plain(rule.game)
                })
            );
            assert!(h.query_by_label(&i.t("guided-ann-caveat")).is_some());
            // Rendered comparison: value against the bid, then the candidate.
            let values = [("value", rule.value.into()), ("bid", rule.bid.into())];
            let fit = if rule.fits {
                "guided-ann-fits"
            } else {
                "guided-ann-short"
            };
            assert!(h.query_by_label(&i.tf(fit, &values)).is_some());
            assert_eq!(rule.fits, seed == 21);
            if !rule.fits {
                let game = [("game", game_name(&i, rule.game).into())];
                assert!(
                    h.query_by_label(&i.tf("guided-ann-strongest", &game))
                        .is_some()
                );
            }
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            // Choose freely: a different legal trump game than the rule's.
            let other = if rule.game == GameType::Grand {
                GameType::Farbe(Suit::Kreuz)
            } else {
                GameType::Grand
            };
            h.get_by_role_and_label(Role::RadioButton, &game_name(&i, other))
                .click();
            h.run_steps(4);
            click(&mut h, &i.t("button-announce"));
            assert!(h.state().session().guided().is_none());
            let v = h.state().session().view();
            assert_eq!(v.phase, Phase::Spiel);
            assert_eq!(v.contract.unwrap().game, other);
            assert_eq!(h.state().learning_history(), &history_before);
        }
    }
}

fn kontra_app(seed: u64) -> Harness<'static, GrandCoachApp> {
    let config = SessionConfig {
        seed,
        series: SeriesLength::Free,
        kontra_re: true,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    };
    Harness::builder()
        .with_size([1600.0, 1200.0])
        .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config))
}

/// Ordinary app, real widgets: seed 18 (defender, five Trumpf, Easy says
/// Kontra) and seed 0 (defender, Easy declines with a card). Wrong factor,
/// retry, wrong condition, retry, correct attempt; the human then plays a
/// card (seed 18) or says Kontra (seed 0) and the actual next public step
/// is shown. Answers never act on the table.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_kontra_widgets_hide_answers_until_attempt_and_observe_actual_step_in_de_en() {
    use skat_app::{Action, GuidedKontraObservation};
    for language in Language::ALL {
        for (seed, wants) in [(18, true), (0, false)] {
            let mut h = kontra_app(seed);
            let i = I18n::new(language);
            enter_kontra(&mut h, language, false);
            let before = h.state().session().view();
            let history_before = h.state().learning_history().clone();
            let g = h.state().session().guided().unwrap();
            let k = g.kontra.unwrap();
            let game = ("game", game_name(&i, k.game).into());
            assert!(
                h.query_by_label(&i.tf("guided-kontra-public-kontra", std::slice::from_ref(&game)))
                    .is_some()
            );
            assert!(
                h.query_by_label(&i.t("guided-kontra-orient-kontra"))
                    .is_some()
            );
            for absent in ["guided-kontra-yes", "guided-memory", "guided-figures"] {
                assert!(h.query_by_label(&i.t(absent)).is_none(), "{absent}");
            }
            assert!(
                h.query_by_label(&i.t("guided-kontra-caveat-kontra"))
                    .is_none()
            );
            let factor = |f: i32| i.tf("guided-kontra-factor-button", &[("factor", f.into())]);
            click(&mut h, &factor(4));
            assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
            click(&mut h, &i.t("guided-retry"));
            click(&mut h, &factor(2));
            let figures = [("factor", 2.into()), ("threshold", 5.into())];
            assert!(
                h.query_by_label(&i.tf("guided-kontra-threshold-kontra", &figures))
                    .is_some()
            );
            assert!(h.state().session().guided().unwrap().kontra_rule.is_none());
            let (right, wrong) = if wants {
                ("guided-kontra-yes", "guided-kontra-no")
            } else {
                ("guided-kontra-no", "guided-kontra-yes")
            };
            click(&mut h, &i.t(wrong));
            assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
            assert!(h.state().session().guided().unwrap().kontra_rule.is_none());
            click(&mut h, &i.t("guided-retry"));
            click(&mut h, &i.t(right));
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.step, GuidedStep::Act);
            let rule = g.kontra_rule.clone().unwrap();
            assert_eq!(rule.wants, wants);
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            assert_eq!(h.state().session().view().hand, before.hand);
            let shown = coach::action(&i, &before, rule.action);
            assert!(
                h.query_by_label(&i.tf("guided-bid-rule", &[("action", shown.into())]))
                    .is_some()
            );
            let count = [
                ("count", rule.count.into()),
                ("threshold", rule.threshold.into()),
            ];
            let key = if wants {
                "guided-kontra-count-kontra-yes"
            } else {
                "guided-kontra-count-kontra-no"
            };
            assert!(h.query_by_label(&i.tf(key, &count)).is_some());
            assert!(h.query_by_label(&i.t("guided-kontra-act-kontra")).is_some());
            assert!(
                h.query_by_label(&i.t("guided-kontra-fact-factor"))
                    .is_some()
            );
            assert!(
                h.query_by_label(&i.t("guided-kontra-caveat-kontra"))
                    .is_some()
            );
            if let Some(card) = &rule.card {
                let name = ("card", card_name(&i, card.card).into());
                assert!(
                    h.query_by_label(&i.tf("guided-kontra-card", &[name]))
                        .is_some()
                );
            }
            // The human chooses the other option than Easy, by real widgets.
            if wants {
                let card = before
                    .legal_actions
                    .iter()
                    .find_map(|a| match a {
                        Action::Play(c) => Some(*c),
                        _ => None,
                    })
                    .unwrap();
                click(&mut h, &card_name(&i, card));
                let g = h.state().session().guided().unwrap();
                assert!(matches!(
                    g.kontra_observation,
                    Some(GuidedKontraObservation::Declined { card: c, .. }) if c == card
                ));
            } else {
                click(&mut h, &i.t("button-kontra"));
                settle(&mut h);
                let g = h.state().session().guided().unwrap();
                assert_eq!(g.step, GuidedStep::Complete);
                assert!(matches!(
                    g.kontra_observation,
                    Some(GuidedKontraObservation::Reply { .. })
                ));
            }
            assert!(h.query_by_label(&i.t("guided-kontra-complete")).is_some());
            assert_eq!(h.state().learning_history(), &history_before);
            click(&mut h, &i.t("guided-return"));
            assert!(h.state().session().guided().is_none());
        }
    }
}

/// Ordinary app, real widgets, seed 1: the human declares the suit it holds
/// fewest of, a defender says Kontra and the human answers. Deliberate
/// reveal, then Re (whatever Easy says); the resumed card play is shown.
/// Seed 848, second game: the Null ouvert round gets the honest exclusion text.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_re_widgets_reveal_free_reply_resumed_play_and_ouvert_fallback_in_de_en() {
    use skat_app::{Action, GuidedKontraObservation, KontraLevel};
    for language in Language::ALL {
        let mut h = kontra_app(1);
        let i = I18n::new(language);
        enter_kontra(&mut h, language, true);
        let k = h.state().session().guided().unwrap().kontra.unwrap();
        let game = ("game", game_name(&i, k.game).into());
        assert!(
            h.query_by_label(&i.tf("guided-kontra-public-re", &[game]))
                .is_some()
        );
        assert!(h.query_by_label(&i.t("guided-kontra-orient-re")).is_some());
        click(
            &mut h,
            &i.tf("guided-kontra-factor-button", &[("factor", 4.into())]),
        );
        let figures = [("factor", 4.into()), ("threshold", 90.into())];
        assert!(
            h.query_by_label(&i.tf("guided-kontra-threshold-re", &figures))
                .is_some()
        );
        click(&mut h, &i.t("guided-reveal"));
        let rule = h.state().session().guided().unwrap().kontra_rule.unwrap();
        let key = if rule.wants {
            "guided-kontra-count-re-yes"
        } else {
            "guided-kontra-count-re-no"
        };
        let count = [
            ("count", rule.count.into()),
            ("threshold", rule.threshold.into()),
        ];
        assert!(h.query_by_label(&i.tf(key, &count)).is_some());
        assert!(
            h.query_by_label(&i.t("guided-pre-strength-count"))
                .is_some()
        );
        assert!(h.query_by_label(&i.t("guided-kontra-caveat-re")).is_some());
        assert!(h.query_by_label(&i.t("guided-kontra-act-re")).is_some());
        click(&mut h, &i.t("button-re"));
        settle(&mut h);
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Complete);
        let Some(GuidedKontraObservation::Resumed { seat, card, level }) = g.kontra_observation
        else {
            panic!("resumed play expected");
        };
        assert_eq!(level, KontraLevel::Re);
        let line = i.tf(
            "guided-kontra-resumed",
            &[
                ("seat", seat_name(&i, seat).into()),
                ("card", card_name(&i, card).into()),
                ("factor", 4.into()),
            ],
        );
        assert!(h.query_by_label(&line).is_some());
        click(&mut h, &i.t("guided-return"));

        let mut h = kontra_app(848);
        enter_kontra_in(&mut h, language, false, KontraAt::Ouvert);
        assert!(
            h.query_by_label(&i.t("guided-unsupported-kontra-ouvert-round"))
                .is_some()
        );
        assert!(h.query_by_label(&i.t("guided-kontra-yes")).is_none());
        assert!(
            h.state()
                .session()
                .view()
                .legal_actions
                .contains(&Action::Kontra)
        );
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
    }
}

/// Ordinary app, real widgets (#155). Seed 848, first game: Grand, the human
/// defends below Easy's five Trumpf (at most the four Buben); wrong "yes",
/// retry, correct "no", then the human says Kontra anyway and the actual
/// declarer reply is shown. Seed 1282, first game: Null; the applicability
/// question shows no count or threshold, "applies" is wrong, then the human
/// declines with a card and that actual card is shown. Answers never act on
/// the table.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_grand_and_null_kontra_widgets_explain_rule_limits_and_observe_actual_step_in_de_en() {
    use skat_app::{Action, GameType, GuidedKontraObservation};
    for language in Language::ALL {
        for (seed, at) in [(848, KontraAt::Grand), (1282, KontraAt::Null)] {
            let grand = at == KontraAt::Grand;
            let mut h = kontra_app(seed);
            let i = I18n::new(language);
            enter_kontra_in(&mut h, language, false, at);
            let before = h.state().session().view();
            let history_before = h.state().learning_history().clone();
            let k = h.state().session().guided().unwrap().kontra.unwrap();
            assert_eq!(
                k.game,
                if grand {
                    GameType::Grand
                } else {
                    GameType::Null
                }
            );
            let game = ("game", game_name(&i, k.game).into());
            assert!(
                h.query_by_label(&i.tf("guided-kontra-public-kontra", &[game]))
                    .is_some()
            );
            click(
                &mut h,
                &i.tf("guided-kontra-factor-button", &[("factor", 2.into())]),
            );
            let k = h.state().session().guided().unwrap().kontra.unwrap();
            let (prompt, right, wrong) = if grand {
                assert_eq!(k.threshold, Some(5));
                (
                    "guided-kontra-threshold-kontra-grand",
                    "guided-kontra-no",
                    "guided-kontra-yes",
                )
            } else {
                assert_eq!(k.threshold, None);
                assert!(h.query_by_label(&i.t("guided-kontra-yes")).is_none());
                (
                    "guided-kontra-threshold-kontra-null",
                    "guided-kontra-applies-no",
                    "guided-kontra-applies-yes",
                )
            };
            let figures = [
                ("factor", 2.into()),
                ("threshold", k.threshold.unwrap_or_default().into()),
            ];
            assert!(h.query_by_label(&i.tf(prompt, &figures)).is_some());
            click(&mut h, &i.t(wrong));
            assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
            assert!(h.state().session().guided().unwrap().kontra_rule.is_none());
            click(&mut h, &i.t("guided-retry"));
            click(&mut h, &i.t(right));
            let g = h.state().session().guided().unwrap();
            assert_eq!(g.step, GuidedStep::Act);
            let rule = g.kontra_rule.clone().unwrap();
            assert!(!rule.wants && rule.null != grand);
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            let shown = coach::action(&i, &before, rule.action);
            assert!(
                h.query_by_label(&i.tf("guided-bid-rule", &[("action", shown.into())]))
                    .is_some()
            );
            let count = [
                ("count", rule.count.into()),
                ("threshold", rule.threshold.into()),
            ];
            if grand {
                assert!(rule.count <= 4);
                assert!(
                    h.query_by_label(&i.tf("guided-kontra-count-kontra-grand", &count))
                        .is_some()
                );
                assert!(
                    h.query_by_label(&i.t("guided-kontra-caveat-kontra-grand"))
                        .is_some()
                );
            } else {
                assert!(
                    h.query_by_label(&i.t("guided-kontra-count-kontra-null"))
                        .is_some()
                );
                assert!(
                    h.query_by_label(&i.t("guided-kontra-caveat-null"))
                        .is_some()
                );
                // No Trumpf count or strength comparison is presented in Null.
                assert!(
                    h.query_by_label(&i.tf("guided-kontra-count-kontra-no", &count))
                        .is_none()
                );
            }
            assert!(
                h.query_by_label(&i.t("guided-kontra-fact-factor"))
                    .is_some()
            );
            let card = rule.card.as_ref().unwrap().card;
            assert!(
                h.query_by_label(&i.tf(
                    "guided-kontra-card",
                    &[("card", card_name(&i, card).into())]
                ))
                .is_some()
            );
            if grand {
                click(&mut h, &i.t("button-kontra"));
                settle(&mut h);
                let g = h.state().session().guided().unwrap();
                assert_eq!(g.step, GuidedStep::Complete);
                assert!(matches!(
                    g.kontra_observation,
                    Some(GuidedKontraObservation::Reply { .. })
                ));
            } else {
                let played = before
                    .legal_actions
                    .iter()
                    .find_map(|a| match a {
                        Action::Play(c) => Some(*c),
                        _ => None,
                    })
                    .unwrap();
                click(&mut h, &card_name(&i, played));
                let g = h.state().session().guided().unwrap();
                assert!(matches!(
                    g.kontra_observation,
                    Some(GuidedKontraObservation::Declined { card: c, .. }) if c == played
                ));
            }
            assert!(h.query_by_label(&i.t("guided-kontra-complete")).is_some());
            assert_eq!(h.state().learning_history(), &history_before);
            click(&mut h, &i.t("guided-return"));
            assert!(h.state().session().guided().is_none());
        }
    }
}

fn grand_app(seed: u64) -> Harness<'static, GrandCoachApp> {
    let config = SessionConfig {
        seed,
        series: SeriesLength::Free,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    };
    Harness::builder()
        .with_size([1600.0, 1200.0])
        .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config))
}

/// Ordinary app, real widgets, Kontra/Re off (#157). Seed 19, second game:
/// the human (Hinterhand) passes and defends last against Mittelhand's
/// Grand; MH's C9 is ahead of VH's C7, CT/CA overtake, C8 does not, Easy
/// takes CT. Seed 19 later reaches an unsupported Grand decision. Seed 2,
/// first game: the human (Vorhand) declares Grand at most 30 after a pickup
/// and plays last against HH's HK: only HA overtakes HQ. Hand-derived.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_grand_win_last_widgets_both_roles_any_card_actual_trick_and_fallback_in_de_en() {
    use skat_app::{Action, Card, GameType, Rank, Seat, Suit};
    let play = |c: Card| Action::Play(c);
    for language in Language::ALL {
        let i = I18n::new(language);
        // Defender: wrong/retry, correct attempt with the non-Easy winner CA,
        // then the losing alternative C8 and the actual completed trick.
        let mut h = grand_app(19);
        enter_grand(
            &mut h,
            language,
            false,
            Some(skat_app::GuidedBranch::WinLast),
        );
        let before = h.state().session().view();
        let history_before = h.state().learning_history().clone();
        let (c8, ct, ca) = (
            Card::new(Suit::Kreuz, Rank::Acht),
            Card::new(Suit::Kreuz, Rank::Zehn),
            Card::new(Suit::Kreuz, Rank::Ass),
        );
        assert_eq!(before.contract.unwrap().game, GameType::Grand);
        assert_ne!(before.declarer, Some(skat_app::PlayerId::HUMAN));
        assert_eq!(before.players[0].seat, Seat::Hinterhand);
        assert_eq!(before.legal_actions, vec![play(c8), play(ct), play(ca)]);
        let consider = |c| i.tf("guided-consider", &[("card", card_name(&i, c).into())]);
        let rule_card = |c| i.tf("guided-rule-card", &[("card", card_name(&i, c).into())]);
        assert!(h.query_by_label(&consider(ct)).is_none());
        assert!(h.query_by_label(&rule_card(ct)).is_none());
        assert!(h.query_by_label(&i.t("guided-memory")).is_none());
        click(&mut h, &seat_name(&i, Seat::Hinterhand));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &seat_name(&i, Seat::Mittelhand));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Check
        );
        click(&mut h, &i.t("guided-none"));
        assert_eq!(
            h.state().session().guided().unwrap().task,
            Some(skat_app::GuidedBranch::WinLast)
        );
        assert!(h.query_by_label(&i.t("guided-win")).is_some());
        assert!(h.query_by_label(&rule_card(ct)).is_none());
        click(&mut h, &consider(c8));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &consider(ca));
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Act);
        assert_eq!(g.rule.unwrap().card, ct);
        assert!(h.query_by_label(&rule_card(ct)).is_some());
        assert!(h.query_by_label(&i.t("guided-rule-win")).is_some());
        let cards = format!("{}, {}", card_name(&i, ct), card_name(&i, ca));
        assert!(
            h.query_by_label(&i.tf("guided-candidates", &[("cards", cards.into())]))
                .is_some()
        );
        click(&mut h, &i.t("guided-memory"));
        let m = h.state().session().guided().unwrap().memory.unwrap();
        assert_eq!(m.skat, None);
        assert!(m.trumpf.is_some());
        click(&mut h, &i.t("guided-memory-hide"));
        assert_eq!(
            h.state().session().view().legal_actions,
            before.legal_actions
        );
        click(&mut h, &card_name(&i, c8));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        click(&mut h, &i.t("guided-void-yes"));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &i.t("guided-void-no"));
        let o = h.state().session().guided().unwrap().observation.unwrap();
        assert!(!o.proves_void);
        assert_eq!(o.winner, Some(Seat::Mittelhand));
        assert!(matches!(
            o.source,
            skat_app::coach_model::Source::Trick { index: 0, seat: Seat::Hinterhand, card, .. } if card == c8
        ));
        assert!(h.query_by_label(&i.t("guided-only-card")).is_some());
        let winner = i.tf(
            "guided-winner",
            &[("seat", seat_name(&i, Seat::Mittelhand).into())],
        );
        assert!(h.query_by_label(&winner).is_some());
        assert_eq!(h.state().learning_history(), &history_before);
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());

        // Same game, a later Grand decision outside WinLast: honest fallback.
        enter_grand(&mut h, language, false, None);
        assert!(h.query_by_label(&i.t("guided-unsupported")).is_some());
        assert!(h.state().session().guided().unwrap().rule.is_none());
        click(&mut h, &i.t("guided-return"));

        // Declarer: "I don't know" reveals Easy's HA; the learner plays HA.
        let mut h = grand_app(2);
        enter_grand(
            &mut h,
            language,
            true,
            Some(skat_app::GuidedBranch::WinLast),
        );
        let before = h.state().session().view();
        let (hq, ha) = (
            Card::new(Suit::Herz, Rank::Dame),
            Card::new(Suit::Herz, Rank::Ass),
        );
        assert_eq!(before.declarer, Some(skat_app::PlayerId::HUMAN));
        assert_eq!(before.players[0].seat, Seat::Vorhand);
        assert_eq!(before.legal_actions, vec![play(hq), play(ha)]);
        click(&mut h, &i.t("guided-unknown"));
        let rule = h.state().session().guided().unwrap().rule.unwrap();
        assert_eq!((rule.card, rule.candidates), (ha, vec![ha]));
        assert!(h.query_by_label(&rule_card(ha)).is_some());
        click(&mut h, &card_name(&i, ha));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        click(&mut h, &i.t("guided-reveal"));
        let o = h.state().session().guided().unwrap().observation.unwrap();
        assert_eq!(o.winner, Some(Seat::Vorhand));
        let winner = i.tf(
            "guided-winner",
            &[("seat", seat_name(&i, Seat::Vorhand).into())],
        );
        assert!(h.query_by_label(&winner).is_some());
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
    }
}

/// Ordinary app, real widgets, Kontra/Re off (#159). Seed 119, third game,
/// sixth trick (route recorded by skat-app's bounded seed search): the human
/// (Mittelhand) passes every bid and defends against Hinterhand's Grand.
/// Partner VH leads H8; H7 stays below, H9 overtakes; Easy H7. The learner
/// plays the non-Easy H9 and the automated declarer actually answers HQ,
/// which wins: staying under or not is a heuristic, never a guarantee.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn guided_grand_under_partner_widgets_wrong_retry_reveal_alternative_and_declarer_reply_in_de_en() {
    use skat_app::{Action, Card, GameType, GuidedBranch, PlayerId, Rank, Seat, Suit};
    for language in Language::ALL {
        let i = I18n::new(language);
        let mut h = grand_app(119);
        enter_grand(&mut h, language, false, Some(GuidedBranch::UnderPartner));
        let before = h.state().session().view();
        let history_before = h.state().learning_history().clone();
        let herz = |rank| Card::new(Suit::Herz, rank);
        let (h7, h8, h9, hq) = (
            herz(Rank::Sieben),
            herz(Rank::Acht),
            herz(Rank::Neun),
            herz(Rank::Dame),
        );
        assert_eq!(before.game_number, 3);
        assert_eq!(before.tricks_played, 5);
        assert_eq!(before.contract.unwrap().game, GameType::Grand);
        assert!(!before.contract.unwrap().ouvert);
        assert_eq!(before.players[0].seat, Seat::Mittelhand);
        let declarer = before.declarer.unwrap();
        assert_ne!(declarer, PlayerId::HUMAN);
        assert_eq!(declarer.seat(before.dealer), Seat::Hinterhand);
        assert_eq!(
            before
                .current_trick
                .iter()
                .map(|p| p.card)
                .collect::<Vec<_>>(),
            vec![h8]
        );
        assert_eq!(
            before.legal_actions,
            vec![Action::Play(h7), Action::Play(h9)]
        );
        let consider = |c| i.tf("guided-consider", &[("card", card_name(&i, c).into())]);
        let rule_card = |c| i.tf("guided-rule-card", &[("card", card_name(&i, c).into())]);
        // Answer and memory are withheld before consent.
        assert!(h.query_by_label(&consider(h7)).is_none());
        assert!(h.query_by_label(&rule_card(h7)).is_none());
        assert!(h.query_by_label(&i.t("guided-memory")).is_none());
        // Orientation: the declarer is not ahead; partner VH is.
        click(&mut h, &seat_name(&i, Seat::Hinterhand));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &seat_name(&i, Seat::Vorhand));
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Check
        );
        // Not last: the declarer still follows.
        click(&mut h, &i.t("guided-none"));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &seat_name(&i, Seat::Hinterhand));
        let g = h.state().session().guided().unwrap();
        assert_eq!(
            (g.step, g.task),
            (GuidedStep::Narrow, Some(GuidedBranch::UnderPartner))
        );
        assert!(g.rule.is_none());
        assert!(h.query_by_label(&i.t("guided-under")).is_some());
        assert!(h.query_by_label(&rule_card(h7)).is_none());
        // H9 overtakes the partner: wrong for this task.
        click(&mut h, &consider(h9));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        if language == Language::De {
            click(&mut h, &consider(h7));
        } else {
            click(&mut h, &i.t("guided-unknown"));
        }
        let g = h.state().session().guided().unwrap();
        assert_eq!(g.step, GuidedStep::Act);
        let rule = g.rule.unwrap();
        assert_eq!(
            (rule.card, rule.branch, rule.candidates.clone()),
            (h7, GuidedBranch::UnderPartner, vec![h7])
        );
        assert!(h.query_by_label(&rule_card(h7)).is_some());
        assert!(h.query_by_label(&i.t("guided-rule-under")).is_some());
        let cards = card_name(&i, h7);
        assert!(
            h.query_by_label(&i.tf("guided-candidates", &[("cards", cards.into())]))
                .is_some()
        );
        click(&mut h, &i.t("guided-memory"));
        let m = h.state().session().guided().unwrap().memory.unwrap();
        assert_eq!(m.skat, None);
        // All four Buben are already played in tricks 2 and 3.
        assert_eq!(m.trumpf.unwrap().outside, 0);
        click(&mut h, &i.t("guided-memory-hide"));
        assert_eq!(
            h.state().session().view().legal_actions,
            before.legal_actions
        );
        // A legal non-Easy alternative: H9 overtakes the partner's H8.
        click(&mut h, &card_name(&i, h9));
        settle(&mut h);
        assert_eq!(
            h.state().session().guided().unwrap().step,
            GuidedStep::Observe
        );
        click(&mut h, &i.t("guided-void-yes"));
        assert!(h.query_by_label(&i.t("guided-wrong")).is_some());
        click(&mut h, &i.t("guided-retry"));
        click(&mut h, &i.t("guided-void-no"));
        let o = h.state().session().guided().unwrap().observation.unwrap();
        assert!(!o.proves_void);
        // The declarer's actual reply HQ wins the completed trick.
        assert_eq!(
            o.source,
            skat_app::coach_model::Source::Trick {
                index: 5,
                seat: Seat::Hinterhand,
                led: h8,
                card: hq
            }
        );
        assert_eq!(o.winner, Some(Seat::Hinterhand));
        let last = h.state().session().view().last_trick.unwrap();
        assert_eq!(
            last.cards.iter().map(|p| p.card).collect::<Vec<_>>(),
            vec![h8, h9, hq]
        );
        assert!(h.query_by_label(&i.t("guided-only-card")).is_some());
        let winner = i.tf(
            "guided-winner",
            &[("seat", seat_name(&i, Seat::Hinterhand).into())],
        );
        assert!(h.query_by_label(&winner).is_some());
        assert_eq!(h.state().learning_history(), &history_before);
        click(&mut h, &i.t("guided-return"));
        assert!(h.state().session().guided().is_none());
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn grand_feed_ordinary_bilingual_widgets_chosen_alternative_retry_and_free_return() {
    use skat_app::*;
    for language in Language::ALL {
        for narrow in [false, true] {
            for chosen in [11, 8] {
                let mut h = Harness::builder()
                    .with_size(if narrow {
                        [960.0, 640.0]
                    } else {
                        [1440.0, 1000.0]
                    })
                    .build_eframe(|cc| {
                        GrandCoachApp::new(
                            &cc.egui_ctx,
                            SessionConfig {
                                seed: 15,
                                series: SeriesLength::Fixed(3),
                                opponents: Opponents::Levels([Level::Easy; 2]),
                                ..SessionConfig::default()
                            },
                        )
                    });
                feed_flow(&mut h, language, 4, chosen);
            }
            let mut h = Harness::builder()
                .with_size([1440.0, 1000.0])
                .build_eframe(|cc| {
                    GrandCoachApp::new(
                        &cc.egui_ctx,
                        SessionConfig {
                            seed: 15,
                            series: SeriesLength::Fixed(3),
                            opponents: Opponents::Levels([Level::Easy; 2]),
                            ..SessionConfig::default()
                        },
                    )
                });
            enter_feed(&mut h, language);
            let i = I18n::new(language);
            let before = h.state().session().view();
            click(&mut h, &seat_name(&i, Seat::Mittelhand));
            assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
            h.get_by_label(&i.tf(
                "guided-source",
                &[
                    ("trick", 3.into()),
                    ("seat", seat_name(&i, Seat::Hinterhand).into()),
                    ("card", card_name(&i, Card::from_index(12).unwrap()).into()),
                ],
            ));
            click(&mut h, &i.t("guided-retry"));
            assert_eq!(h.state().session().guided().unwrap().correct, None);
            click(&mut h, &i.t("guided-return"));
            assert_eq!(
                h.state().session().view().legal_actions,
                before.legal_actions
            );
            click(&mut h, &card_name(&i, Card::from_index(8).unwrap()));
        }
    }
}
