// SPDX-License-Identifier: EUPL-1.2

//! Opt-in hybrid adapter: legal games, Classic fallback and cancellation.

use std::sync::Arc;

use skat_ai::{BaselineOpponent, Budget, Cancel, HybridOpponent, Level, Opponent, Player};
use skat_analysis::hybrid::HybridAnalysis;
use skat_core::{Action, Announcement, Game, GameType, Phase, RuleConfig};
use skat_nn::{ModelError, NeuralModel, Phases, fixture_model, sha256};

fn opponent(missing: bool) -> HybridOpponent {
    let bytes = fixture_model(11, Phases::SPIEL);
    let model = if missing {
        Err(ModelError::Missing)
    } else {
        NeuralModel::load(&bytes, &sha256(&bytes)).map(Arc::new)
    };
    let mut o = HybridOpponent::new(model, 42);
    o.budget = Budget {
        samples: 2,
        nodes_per_task: 5_000,
    };
    o
}

#[test]
fn hybrid_plays_legal_complete_games_and_reports_verified_choices() {
    let o = opponent(false);
    let mut learned = 0;
    let mut classic = 0;
    for deal in 0..3 {
        let mut g = Game::new(deal, RuleConfig { kontra_re: true });
        while let Some(seat) = g.current_seat() {
            let view = g.view(seat);
            let d = o.decide_explained(&view, None).unwrap();
            assert!(view.legal_actions.contains(&d.decision.action));
            assert_eq!(Some(d.decision), o.decide(&view, None));
            if let Some(HybridAnalysis::LearnedEstimate(p)) = &d.analysis {
                learned += 1;
                assert_eq!(d.decision.status, None);
                if !d.decision.fallback {
                    assert!(p.options.iter().any(|v| v.action == d.decision.action
                        && v.proposed
                        && v.verification.is_some()));
                } else {
                    assert_eq!(Some(d.decision.action), BaselineOpponent.decide(&view));
                }
            } else {
                classic += 1;
            }
            g.apply(d.decision.action).unwrap();
        }
        assert!(matches!(g.phase(), Phase::Beendet | Phase::Eingepasst));
    }
    assert!(learned > 0 && classic > 0);
}

#[test]
fn missing_models_preserve_classic_decisions_and_cancel_returns_legal_rules() {
    let o = opponent(true);
    let mut classic = Opponent::new(Level::Strong, o.seed);
    classic.profile.budget = Some(o.budget);
    let mut g = Game::new(0, RuleConfig { kontra_re: true });
    while let Some(seat) = g.current_seat() {
        let view = g.view(seat);
        let decision = o.decide(&view, None).unwrap();
        assert_eq!(Some(decision), classic.decide(&view, None));
        g.apply(decision.action).unwrap();
    }
    let mut g = Game::new(7, RuleConfig { kontra_re: false });
    while g.phase() != Phase::Spiel {
        let v = g.view(g.current_seat().unwrap());
        let a = match v.phase {
            Phase::Reizen if v.bid == 0 => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
            _ => v.legal_actions[0],
        };
        g.apply(a).unwrap();
    }
    let view = g.view(g.current_seat().unwrap());
    let cancel = Cancel::new();
    cancel.cancel();
    assert_eq!(
        o.decide(&view, Some(&cancel)),
        classic.decide(&view, Some(&cancel))
    );
    for seat in skat_core::Seat::ALL {
        if Some(seat) != g.current_seat() {
            assert!(o.decide(&g.view(seat), None).is_none());
        }
    }
}

#[test]
fn nonfinite_inference_has_a_named_classic_fallback() {
    use burn::tensor::Device;
    use skat_analysis::hybrid::{HybridFallback, analyze_hybrid};
    use skat_nn::{ENCODER_VERSION, Fallback, ModelHeader, NetShape, PolicyValueNet, write_model};
    let shape = NetShape::current(8);
    let net = PolicyValueNet::from_weights(shape, &Device::flex(), || f32::NAN);
    let bytes = write_model(
        &ModelHeader {
            encoder_version: ENCODER_VERSION,
            shape,
            phases: Phases::SPIEL,
        },
        net,
    )
    .unwrap();
    let model = NeuralModel::load(&bytes, &sha256(&bytes)).unwrap();
    let mut g = Game::new(7, RuleConfig { kontra_re: false });
    while let Some(seat) = g.current_seat() {
        let view = g.view(seat);
        if view.phase == Phase::Spiel && view.legal_actions.len() > 3 {
            let result = analyze_hybrid(
                &view,
                Ok(&model),
                Budget {
                    samples: 1,
                    nodes_per_task: 0,
                },
                1,
                None,
            )
            .unwrap();
            assert!(matches!(
                result,
                HybridAnalysis::Classic {
                    reason: HybridFallback::Model(Fallback::NonFinite),
                    ..
                }
            ));
            return;
        }
        let action = match view.phase {
            Phase::Reizen if view.bid == 0 => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
            _ => view.legal_actions[0],
        };
        g.apply(action).unwrap();
    }
    panic!("a multi-card decision must exercise inference");
}

#[test]
fn equal_policy_priors_propose_first_two_and_the_shared_rule() {
    use burn::tensor::Device;
    use skat_analysis::hybrid::analyze_hybrid;
    use skat_nn::{ENCODER_VERSION, ModelHeader, NetShape, PolicyValueNet, write_model};
    let shape = NetShape::current(8);
    let net = PolicyValueNet::from_weights(shape, &Device::flex(), || 0.0);
    let bytes = write_model(
        &ModelHeader {
            encoder_version: ENCODER_VERSION,
            shape,
            phases: Phases::SPIEL,
        },
        net,
    )
    .unwrap();
    let model = NeuralModel::load(&bytes, &sha256(&bytes)).unwrap();
    let mut g = Game::new(7, RuleConfig { kontra_re: false });
    while let Some(seat) = g.current_seat() {
        let view = g.view(seat);
        if view.phase == Phase::Spiel && view.legal_actions.len() > 3 {
            let HybridAnalysis::LearnedEstimate(p) = analyze_hybrid(
                &view,
                Ok(&model),
                Budget {
                    samples: 1,
                    nodes_per_task: 0,
                },
                1,
                None,
            )
            .unwrap() else {
                panic!("learned proposals")
            };
            let rule = skat_analysis::baseline::play(&view);
            assert!(p.options.iter().all(|o| o.prior == p.options[0].prior));
            for (i, o) in p.options.iter().enumerate() {
                assert_eq!(o.proposed, i < 2 || o.action == rule);
            }
            return;
        }
        let action = match view.phase {
            Phase::Reizen if view.bid == 0 => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
            _ => view.legal_actions[0],
        };
        g.apply(action).unwrap();
    }
    panic!("a multi-card decision must exercise inference");
}

#[test]
fn easy_hybrid_uses_model_and_missing_model_uses_easy_rules() {
    let mut g = Game::new(7, RuleConfig { kontra_re: false });
    while g.phase() != Phase::Spiel {
        let v = g.view(g.current_seat().unwrap());
        let a = match v.phase {
            Phase::Reizen if v.bid == 0 => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
            _ => v.legal_actions[0],
        };
        g.apply(a).unwrap();
    }
    let view = g.view(g.current_seat().unwrap());
    let mut hybrid = opponent(false);
    hybrid.level = Level::Easy;
    hybrid.budget.samples = 1;
    assert!(matches!(
        hybrid.decide_explained(&view, None).unwrap().analysis,
        Some(HybridAnalysis::LearnedEstimate(_))
    ));
    let mut missing = opponent(true);
    missing.level = Level::Easy;
    assert_eq!(
        missing.decide(&view, None),
        Opponent::new(Level::Easy, missing.seed).decide(&view, None)
    );
}
