// SPDX-License-Identifier: EUPL-1.2

//! Self-play refinement (#75): the learner decides card play from its own
//! view only, its rewards are its seats' final Seeger-Fabian points, seats
//! and roles rotate, generation is reproducible from seeds and independent
//! of the worker count, a run resumes bit-identically, refuses another
//! recipe or a corrupt checkpoint and starts no iteration after its cap, and
//! a PPO update moves the policy towards better-than-expected actions.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use burn::tensor::Device;
use nn_train::rl::{
    IterationLog, PoolKind, RlHyper, RlLimits, RlStop, learner_seats, learner_step, parse_pool,
    play_one, recipe, run, update_model,
};
use nn_train::selfplay::RULES;
use skat_core::{Action, Card, CardSet, Deal, Game, Phase, Seat};
use skat_nn::{NeuralModel, Phases, action_index, encode, fixture_model, masked_softmax, sha256};

fn learner() -> Vec<u8> {
    fixture_model(1, Phases::SPIEL)
}

fn reference() -> Vec<u8> {
    fixture_model(2, Phases::SPIEL)
}

fn model(bytes: &[u8]) -> NeuralModel {
    NeuralModel::load(bytes, &sha256(bytes)).expect("fixture loads")
}

/// A tiny recipe: cheap opponents only, small batches.
fn hyper() -> RlHyper {
    RlHyper {
        games: 6,
        batch: 32,
        passes: 2,
        learning_rate: 1e-2,
        snapshot_every: 1,
        pool_snapshots: 2,
        pool: parse_pool("easy=1,ref=1,snap=1").expect("pool"),
        seed: 75,
        ..RlHyper::default()
    }
}

fn limits(iterations: usize, workers: usize) -> RlLimits {
    RlLimits {
        wall_time: Duration::from_secs(3600),
        iterations,
        cpus: NonZeroUsize::new(workers).expect("> 0"),
        workers: NonZeroUsize::new(workers).expect("> 0"),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("rl-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Logs without their timings, which differ between runs.
fn untimed(logs: &[IterationLog]) -> Vec<IterationLog> {
    logs.iter()
        .map(|l| IterationLog {
            play_seconds: 0.0,
            update_seconds: 0.0,
            ..l.clone()
        })
        .collect()
}

#[test]
fn learner_steps_are_its_own_card_play_decisions_with_their_rewards() {
    let (learner_bytes, reference_bytes) = (learner(), reference());
    let (l, r) = (model(&learner_bytes), model(&reference_bytes));
    let mut declarer = false;
    let mut defender = false;
    for g in 0..6 {
        let played = play_one(&hyper(), &learner_bytes, &reference_bytes, 0, g).expect("game");
        assert_eq!(played.learner, learner_seats(g));
        let mut game = Game::new(played.deal_seed, RULES);
        let mut steps = played.steps.iter();
        for &action in &played.actions {
            let seat = game.current_seat().expect("seat to move");
            let view = game.view(seat);
            if played.learner[seat.index()] && view.phase == Phase::Spiel {
                let step = steps.next().expect("a step per learner card-play decision");
                assert_eq!(step.seat, seat);
                assert_eq!(step.action, action);
                // The only input is the acting seat's view.
                assert_eq!(step.features, encode(&view));
                let legal: Vec<u16> = view
                    .legal_actions
                    .iter()
                    .map(|&a| action_index(a).expect("index") as u16)
                    .collect();
                assert_eq!(step.legal, legal);
                assert_eq!(view.legal_actions[step.taken], action);
                let indices: Vec<usize> = legal.iter().map(|&i| usize::from(i)).collect();
                let probs =
                    masked_softmax(&l.forward_all(encode(&view)).logits, &indices).expect("finite");
                assert!((step.log_prob - probs[step.taken].ln()).abs() < 1e-5);
                let total: f32 = step.reference.iter().sum();
                assert!((total - 1.0).abs() < 1e-4);
                assert_eq!(step.reference.len(), legal.len());
                if game.declarer() == Some(seat) {
                    declarer = true;
                } else {
                    defender = true;
                }
            }
            game.apply(action).expect("recorded actions are legal");
        }
        assert!(steps.next().is_none(), "no step outside learner card play");
        assert!(game.current_seat().is_none(), "the game is over");
        let (points, won) = match game.result() {
            Some(res) => (
                res.seeger_fabian(),
                Seat::ALL.map(|s| (game.declarer() == Some(s)) == res.won),
            ),
            None => ([0; 3], [false; 3]),
        };
        assert_eq!(played.points, points);
        for s in &played.steps {
            assert_eq!(s.reward, points[s.seat.index()] as f32);
            assert_eq!(s.won, won[s.seat.index()]);
        }
        // The learner step is a function of the view: recomputing it gives
        // the same decision for the same draw.
        if let Some(s) = played.steps.first() {
            assert!(r.forward_all(s.features.clone()).value.is_finite());
        }
    }
    assert!(declarer && defender, "the learner played both roles");
}

#[test]
fn hidden_cards_do_not_change_a_learner_step() {
    let (l, r) = (model(&learner()), model(&reference()));
    let mut checked = 0;
    for g in 0..6 {
        let played = play_one(&hyper(), &learner(), &reference(), 0, g).expect("game");
        let deal = Deal::from_seed(played.deal_seed);
        let mut game = Game::new(played.deal_seed, RULES);
        for (k, &action) in played.actions.iter().enumerate() {
            let seat = game.current_seat().expect("seat to move");
            let view = game.view(seat);
            if view.phase == Phase::Spiel && view.declarer_cards.is_none() {
                // Swap two unplayed cards of the other seats that follow
                // alike, replay the same actions and compare the step.
                if let Some(other) = swap(deal, &played.actions[..k], seat, &view) {
                    let mut again = Game::from_deal(other, RULES);
                    for &a in &played.actions[..k] {
                        again.apply(a).expect("still legal after the swap");
                    }
                    let other_view = again.view(seat);
                    assert_eq!(other_view, view);
                    for u in [0.1, 0.5, 0.9] {
                        assert_eq!(
                            learner_step(&other_view, &l, &r, u).expect("step"),
                            learner_step(&view, &l, &r, u).expect("step")
                        );
                    }
                    checked += 1;
                }
            }
            game.apply(action).expect("legal");
        }
    }
    assert!(checked > 20, "only {checked} decisions checked");
}

/// The deal with one unplayed card of each other seat swapped, if both
/// follow the same suit in the contract; `None` otherwise.
fn swap(deal: Deal, before: &[Action], seat: Seat, view: &skat_core::PlayerView) -> Option<Deal> {
    let [p, q] = [seat.next(), seat.next().next()];
    let mut used = CardSet::EMPTY;
    for a in before {
        match *a {
            Action::Play(c) => {
                used.insert(c);
            }
            Action::Discard(cards) => used |= cards,
            _ => {}
        }
    }
    let game = view.contract.map(|c| c.game)?;
    let free =
        |s: Seat| -> Vec<Card> { deal.hand(s).iter().filter(|c| !used.contains(*c)).collect() };
    for x in free(p) {
        for y in free(q) {
            if game.follow(x) != game.follow(y) {
                continue;
            }
            let mut hands = Seat::ALL.map(|s| deal.hand(s));
            hands[p.index()].remove(x);
            hands[p.index()].insert(y);
            hands[q.index()].remove(y);
            hands[q.index()].insert(x);
            return Deal::new(hands, deal.skat()).ok();
        }
    }
    None
}

#[test]
fn seatings_rotate_the_learner_over_every_seat_alone_and_paired() {
    let mut alone = [false; 3];
    let mut paired = [false; 3];
    for g in 0..6 {
        let seats = learner_seats(g);
        let n = seats.iter().filter(|&&b| b).count();
        for s in 0..3 {
            if seats[s] && n == 1 {
                alone[s] = true;
            }
            if seats[s] && n == 2 {
                paired[s] = true;
            }
        }
        assert_eq!(learner_seats(g + 6), seats);
    }
    assert_eq!(alone, [true; 3]);
    assert_eq!(paired, [true; 3]);
}

#[test]
fn games_are_reproducible_from_seeds() {
    let h = hyper();
    let a = play_one(&h, &learner(), &reference(), 3, 4).expect("game");
    assert_eq!(
        a,
        play_one(&h, &learner(), &reference(), 3, 4).expect("game")
    );
    let b = play_one(&h, &learner(), &reference(), 4, 4).expect("game");
    assert_ne!(a.deal_seed, b.deal_seed, "each iteration deals anew");
    let c = play_one(&RlHyper { seed: 76, ..h }, &learner(), &reference(), 3, 4).expect("game");
    assert_ne!(a.deal_seed, c.deal_seed, "the run seed changes the deals");
}

#[test]
fn pools_parse_and_reject_bad_specs() {
    assert_eq!(
        parse_pool("easy=1,strong=2").expect("valid"),
        vec![(PoolKind::Easy, 1), (PoolKind::Strong, 2)]
    );
    assert!(parse_pool("easy=0").is_err());
    assert!(parse_pool("hard=1").is_err());
    assert!(parse_pool("easy").is_err());
}

#[test]
fn a_ppo_update_favours_better_than_expected_actions() {
    let bytes = learner();
    let (l, r) = (model(&bytes), model(&reference()));
    // The first card-play view of a game.
    let played = play_one(&hyper(), &bytes, &reference(), 0, 0).expect("game");
    let mut game = Game::new(played.deal_seed, RULES);
    let mut first = None;
    for &a in &played.actions {
        let view = game.view(game.current_seat().expect("seat"));
        if view.phase == Phase::Spiel && view.legal_actions.len() > 2 {
            first = Some(view);
            break;
        }
        game.apply(a).expect("legal");
    }
    let view = first.expect("a card-play decision with three options");
    let good = learner_step(&view, &l, &r, 0.0).expect("step");
    let bad = learner_step(&view, &l, &r, 0.999).expect("step");
    assert_ne!(good.taken, bad.taken);
    let steps: Vec<_> = (0..16)
        .flat_map(|_| {
            [
                nn_train::rl::Step {
                    reward: 100.0,
                    ..good.clone()
                },
                nn_train::rl::Step {
                    reward: -100.0,
                    ..bad.clone()
                },
            ]
        })
        .collect();
    let h = RlHyper {
        reference_weight: 0.0,
        entropy_weight: 0.0,
        ..hyper()
    };
    let updated = update_model(&bytes, &steps, &h, 0, &Device::flex().autodiff()).expect("update");
    let after = model(&updated);
    let probs = |m: &NeuralModel| {
        let idx: Vec<usize> = good.legal.iter().map(|&i| usize::from(i)).collect();
        masked_softmax(&m.forward_all(good.features.clone()).logits, &idx).expect("finite")
    };
    let (p0, p1) = (probs(&l), probs(&after));
    assert!(p1[good.taken] > p0[good.taken], "{p0:?} -> {p1:?}");
    assert!(p1[bad.taken] < p0[bad.taken], "{p0:?} -> {p1:?}");
}

#[test]
fn runs_are_reproducible_and_independent_of_the_worker_count() {
    let start = learner();
    let recipe = recipe(&start, hyper()).expect("recipe");
    let device = Device::flex().autodiff();
    let one = run(
        &scratch("w1"),
        &recipe,
        &start,
        &device,
        limits(3, 1),
        |_| true,
    )
    .expect("run");
    let three = run(
        &scratch("w3"),
        &recipe,
        &start,
        &device,
        limits(3, 3),
        |_| true,
    )
    .expect("run");
    assert_eq!(one.stop, RlStop::Iterations);
    assert_eq!(one.model, three.model);
    assert_eq!(untimed(&one.state.logs), untimed(&three.state.logs));
    assert_eq!(one.state.snapshots, three.state.snapshots);
    assert_eq!(
        one.state.snapshots.len(),
        2,
        "snapshots after iterations 1 and 2"
    );
    assert_ne!(one.model, start, "the learner changed");
}

#[test]
fn an_interrupted_run_resumes_bit_identically() {
    let start = learner();
    let recipe = recipe(&start, hyper()).expect("recipe");
    let device = Device::flex().autodiff();
    let whole = run(
        &scratch("whole"),
        &recipe,
        &start,
        &device,
        limits(4, 2),
        |_| true,
    )
    .expect("run");
    let dir = scratch("parts");
    // Stopped by the callback after one iteration, then by the cap.
    let first = run(&dir, &recipe, &start, &device, limits(4, 2), |_| false).expect("run");
    assert_eq!(first.stop, RlStop::Stopped);
    assert_eq!(first.state.logs.len(), 1);
    let second = run(&dir, &recipe, &start, &device, limits(2, 2), |_| true).expect("run");
    assert_eq!(second.state.logs.len(), 2);
    let rest = run(&dir, &recipe, &start, &device, limits(4, 2), |_| true).expect("run");
    assert_eq!(rest.model, whole.model);
    assert_eq!(untimed(&rest.state.logs), untimed(&whole.state.logs));
    assert_eq!(rest.state.snapshots, whole.state.snapshots);
}

#[test]
fn a_run_refuses_another_recipe_a_corrupt_checkpoint_and_another_start() {
    let start = learner();
    let recipe = recipe(&start, hyper()).expect("recipe");
    let device = Device::flex().autodiff();
    let dir = scratch("refuse");
    run(&dir, &recipe, &start, &device, limits(1, 1), |_| true).expect("run");
    let other = nn_train::rl::recipe(&start, RlHyper { seed: 1, ..hyper() }).expect("recipe");
    assert!(run(&dir, &other, &start, &device, limits(2, 1), |_| true).is_err());
    assert!(run(&dir, &recipe, &reference(), &device, limits(2, 1), |_| true).is_err());
    let weights = dir.join("checkpoint/iter-00001/weights.bpk");
    let mut bytes = std::fs::read(&weights).expect("weights");
    bytes[100] ^= 1;
    std::fs::write(&weights, bytes).expect("write");
    assert!(run(&dir, &recipe, &start, &device, limits(2, 1), |_| true).is_err());
}

#[test]
fn no_iteration_starts_after_the_wall_time_cap() {
    let start = learner();
    let recipe = recipe(&start, hyper()).expect("recipe");
    let capped = RlLimits {
        wall_time: Duration::ZERO,
        ..limits(5, 1)
    };
    let r = run(
        &scratch("cap"),
        &recipe,
        &start,
        &Device::flex().autodiff(),
        capped,
        |_| true,
    )
    .expect("run");
    assert_eq!(r.stop, RlStop::WallTime);
    assert!(r.state.logs.is_empty());
    assert_eq!(r.model.len(), start.len());
}
