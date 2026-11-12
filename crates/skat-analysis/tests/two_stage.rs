// SPDX-License-Identifier: EUPL-1.2

//! Opt-in two-stage card-play analysis (#104): behaviour against the
//! single-stage reference. Thread determinism, permitted-view hidden-deal
//! substitution, per-option sample counts, effective samples and status,
//! cancellation and budget exhaustion, and preservation of exact and
//! excluded decisions. No timing or strength claims.

mod common;

use std::num::NonZeroUsize;

use common::*;
use skat_analysis::{
    Analysis, Budget, Cancel, Kernel, SkipReason, StageOutcome, Status, TwoStage, TwoStageAnalysis,
    agreement, analyze, analyze_experimental, analyze_two_stage, with_max_threads,
};
use skat_core::{Action, Announcement, Deal, Game, GameType, Phase, PlayerView, RuleConfig, Seat};

/// More samples than the proposal screens, so stage 2 has work.
const BUDGET: Budget = Budget {
    samples: 12,
    nodes_per_task: 50_000_000,
};

/// MH 18, VH holds, MH and HH pass: VH declarer at 18.
const VH_AT_18: [Action; 4] = [Action::Bid(18), Action::Hold, Action::Pass, Action::Pass];

fn standard_deal() -> Deal {
    deal(
        "CJ SJ HJ DJ CA CT CK CQ C9 C8",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 DA DT DK DQ D9 D8",
        "C7 D7",
    )
}

fn run(g: &mut Game, actions: &[Action]) {
    for &a in actions {
        g.apply(a).unwrap_or_else(|e| panic!("{a:?}: {e:?}"));
    }
}

fn current_view(g: &Game) -> PlayerView {
    g.view(g.current_seat().expect("a decision is pending"))
}

/// VH plays Grand Hand on `deal`; `plays` cards follow.
fn grand_hand(deal: Deal, kontra_re: bool, plays: &[&str]) -> Game {
    let mut g = Game::from_deal(deal, RuleConfig { kontra_re });
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    for card in plays {
        run(&mut g, &[Action::Play(c(card))]);
    }
    g
}

/// Card-play decisions of seeded games played by the first legal action
/// (bid 18 / pass, Grand), at `tricks` completed tricks, with at least
/// `min_options` legal cards.
fn positions(tricks: usize, min_options: usize, count: usize) -> Vec<PlayerView> {
    let mut out = Vec::new();
    for seed in 0..200 {
        let mut g = Game::new(seed, RuleConfig::default());
        while g.phase() != Phase::Spiel || g.tricks().len() < tricks {
            let Some(seat) = g.current_seat() else { break };
            let view = g.view(seat);
            let action = match view.phase {
                Phase::Reizen if view.bid == 0 => Action::Bid(18),
                Phase::Reizen => Action::Pass,
                Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
                _ => view.legal_actions[0],
            };
            g.apply(action).unwrap();
        }
        let Some(seat) = g.current_seat() else {
            continue;
        };
        let view = g.view(seat);
        if view.phase == Phase::Spiel && view.legal_actions.len() >= min_options {
            out.push(view);
            if out.len() == count {
                break;
            }
        }
    }
    out
}

/// The first card-play decision of the seeded first-legal-action games
/// (as in [`positions`]) that satisfies `wanted`.
fn any_position(wanted: impl Fn(&PlayerView) -> bool) -> Option<PlayerView> {
    for seed in 0..200 {
        let mut g = Game::new(seed, RuleConfig::default());
        while let Some(seat) = g.current_seat() {
            let view = g.view(seat);
            if view.phase == Phase::Spiel && wanted(&view) {
                return Some(view);
            }
            let action = match view.phase {
                Phase::Reizen if view.bid == 0 => Action::Bid(18),
                Phase::Reizen => Action::Pass,
                Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
                _ => view.legal_actions[0],
            };
            g.apply(action).unwrap();
        }
    }
    None
}

fn two_stage(view: &PlayerView, budget: Budget, seed: u64) -> TwoStageAnalysis {
    analyze_two_stage(
        view,
        budget,
        seed,
        None,
        Kernel::Classic,
        TwoStage::PROPOSAL,
    )
    .expect("consistent view")
}

/// The report covers every option; survivors are ascending and match the
/// `kept` flags.
fn assert_report_shape(view: &PlayerView, t: &TwoStageAnalysis) {
    let r = &t.report;
    assert_eq!(r.options.len(), view.legal_actions.len());
    assert!(r.survivors.windows(2).all(|p| p[0] < p[1]), "{r:?}");
    for (i, o) in r.options.iter().enumerate() {
        assert_eq!(o.kept, r.survivors.contains(&i), "{r:?}");
    }
    let actions: Vec<Action> = t.analysis.options.iter().map(|o| o.action).collect();
    assert_eq!(actions, view.legal_actions);
}

/// Skipped screening: the analysis is the single-stage reference.
fn assert_skipped_like_reference(
    view: &PlayerView,
    budget: Budget,
    seed: u64,
    reason: SkipReason,
) -> Analysis {
    let t = two_stage(view, budget, seed);
    assert_report_shape(view, &t);
    assert_eq!(t.report.outcome, StageOutcome::Skipped(reason));
    assert_eq!(t.report.survivors.len(), view.legal_actions.len());
    let reference = analyze(view, budget, seed, None).unwrap();
    assert_eq!(t.analysis, reference);
    reference
}

#[test]
fn survivors_keep_the_reference_values_and_dropped_options_report_fewer_samples() {
    let mut screened = 0;
    for (k, view) in positions(1, 4, 6).iter().enumerate() {
        let seed = 20 + k as u64;
        let t = two_stage(view, BUDGET, seed);
        assert_report_shape(view, &t);
        assert_eq!(t.analysis.status, Status::Complete);
        let reference = analyze(view, BUDGET, seed, None).unwrap();
        // The first 8 sampled worlds of the reference, in the same order.
        let prefix = analyze(
            view,
            Budget {
                samples: TwoStage::PROPOSAL.screen_worlds,
                ..BUDGET
            },
            seed,
            None,
        )
        .unwrap();
        assert!(!reference.effort.enumerated && !prefix.effort.enumerated);
        assert_eq!(t.analysis.effort.planned, BUDGET.samples);
        assert_eq!(t.analysis.effort.finished, BUDGET.samples);
        match t.report.outcome {
            StageOutcome::Screened => {
                screened += 1;
                assert_eq!(t.report.screen_worlds, TwoStage::PROPOSAL.screen_worlds);
                let n = t.report.survivors.len();
                assert!((2..=4).contains(&n), "{:?}", t.report);
                assert!(n < view.legal_actions.len());
                assert!(t.report.screen_nodes < t.analysis.effort.nodes);
            }
            StageOutcome::AllSurvived => assert_eq!(t.analysis, reference),
            other => panic!("unexpected {other:?}"),
        }
        for (i, (o, stage)) in t.analysis.options.iter().zip(&t.report.options).enumerate() {
            let e = o.estimate.expect("every option has a stage-1 estimate");
            assert!(!e.exact);
            if stage.kept {
                // Identical worlds and order: bitwise the reference value.
                assert_eq!(o.estimate, reference.options[i].estimate, "option {i}");
                assert_eq!(e.samples, BUDGET.samples);
                assert_eq!(stage.effective_samples, t.analysis.effort.effective_samples);
            } else {
                assert_eq!(o.estimate, prefix.options[i].estimate, "option {i}");
                assert_eq!(e.samples, TwoStage::PROPOSAL.screen_worlds);
                assert_eq!(stage.effective_samples, prefix.effort.effective_samples);
                assert!(stage.effective_samples > 0.0);
                assert!(stage.effective_samples <= t.analysis.effort.effective_samples);
            }
        }
        // Paired measurement: regret is valued by the reference.
        let a = agreement(view, &reference, &t.analysis);
        assert!(a.reference.is_some() && a.candidate.is_some());
        let regret = a.regret.expect("reference values every option");
        assert!(regret >= 0.0);
        assert_eq!(a.identical(), regret == 0.0 && a.reference == a.candidate);
        assert!(t.report.survivors.contains(&a.candidate.unwrap()));
        assert_eq!(agreement(view, &reference, &reference).regret, Some(0.0));
    }
    assert!(screened > 0, "no position exercised stage 2");
}

#[test]
fn experimental_kernel_survivors_match_the_experimental_reference() {
    let view = &positions(1, 4, 1)[0];
    let t = analyze_two_stage(
        view,
        BUDGET,
        5,
        None,
        Kernel::Experimental,
        TwoStage::PROPOSAL,
    )
    .unwrap();
    let reference = analyze_experimental(view, BUDGET, 5, None, None).unwrap();
    for &i in &t.report.survivors {
        assert_eq!(
            t.analysis.options[i].estimate,
            reference.options[i].estimate
        );
    }
}

#[test]
fn same_inputs_same_result_for_any_thread_count() {
    let view = &positions(1, 4, 1)[0];
    let a = two_stage(view, BUDGET, 11);
    assert_eq!(a, two_stage(view, BUDGET, 11));
    for threads in [1, 2, 3] {
        let n = NonZeroUsize::new(threads).unwrap();
        assert_eq!(a, with_max_threads(n, || two_stage(view, BUDGET, 11)));
    }
    assert_eq!(a.analysis.effort.seed, 11);
}

#[test]
fn concealed_cards_do_not_change_the_two_stage_analysis() {
    // MH's cards and the public history agree; VH, HH and the Skat differ.
    // MH holds no trump, so every card is a legal answer to VH's CJ.
    let other = deal(
        "CJ SJ HJ DJ CA CT CK CQ DA DT",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 C9 C8 DK DQ D9 D8",
        "C7 D7",
    );
    let views: Vec<PlayerView> = [standard_deal(), other]
        .into_iter()
        .map(|d| grand_hand(d, false, &["CJ"]).view(Seat::Mittelhand))
        .collect();
    assert_eq!(views[0], views[1]);
    assert_eq!(views[0].legal_actions.len(), 10);
    let cheap = TwoStage {
        screen_worlds: 3,
        ..TwoStage::PROPOSAL
    };
    let budget = Budget {
        samples: 5,
        ..BUDGET
    };
    let run =
        |v: &PlayerView| analyze_two_stage(v, budget, 12, None, Kernel::Classic, cheap).unwrap();
    let (a, b) = (run(&views[0]), run(&views[1]));
    assert_eq!(a, b);
    assert_ne!(a.report.outcome, StageOutcome::Skipped(SkipReason::Exact));
}

#[test]
fn cancellation_and_budget_exhaustion_fall_back_to_the_reference() {
    let view = &positions(1, 4, 1)[0];
    let cancel = Cancel::new();
    cancel.cancel();
    let t = analyze_two_stage(
        view,
        BUDGET,
        1,
        Some(&cancel),
        Kernel::Classic,
        TwoStage::PROPOSAL,
    )
    .unwrap();
    assert_report_shape(view, &t);
    assert_eq!(t.analysis.status, Status::Cancelled);
    assert_eq!(
        t.report.outcome,
        StageOutcome::Skipped(SkipReason::Interrupted)
    );
    assert_eq!(t.analysis.effort.finished, 0);
    assert!(t.analysis.options.iter().all(|o| o.estimate.is_none()));
    assert!(t.report.options.iter().all(|o| o.effective_samples == 0.0));
    assert_eq!(t.analysis, analyze(view, BUDGET, 1, Some(&cancel)).unwrap());

    let tiny = Budget {
        nodes_per_task: 10,
        ..BUDGET
    };
    let t = two_stage(view, tiny, 1);
    assert_eq!(t.analysis.status, Status::BudgetExhausted);
    assert_eq!(
        t.report.outcome,
        StageOutcome::Skipped(SkipReason::Interrupted)
    );
    assert_eq!(t.analysis, analyze(view, tiny, 1, None).unwrap());
}

#[test]
fn excluded_and_exact_decisions_equal_the_reference() {
    // Exact: three tricks left, every distribution enumerated.
    let mut g = grand_hand(standard_deal(), false, &[]);
    for _ in 0..21 {
        let first = current_view(&g).legal_actions[0];
        g.apply(first).unwrap();
    }
    let view = current_view(&g);
    let budget = Budget {
        samples: 1000,
        ..BUDGET
    };
    let exact = assert_skipped_like_reference(&view, budget, 8, SkipReason::Exact);
    assert!(exact.options.iter().all(|o| o.estimate.unwrap().exact));

    // Kontra offered in card play: excluded.
    let view = current_view(&grand_hand(standard_deal(), true, &["CJ"]));
    assert!(view.legal_actions.contains(&Action::Kontra));
    let small = Budget {
        samples: 4,
        ..BUDGET
    };
    // Few samples would also skip; the Kontra check comes first.
    assert_skipped_like_reference(&view, small, 6, SkipReason::KontraRe);

    // Before card play (Hand or pickup): not card play.
    let mut g = Game::from_deal(standard_deal(), RuleConfig::default());
    run(&mut g, &VH_AT_18);
    assert_skipped_like_reference(&current_view(&g), small, 3, SkipReason::NotCardPlay);

    // Not this seat's turn.
    let t = two_stage(&g.view(Seat::Hinterhand), small, 3);
    assert_eq!(t.analysis.status, Status::NoDecision);
    assert_eq!(
        t.report.outcome,
        StageOutcome::Skipped(SkipReason::NoDecision)
    );
}

#[test]
fn small_budgets_and_few_options_skip_the_screening() {
    let view = &positions(1, 4, 1)[0];
    for samples in [1, 2, TwoStage::PROPOSAL.screen_worlds] {
        let budget = Budget { samples, ..BUDGET };
        assert_skipped_like_reference(view, budget, 4, SkipReason::SmallBudget);
    }
    // At most two legal cards (sampled, not yet enumerated): both would
    // survive anyway.
    for n in [1, 2] {
        let view = any_position(|v| v.tricks.len() < 5 && v.legal_actions.len() == n)
            .unwrap_or_else(|| panic!("a decision with {n} legal cards"));
        assert_skipped_like_reference(&view, BUDGET, 4, SkipReason::FewOptions);
    }
    // `min_keep` at least the option count skips as well.
    let all = TwoStage {
        min_keep: view.legal_actions.len() as u32,
        ..TwoStage::PROPOSAL
    };
    let t = analyze_two_stage(view, BUDGET, 4, None, Kernel::Classic, all).unwrap();
    assert_eq!(
        t.report.outcome,
        StageOutcome::Skipped(SkipReason::FewOptions)
    );
    assert_eq!(t.analysis, analyze(view, BUDGET, 4, None).unwrap());
}

#[test]
fn a_single_survivor_skips_stage_two() {
    let view = &positions(1, 4, 1)[0];
    let one = TwoStage {
        min_keep: 1,
        max_keep: 1,
        ..TwoStage::PROPOSAL
    };
    let t = analyze_two_stage(view, BUDGET, 7, None, Kernel::Classic, one).unwrap();
    assert_report_shape(view, &t);
    assert_eq!(t.report.outcome, StageOutcome::OneSurvivor);
    assert_eq!(t.report.survivors.len(), 1);
    assert_eq!(t.analysis.status, Status::Complete);
    // Only the stage-1 worlds were evaluated, for every option.
    assert_eq!(t.analysis.effort.planned, BUDGET.samples);
    assert_eq!(t.analysis.effort.finished, one.screen_worlds);
    assert_eq!(t.report.screen_nodes, t.analysis.effort.nodes);
    for o in &t.analysis.options {
        assert_eq!(o.estimate.unwrap().samples, one.screen_worlds);
    }
}
