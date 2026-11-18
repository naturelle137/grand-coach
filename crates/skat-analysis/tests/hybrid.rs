// SPDX-License-Identifier: EUPL-1.2

//! Hybrid proposals, provenance, permitted views and unchanged Classic fallbacks.

mod common;

use common::*;
use skat_analysis::hybrid::{HybridAnalysis, HybridFallback, analyze_hybrid};
use skat_analysis::{Budget, Cancel, Status, analyze, with_max_threads};
use skat_core::{Action, Announcement, Deal, Game, GameType, Phase, PlayerView, RuleConfig, Seat};
use skat_nn::{ModelError, NeuralModel, Phases, fixture_model, sha256};

const BUDGET: Budget = Budget {
    samples: 3,
    nodes_per_task: 100_000,
};

fn standard() -> Deal {
    deal(
        "CJ SJ HJ DJ CA CT CK CQ C9 C8",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 DA DT DK DQ D9 D8",
        "C7 D7",
    )
}

fn game(deal: Deal) -> Game {
    let mut g = Game::from_deal(deal, RuleConfig { kontra_re: false });
    for a in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Grand)),
        Action::Play(c("CJ")),
    ] {
        g.apply(a).unwrap();
    }
    g
}

fn model() -> NeuralModel {
    let bytes = fixture_model(11, Phases::SPIEL);
    NeuralModel::load(&bytes, &sha256(&bytes)).unwrap()
}

#[test]
fn proposals_are_verified_but_other_cards_are_not_ranked() {
    let mut g = game(standard());
    // Five tricks of public history keep several options but make a
    // successful verification cheap; early-game exhaustion is tested below.
    for card in [
        "SA", "HQ", "SJ", "ST", "H9", "HJ", "SK", "H8", "DJ", "SQ", "H7", "CA", "S9", "DA", "CT",
    ] {
        g.apply(Action::Play(c(card))).unwrap();
    }
    let view = g.view(Seat::Mittelhand);
    let model = model();
    let result = analyze_hybrid(&view, Ok(&model), BUDGET, 7, None).unwrap();
    let HybridAnalysis::LearnedEstimate(ref learned) = result else {
        panic!("screening expected")
    };
    assert_eq!(learned.model_sha256, model.sha256());
    assert_eq!(learned.encoder_version, model.header().encoder_version);
    assert_eq!(
        learned.options.iter().map(|o| o.action).collect::<Vec<_>>(),
        view.legal_actions
    );
    assert!((2..=3).contains(&learned.options.iter().filter(|o| o.proposed).count()));
    assert!(learned.options.iter().any(|o| !o.proposed));
    let evaluation = model.evaluate(&view).unwrap();
    assert_eq!(learned.value, evaluation.value);
    assert_eq!(learned.search_status, Status::Complete);
    assert_eq!(learned.effort.finished, BUDGET.samples);
    assert!(result.recommendation(&view).is_some());
    for (o, (_, p)) in learned.options.iter().zip(evaluation.policy) {
        assert_eq!(o.prior, p);
        if !o.proposed {
            assert!(o.verification.is_none());
        }
    }
    assert!(
        learned
            .options
            .iter()
            .any(|o| o.proposed && o.action == skat_analysis::baseline::play(&view))
    );
    assert!(
        learned
            .options
            .iter()
            .filter_map(|o| o.verification)
            .all(|e| !e.exact)
    );
    if let Some(action) = result.recommendation(&view) {
        assert!(
            learned
                .options
                .iter()
                .any(|o| o.action == action && o.verification.is_some())
        );
    }
    // Reuse exactly the ordinary solver on the same proposals, budget and worlds.
    let restricted = PlayerView {
        legal_actions: learned
            .options
            .iter()
            .filter(|o| o.proposed)
            .map(|o| o.action)
            .collect(),
        ..view.clone()
    };
    let verified = analyze(&restricted, BUDGET, 7, None).unwrap();
    assert_eq!(learned.search_status, verified.status);
    assert_eq!(learned.effort, verified.effort);
    for o in verified.options {
        assert_eq!(
            learned
                .options
                .iter()
                .find(|p| p.action == o.action)
                .unwrap()
                .verification,
            o.estimate
        );
    }
    assert_eq!(
        result,
        analyze_hybrid(&view, Ok(&model), BUDGET, 7, None).unwrap()
    );
    for n in [1, 2] {
        assert_eq!(
            result,
            with_max_threads(std::num::NonZeroUsize::new(n).unwrap(), || analyze_hybrid(
                &view,
                Ok(&model),
                BUDGET,
                7,
                None
            )
            .unwrap())
        );
    }
}

#[test]
fn hidden_deal_substitution_and_public_history_stay_permitted() {
    let other = deal(
        "CJ SJ HJ DJ CA CT CK CQ DA DT",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 C9 C8 DK DQ D9 D8",
        "C7 D7",
    );
    let mut a = game(standard());
    let mut b = game(other);
    let model = model();
    let mut histories = 0;
    // Same public history and own hand, genuinely different concealed hands.
    assert_ne!(a.omniscient_view().hands, b.omniscient_view().hands);
    for card in ["SA", "HQ", "SJ", "ST", "H9", "HJ"] {
        let seat = a.current_seat().unwrap();
        let av = a.view(seat);
        let bv = b.view(seat);
        if av == bv {
            assert_eq!(
                analyze_hybrid(&av, Ok(&model), BUDGET, 9, None),
                analyze_hybrid(&bv, Ok(&model), BUDGET, 9, None)
            );
            histories += 1;
        }
        let action = Action::Play(c(card));
        a.apply(action).unwrap();
        b.apply(action).unwrap();
    }
    assert!(histories >= 2);
}

#[test]
fn exact_few_option_and_missing_model_paths_preserve_classic() {
    let mut g = game(standard());
    let model = model();
    let view = g.view(Seat::Mittelhand);
    let missing = analyze_hybrid(&view, Err(&ModelError::Missing), BUDGET, 8, None).unwrap();
    assert_eq!(
        missing,
        HybridAnalysis::Classic {
            analysis: Box::new(analyze(&view, BUDGET, 8, None).unwrap()),
            reason: HybridFallback::Model(skat_nn::Fallback::Model(ModelError::Missing))
        }
    );
    while g.phase() == Phase::Spiel && g.tricks().len() < 8 {
        let v = g.view(g.current_seat().unwrap());
        g.apply(v.legal_actions[0]).unwrap();
    }
    let view = g.view(g.current_seat().unwrap());
    let budget = Budget {
        samples: 1000,
        nodes_per_task: 1_000_000,
    };
    let result = analyze_hybrid(&view, Ok(&model), budget, 2, None).unwrap();
    let HybridAnalysis::Classic { analysis, .. } = result else {
        panic!("late game must stay classic")
    };
    assert_eq!(*analysis, analyze(&view, budget, 2, None).unwrap());
    assert!(analysis.options.iter().all(|o| o.estimate.unwrap().exact));
}

#[test]
fn cancellation_and_exhaustion_never_invent_verified_results() {
    let view = game(standard()).view(Seat::Mittelhand);
    let model = model();
    let cancel = Cancel::new();
    cancel.cancel();
    let result = analyze_hybrid(&view, Ok(&model), BUDGET, 1, Some(&cancel)).unwrap();
    assert!(result.recommendation(&view).is_none());
    let HybridAnalysis::Classic { analysis, reason } = result else {
        panic!("cancel before inference")
    };
    assert_eq!(reason, HybridFallback::Cancelled);
    assert_eq!(analysis.status, Status::Cancelled);
    let result = analyze_hybrid(
        &view,
        Ok(&model),
        Budget {
            nodes_per_task: 0,
            ..BUDGET
        },
        1,
        None,
    )
    .unwrap();
    let HybridAnalysis::LearnedEstimate(ref learned) = result else {
        panic!("proposal expected")
    };
    assert_eq!(learned.search_status, Status::BudgetExhausted);
    assert!(learned.options.iter().all(|o| o.verification.is_none()));
    assert!(result.recommendation(&view).is_none());
}

#[test]
fn unsupported_model_and_malformed_history_fall_back_or_fail_explicitly() {
    let view = game(standard()).view(Seat::Mittelhand);
    let bytes = fixture_model(12, Phases::from_bits(1).unwrap());
    let unsupported = NeuralModel::load(&bytes, &sha256(&bytes)).unwrap();
    let result = analyze_hybrid(&view, Ok(&unsupported), BUDGET, 1, None).unwrap();
    assert!(matches!(
        result,
        HybridAnalysis::Classic {
            reason: HybridFallback::Model(skat_nn::Fallback::UnsupportedPhase(Phase::Spiel)),
            ..
        }
    ));
    let mut malformed = view.clone();
    malformed.contract = None;
    let model = model();
    assert_eq!(
        analyze_hybrid(&malformed, Ok(&model), BUDGET, 1, None),
        Err(vec![skat_analysis::sampling::Diagnostic::InvalidHistory])
    );
    let preplay = Game::new(1, RuleConfig::default());
    let view = preplay.view(preplay.current_seat().unwrap());
    let HybridAnalysis::Classic { analysis, reason } = analyze_hybrid(
        &view,
        Ok(&model),
        Budget {
            nodes_per_task: 0,
            ..BUDGET
        },
        1,
        None,
    )
    .unwrap() else {
        panic!("preplay stays classic")
    };
    assert_eq!(reason, HybridFallback::NotCardPlay);
    assert_eq!(
        *analysis,
        analyze(
            &view,
            Budget {
                nodes_per_task: 0,
                ..BUDGET
            },
            1,
            None
        )
        .unwrap()
    );
}

#[test]
#[cfg(feature = "serde")]
fn learned_provenance_round_trips_as_a_distinct_wire_type() {
    let view = game(standard()).view(Seat::Mittelhand);
    let model = model();
    let result = analyze_hybrid(&view, Ok(&model), BUDGET, 7, None).unwrap();
    let wire = serde_json::to_string(&result).unwrap();
    assert!(wire.contains("LearnedEstimate"));
    assert_eq!(result, serde_json::from_str(&wire).unwrap());
    assert!(serde_json::from_str::<skat_analysis::Analysis>(&wire).is_err());
}

#[test]
fn few_options_and_nonacting_seats_use_all_option_classic() {
    let model = model();
    let mut g = game(standard());
    let budget = Budget {
        samples: 1,
        nodes_per_task: 0,
    };
    let mut checked = false;
    while let Some(seat) = g.current_seat() {
        let view = g.view(seat);
        if view.legal_actions.len() <= 3
            && skat_analysis::sampling::HiddenCards::from_view(&view)
                .unwrap()
                .count()
                > 1
        {
            let result = analyze_hybrid(&view, Ok(&model), budget, 3, None).unwrap();
            assert_eq!(
                result,
                HybridAnalysis::Classic {
                    analysis: Box::new(analyze(&view, budget, 3, None).unwrap()),
                    reason: HybridFallback::FewOptions
                }
            );
            checked = true;
            break;
        }
        g.apply(view.legal_actions[0]).unwrap();
    }
    assert!(checked);
    let inactive = g.view(Seat::from_index(g.current_seat().unwrap().index() + 1));
    let HybridAnalysis::Classic { analysis, reason } =
        analyze_hybrid(&inactive, Ok(&model), budget, 3, None).unwrap()
    else {
        panic!("not a decision")
    };
    assert_eq!(reason, HybridFallback::NotCardPlay);
    assert_eq!(analysis.status, Status::NoDecision);
    assert!(analysis.options.is_empty());
}
