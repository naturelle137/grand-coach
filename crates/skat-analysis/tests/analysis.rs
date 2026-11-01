// SPDX-License-Identifier: EUPL-1.2

//! Option evaluation: every decision type, information isolation,
//! reproducibility, cancellation and budgets, scoring-sensitive decisions
//! (Überreizen, Kontra/Re) and Monte Carlo against exact enumeration.

mod common;

use common::*;
use skat_analysis::sampling::Diagnostic;
use skat_analysis::sampling::Place;
use skat_analysis::sampling::World;
use skat_analysis::{
    Analysis, Assumption, Budget, Cancel, CardClass, Estimate, Status, analyze, analyze_forgetful,
    analyze_known, with_max_threads,
};
use skat_core::{
    Action, Announcement, CardSet, Deal, Game, GameType, Phase, PlayerView, RuleConfig, Seat, Suit,
};

const SMALL: Budget = Budget {
    samples: 4,
    nodes_per_task: 50_000_000,
};

fn standard_deal() -> Deal {
    deal(
        "CJ SJ HJ DJ CA CT CK CQ C9 C8",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 DA DT DK DQ D9 D8",
        "C7 D7",
    )
}

fn game(kontra_re: bool) -> Game {
    Game::from_deal(standard_deal(), RuleConfig { kontra_re })
}

/// MH 18, VH holds, MH and HH pass: VH declarer at 18.
const VH_AT_18: [Action; 4] = [Action::Bid(18), Action::Hold, Action::Pass, Action::Pass];

fn run(g: &mut Game, actions: &[Action]) {
    for &a in actions {
        g.apply(a).unwrap_or_else(|e| panic!("{a:?}: {e:?}"));
    }
}

fn current_view(g: &Game) -> PlayerView {
    g.view(g.current_seat().expect("a decision is pending"))
}

fn estimate(a: &Analysis, action: Action) -> Estimate {
    a.options
        .iter()
        .find(|o| o.action == action)
        .unwrap_or_else(|| panic!("{action:?} not an option"))
        .estimate
        .expect("evaluated")
}

/// Every legal action is an option, in order, with sane values.
fn assert_complete(view: &PlayerView, a: &Analysis) {
    assert_eq!(a.status, Status::Complete);
    assert_eq!(a.seat, view.seat);
    assert_eq!(a.phase, view.phase);
    let actions: Vec<Action> = a.options.iter().map(|o| o.action).collect();
    assert_eq!(actions, view.legal_actions);
    for o in &a.options {
        let e = o.estimate.expect("every option evaluated");
        assert!((0.0..=1.0).contains(&e.win_chance), "{e:?}");
        assert!((0.0..=1.0).contains(&e.thrown_in), "{e:?}");
        assert!(e.score_std_error >= 0.0);
        assert!(e.samples > 0);
    }
}

#[test]
fn bidding_options_include_pass_and_every_bid() {
    let g = game(false);
    let view = current_view(&g);
    assert_eq!((view.seat, view.phase), (Seat::Mittelhand, Phase::Reizen));
    let a = analyze(&view, SMALL, 1, None).unwrap();
    assert_complete(&view, &a);
    // MH holds Pik and Herz only: passing gives the deal to VH or HH.
    let pass = estimate(&a, Action::Pass);
    assert!(pass.win_chance + pass.thrown_in <= 1.0);
    // A bid far above any game MH can play is overbid when MH gets it.
    let high = estimate(&a, Action::Bid(264));
    assert!(
        high.expected_score < pass.expected_score,
        "{high:?} {pass:?}"
    );
}

#[test]
fn hand_or_pickup_options() {
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    let view = current_view(&g);
    assert_eq!(view.phase, Phase::SkatAufnahme);
    let a = analyze(&view, SMALL, 2, None).unwrap();
    assert_complete(&view, &a);
    // With all Jacks and six Kreuz the declarer wins either way.
    assert_eq!(estimate(&a, Action::Hand).win_chance, 1.0);
    assert_eq!(estimate(&a, Action::TakeSkat).win_chance, 1.0);
}

#[test]
fn every_discard_is_an_option() {
    // VH holds only low cards: the continuation policy announces Null, whose
    // double-dummy searches are cheap.
    let low = deal(
        "C7 C8 C9 S7 S8 S9 H7 H8 H9 D7",
        "CJ SJ HJ DJ CA CT CK CQ SA ST",
        "SK SQ HA HT HK HQ DA DT DK DQ",
        "D8 D9",
    );
    let mut g = Game::from_deal(low, RuleConfig::default());
    run(&mut g, &VH_AT_18);
    run(&mut g, &[Action::TakeSkat]);
    let view = current_view(&g);
    assert_eq!(view.legal_actions.len(), 66);
    let budget = Budget {
        samples: 5,
        ..SMALL
    };
    let a = analyze(&view, budget, 3, None).unwrap();
    assert_complete(&view, &a);
    // Screening: every option on 3 distributions, the best 6 on all 5.
    let mut counts: Vec<u32> = a
        .options
        .iter()
        .map(|o| o.estimate.unwrap().samples)
        .collect();
    counts.sort_unstable();
    assert_eq!(counts.iter().filter(|&&n| n == 5).count(), 6);
    assert_eq!(counts.iter().filter(|&&n| n == 3).count(), 60);
}

#[test]
fn ueberreizen_is_scored_through_the_core() {
    // VH declarer at 99 playing Hand. VH holds no Karo, so Karo is at most
    // mit 6 (DA DT in the Skat): 9 * (6 + game, Hand, Schneider, Schwarz)
    // = 90 < 99: always overbid and lost with the smallest multiple of 9
    // reaching 99, i.e. 99, doubled.
    let mut g = game(false);
    let mut bids: Vec<Action> = Vec::new();
    for v in skat_core::reizwerte().into_iter().filter(|&v| v <= 99) {
        bids.push(Action::Bid(v));
        bids.push(Action::Hold);
    }
    run(&mut g, &bids);
    run(&mut g, &[Action::Pass, Action::Pass, Action::Hand]);
    let view = current_view(&g);
    assert_eq!((view.phase, view.bid), (Phase::Ansage, 99));
    let a = analyze(&view, SMALL, 4, None).unwrap();
    assert_complete(&view, &a);
    let karo = estimate(
        &a,
        Action::Announce(Announcement::plain(GameType::Farbe(Suit::Karo))),
    );
    assert_eq!(karo.win_chance, 0.0);
    assert_eq!(karo.expected_game_points, -198.0);
    assert_eq!(karo.expected_score, -198.0 - 50.0);
    // Kreuz Hand: ten trumps, the declarer takes every trick (Schwarz).
    // Mit 10 or 11 (C7 in the Skat or not): 12 * 14 or 12 * 15.
    let kreuz = estimate(
        &a,
        Action::Announce(Announcement::plain(GameType::Farbe(Suit::Kreuz))),
    );
    assert_eq!(kreuz.win_chance, 1.0);
    assert!(
        (168.0..=180.0).contains(&kreuz.expected_game_points),
        "{kreuz:?}"
    );
    // Null Hand (35) is below the bid and therefore not an option.
    assert!(
        a.options
            .iter()
            .all(|o| o.action != Action::Announce(Announcement::plain(GameType::Null)))
    );
}

#[test]
fn re_doubles_the_game_points_of_kein_re() {
    let mut g = game(true);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    run(&mut g, &[Action::Play(c("CJ")), Action::Kontra]);
    let view = current_view(&g);
    assert_eq!(view.legal_actions, vec![Action::Re, Action::KeinRe]);
    let a = analyze(&view, SMALL, 5, None).unwrap();
    assert_complete(&view, &a);
    let re = estimate(&a, Action::Re);
    let kein = estimate(&a, Action::KeinRe);
    assert_eq!(re.win_chance, kein.win_chance);
    assert_eq!(re.expected_game_points, 2.0 * kein.expected_game_points);
}

#[test]
fn kontra_in_play_and_in_the_ouvert_round() {
    // In play: MH may say Kontra instead of its first card.
    let mut g = game(true);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    run(&mut g, &[Action::Play(c("CJ"))]);
    let view = current_view(&g);
    assert!(view.legal_actions.contains(&Action::Kontra));
    let a = analyze(&view, SMALL, 6, None).unwrap();
    assert_complete(&view, &a);

    // Ouvert round: HH is not eligible, so after KeinKontra nobody says
    // Kontra. Kontra multiplies only the declarer's points: the defenders'
    // Seeger-Fabian points (40 if the declarer loses) stay the same.
    let mut g = game(true);
    run(&mut g, &VH_AT_18);
    let ouvert = Announcement {
        game: GameType::Grand,
        schneider: true,
        schwarz: true,
        ouvert: true,
    };
    run(&mut g, &[Action::Hand, Action::Announce(ouvert)]);
    let view = current_view(&g);
    assert_eq!(view.legal_actions, vec![Action::Kontra, Action::KeinKontra]);
    let a = analyze(&view, SMALL, 7, None).unwrap();
    assert_complete(&view, &a);
    let kontra = estimate(&a, Action::Kontra);
    let kein = estimate(&a, Action::KeinKontra);
    assert_eq!(kontra.win_chance, kein.win_chance);
    assert_eq!(kontra.expected_score, kein.expected_score);
    assert_ne!(kontra.expected_game_points, kein.expected_game_points);
}

#[test]
fn card_play_options_and_exact_results() {
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    // Seven tricks by the first legal card; three tricks are left.
    for _ in 0..21 {
        let view = current_view(&g);
        let first = view.legal_actions[0];
        g.apply(first).unwrap();
    }
    let view = current_view(&g);
    let budget = Budget {
        samples: 1000,
        ..SMALL
    };
    let a = analyze(&view, budget, 8, None).unwrap();
    assert_complete(&view, &a);
    assert!(a.effort.enumerated);
    assert_eq!(f64::from(a.effort.planned), a.effort.distributions);
    for o in &a.options {
        let e = o.estimate.unwrap();
        assert!(e.exact && e.score_std_error == 0.0);
    }

    // Early in the game the hidden cards are sampled: an estimate.
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    let view = current_view(&g);
    let a = analyze(&view, SMALL, 9, None).unwrap();
    assert_complete(&view, &a);
    assert!(!a.effort.enumerated);
    assert!(a.options.iter().all(|o| !o.estimate.unwrap().exact));
}

#[test]
fn inferences_count_hidden_trumps_or_jacks() {
    // Bidding: MH holds no Jack, so the four Jacks are with VH, HH or in the
    // Skat; every hidden place gets an estimate, MH itself none.
    let g = game(false);
    let view = current_view(&g);
    let a = analyze(&view, SMALL, 1, None).unwrap();
    let holders: Vec<Place> = a.inferences.iter().map(|i| i.holder).collect();
    assert_eq!(
        holders,
        [
            Place::Seat(Seat::Vorhand),
            Place::Seat(Seat::Hinterhand),
            Place::Skat
        ]
    );
    let total: f64 = a.inferences.iter().map(|i| i.expected).sum();
    assert!((total - 4.0).abs() < 1e-9, "{total}");
    for i in &a.inferences {
        assert_eq!(i.class, CardClass::Jacks);
        assert!(f64::from(i.min) <= i.expected && i.expected <= f64::from(i.max));
        assert!(!i.certain, "sampled, so never certain: {i:?}");
    }

    // Grand Hand by VH with all Jacks: in the last three tricks every
    // distribution is enumerated, and the defenders provably hold no trump.
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    for _ in 0..21 {
        let first = current_view(&g).legal_actions[0];
        g.apply(first).unwrap();
    }
    let view = current_view(&g);
    let budget = Budget {
        samples: 1000,
        ..SMALL
    };
    let a = analyze(&view, budget, 8, None).unwrap();
    assert!(a.effort.enumerated);
    assert!(!a.inferences.is_empty());
    for i in &a.inferences {
        assert_eq!(i.class, CardClass::Trumps);
        if i.min == i.max {
            assert!(i.certain, "{i:?}");
        }
    }

    // Null: no trumps to count.
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Null)),
        ],
    );
    let a = analyze(&current_view(&g), SMALL, 3, None).unwrap();
    assert!(a.inferences.is_empty());
}

#[test]
fn same_inputs_same_result() {
    let mut g = game(true);
    run(&mut g, &VH_AT_18);
    let view = current_view(&g);
    let a = analyze(&view, SMALL, 11, None).unwrap();
    let b = analyze(&view, SMALL, 11, None).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.effort.seed, 11);
    // A thread limit changes the scheduling, not the result.
    let one = with_max_threads(std::num::NonZeroUsize::MIN, || {
        analyze(&view, SMALL, 11, None).unwrap()
    });
    assert_eq!(a, one);
}

#[test]
fn concealed_cards_do_not_change_the_analysis() {
    // MH's cards and the public history are the same; VH, HH and the Skat
    // differ (both deals let MH bid 18, VH hold, MH pass, HH pass).
    let other = deal(
        "CJ SJ HJ DJ CA CT CK CQ DA DT",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 C9 C8 DK DQ D9 D8",
        "C7 D7",
    );
    let mut views = Vec::new();
    for d in [standard_deal(), other] {
        let mut g = Game::from_deal(d, RuleConfig { kontra_re: true });
        run(&mut g, &VH_AT_18);
        run(
            &mut g,
            &[
                Action::Hand,
                Action::Announce(Announcement::plain(GameType::Grand)),
            ],
        );
        run(&mut g, &[Action::Play(c("CJ"))]);
        views.push(g.view(Seat::Mittelhand));
    }
    assert_eq!(views[0], views[1]);
    let a = analyze(&views[0], SMALL, 12, None).unwrap();
    let b = analyze(&views[1], SMALL, 12, None).unwrap();
    assert_eq!(a, b);
}

#[test]
fn cancellation_and_exhausted_budgets_are_reported() {
    let g = game(false);
    let view = current_view(&g);
    let cancel = Cancel::new();
    cancel.cancel();
    let a = analyze(&view, SMALL, 1, Some(&cancel)).unwrap();
    assert_eq!(a.status, Status::Cancelled);
    assert_eq!(a.effort.finished, 0);
    assert!(a.options.iter().all(|o| o.estimate.is_none()));

    let tiny = Budget {
        samples: 4,
        nodes_per_task: 10,
    };
    let a = analyze(&view, tiny, 1, None).unwrap();
    assert_eq!(a.status, Status::BudgetExhausted);
    assert!(a.options.iter().all(|o| o.estimate.is_none()));

    // Not this seat's turn: nothing to decide.
    let a = analyze(&g.view(Seat::Vorhand), SMALL, 1, None).unwrap();
    assert_eq!(a.status, Status::NoDecision);
    assert!(a.options.is_empty());
}

#[test]
fn inconsistent_history_is_diagnosed() {
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    let mut view = current_view(&g);
    view.bidding.truncate(1);
    assert_eq!(
        analyze(&view, SMALL, 1, None).unwrap_err(),
        vec![Diagnostic::InvalidHistory]
    );
}

#[test]
fn monte_carlo_matches_exact_enumeration_within_tolerance() {
    // Seeded games played by the first legal action until 4 tricks are
    // left; there the hidden cards have up to a few thousand distributions.
    // Tolerance: 4 standard errors plus 1 point per option.
    let mut checked = 0;
    for seed in 0..40 {
        let mut g = Game::new(seed, RuleConfig::default());
        while g.phase() != Phase::Spiel || g.tricks().len() < 6 {
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
        if view.legal_actions.len() < 2 {
            continue;
        }
        let exact = analyze(
            &view,
            Budget {
                samples: 5000,
                ..SMALL
            },
            1,
            None,
        )
        .unwrap();
        if !exact.effort.enumerated {
            continue;
        }
        let mc = analyze(
            &view,
            Budget {
                samples: 64,
                ..SMALL
            },
            seed,
            None,
        )
        .unwrap();
        assert!(!mc.effort.enumerated);
        for (e, m) in exact.options.iter().zip(&mc.options) {
            let (e, m) = (e.estimate.unwrap(), m.estimate.unwrap());
            assert!(e.exact && !m.exact);
            let tolerance = 4.0 * m.score_std_error + 1.0;
            assert!(
                (e.expected_score - m.expected_score).abs() <= tolerance,
                "seed {seed}: exact {e:?} vs mc {m:?}"
            );
        }
        checked += 1;
        if checked == 5 {
            break;
        }
    }
    assert!(checked >= 3, "{checked}");
}

/// The real distribution gives one exact result; a distribution that
/// contradicts the view is rejected.
#[test]
fn known_world_analysis_is_exact_and_checked() {
    let mut g = game(false);
    // Bidding: Mittelhand's first call, with every card known.
    let view = current_view(&g);
    let world = World::from_omniscient(&g.omniscient_view());
    let a = analyze_known(&view, world, SMALL, None).unwrap();
    assert_complete(&view, &a);
    assert_eq!(a.effort.planned, 1);
    assert!(a.options.iter().all(|o| o.estimate.unwrap().exact));
    // The estimates use the known deal; the assumptions say so, and the
    // inferences stay what the seat could deduce from its own view.
    assert!(a.effort.known);
    assert!(a.assumptions.contains(&Assumption::KnownDeal));
    assert!(!a.assumptions.contains(&Assumption::UniformHiddenCards));
    assert!(!a.assumptions.contains(&Assumption::BiddingEvidence));
    assert!(
        a.inferences.iter().any(|i| !i.certain),
        "{:?}",
        a.inferences
    );

    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    let view = current_view(&g);
    let world = World::from_omniscient(&g.omniscient_view());
    let a = analyze_known(&view, world, SMALL, None).unwrap();
    assert_complete(&view, &a);
    // Four Jacks and seven clubs: the Grand Hand cannot be lost.
    let best = a
        .options
        .iter()
        .map(|o| o.estimate.unwrap().win_chance)
        .fold(0.0, f64::max);
    assert_eq!(best, 1.0);
    assert_eq!(analyze_known(&view, world, SMALL, None).unwrap(), a);

    // A distribution that moves one of the seat's own cards is rejected.
    let mh = Seat::Mittelhand.index();
    let mut foreign = world;
    let own = view.hand.iter().next().unwrap();
    let other = world.hands[mh].iter().next().unwrap();
    foreign.hands[view.seat.index()] = (view.hand - CardSet::single(own)) | CardSet::single(other);
    foreign.hands[mh] = (world.hands[mh] - CardSet::single(other)) | CardSet::single(own);
    assert!(analyze_known(&view, foreign, SMALL, None).is_err());
}

/// With every card known, the inferences are still those of the seat's
/// own information set: the same as a plain analysis with seed 0.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn known_world_analysis_keeps_the_seats_own_inferences() {
    let g = game(false);
    let view = current_view(&g);
    let world = World::from_omniscient(&g.omniscient_view());
    let known = analyze_known(&view, world, SMALL, None).unwrap();
    let own = analyze(&view, SMALL, 0, None).unwrap();
    assert_complete(&view, &own);
    assert!(!own.effort.known);
    assert!(own.assumptions.contains(&Assumption::UniformHiddenCards));
    assert_eq!(known.inferences, own.inferences);
}

/// Forgotten cards turn an exact late position into an estimate; nothing
/// forgotten (or outside card play) is the plain analysis.
#[test]
fn forgetful_analysis_believes_played_cards_are_out() {
    let mut g = game(false);
    run(&mut g, &VH_AT_18);
    run(
        &mut g,
        &[
            Action::Hand,
            Action::Announce(Announcement::plain(GameType::Grand)),
        ],
    );
    for _ in 0..21 {
        let view = current_view(&g);
        g.apply(view.legal_actions[0]).unwrap();
    }
    let view = current_view(&g);
    let budget = Budget {
        samples: 1000,
        ..SMALL
    };
    let exact = analyze(&view, budget, 3, None).unwrap();
    assert_eq!(
        analyze_forgetful(&view, CardSet::EMPTY, budget, 3, None).unwrap(),
        exact
    );
    let first: CardSet = view.tricks[0].cards.iter().copied().collect();
    let budget = Budget {
        samples: 16,
        ..SMALL
    };
    let a = analyze_forgetful(&view, first, budget, 3, None).unwrap();
    assert_complete(&view, &a);
    assert!(!a.effort.enumerated);
    assert!(a.options.iter().all(|o| !o.estimate.unwrap().exact));
    assert_eq!(analyze_forgetful(&view, first, budget, 3, None).unwrap(), a);
    // Cards never played cannot be forgotten.
    let unplayed = view.hand;
    assert_eq!(
        analyze_forgetful(&view, unplayed, budget, 3, None).unwrap(),
        analyze(&view, budget, 3, None).unwrap()
    );

    let g = game(false);
    let view = current_view(&g);
    let bidding = analyze(&view, SMALL, 4, None).unwrap();
    assert_eq!(
        analyze_forgetful(&view, CardSet::ALL, SMALL, 4, None).unwrap(),
        bidding
    );
}
