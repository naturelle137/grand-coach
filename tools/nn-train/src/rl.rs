// SPDX-License-Identifier: EUPL-1.2

//! Self-play refinement of the card-play network from game outcomes (#75).
//!
//! The learner starts from a trained model file (the #73 network) and
//! improves its card play (phase Spiel, Kontra/Re included) by proximal
//! policy optimisation (PPO, clipped surrogate) with a Monte-Carlo
//! baseline: the reward of a decision is the acting seat's final
//! Seeger-Fabian points (0 if the deal is thrown in), the advantage is that
//! reward minus the value the network predicted at the decision. There is
//! no bootstrapping and no privileged critic: the value head sees the same
//! [`PlayerView`] encoding as the policy, and nothing else enters the
//! network. The option heads keep learning the taken action's outcome
//! (expected points and win), and a cross-entropy term towards the frozen
//! reference policy keeps the learner close to its start.
//!
//! One **iteration** plays [`RlHyper::games`] games and then makes
//! [`RlHyper::passes`] passes of mini-batch Adam over their learner
//! decisions. The opponents of a game come from a frozen pool
//! ([`PoolKind`]): Classic profiles, the reference model and older learner
//! snapshots. Seats rotate with the game index like
//! [`skat_ai::arena::SEATINGS`]: the learner alone on each seat against two
//! copies of the opponent, or as a pair against one; every seat and role
//! is covered. The learner decides pre-play by the rules of thumb (the
//! fallback of a card-play model).
//!
//! Reproducibility: deal seeds, pool draws, opponent seeds and the
//! learner's sampling come from separate keys mixed with the run seed, the
//! iteration and the game index; games run in parallel but are collected
//! in game order, so the result does not depend on the worker count. Every
//! schedule depends on the iteration number only, never on wall time:
//! after every iteration a checkpoint (weights, Adam state, logs, pool) is
//! written, and a restart with the same recipe continues exactly as an
//! uninterrupted run. No iteration starts after the wall-time cap.

use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use burn::module::Module;
use burn::optim::{AdamConfig, GradientsParams, ModuleOptimizer};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Bytes, Device, Tensor};
use serde::{Deserialize, Serialize};
use skat_ai::{Level, NeuralOpponent, Opponent, Player, Profile};
use skat_core::{Action, Game, PlayerView, Seat};
use skat_nn::{
    ACTIONS, HEADER_BYTES, ModelHeader, NeuralModel, PolicyValueNet, action_index, masked_softmax,
    read_header, sha256, write_model,
};

use crate::selfplay::{RULES, mix};
use crate::train::{
    PHASES, SplitMix, VALUE_SCALE, from_record, hex, record_bytes, tensor, write_atomic,
};

/// Version of the self-play recipe and run-directory layout.
pub const RL_FORMAT: u32 = 1;

const DEAL_KEY: u64 = 0x726c_6465_616c;
const POOL_KEY: u64 = 0x726c_706f_6f6c;
const OPPONENT_KEY: u64 = 0x726c_6f70_706f;
const SAMPLE_KEY: u64 = 0x726c_7361_6d70;
const ORDER_KEY: u64 = 0x726c_6f72_6465;

/// Which seats the learner holds in game `g` of an iteration (`true`:
/// learner), like [`skat_ai::arena::SEATINGS`] with the learner as A.
pub fn learner_seats(game: usize) -> [bool; 3] {
    skat_ai::arena::SEATINGS[game % skat_ai::arena::SEATINGS.len()]
}

/// A kind of frozen opponent in the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PoolKind {
    /// Classic Easy (rules of thumb).
    Easy,
    /// Classic Medium.
    Medium,
    /// Classic Strong.
    Strong,
    /// The reference model (the learner's start), greedy by its policy.
    Reference,
    /// One of the newest learner snapshots, greedy by its policy; the
    /// reference model while there is none.
    Snapshot,
}

impl PoolKind {
    /// Every kind, in log order.
    pub const ALL: [PoolKind; 5] = [
        PoolKind::Easy,
        PoolKind::Medium,
        PoolKind::Strong,
        PoolKind::Reference,
        PoolKind::Snapshot,
    ];

    /// The name used on the command line and in logs.
    pub fn name(self) -> &'static str {
        match self {
            PoolKind::Easy => "easy",
            PoolKind::Medium => "medium",
            PoolKind::Strong => "strong",
            PoolKind::Reference => "ref",
            PoolKind::Snapshot => "snap",
        }
    }

    /// The kind called `name`.
    pub fn parse(name: &str) -> Option<PoolKind> {
        PoolKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// Parses a pool such as `easy=1,medium=2,strong=2,ref=1,snap=3`.
///
/// # Errors
///
/// On an unknown kind, a missing weight or an all-zero pool.
pub fn parse_pool(spec: &str) -> Result<Vec<(PoolKind, u32)>, String> {
    let mut pool = Vec::new();
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (name, weight) = part
            .split_once('=')
            .ok_or_else(|| format!("pool entry {part:?} needs KIND=WEIGHT"))?;
        let kind = PoolKind::parse(name).ok_or_else(|| format!("unknown pool kind {name:?}"))?;
        let weight = weight
            .parse()
            .map_err(|_| format!("pool weight {weight:?} is not a number"))?;
        pool.push((kind, weight));
    }
    if pool.iter().all(|&(_, w)| w == 0) {
        return Err("the pool needs a positive weight".into());
    }
    Ok(pool)
}

/// Hyperparameters of a self-play run. Everything here determines the
/// result; caps that only decide where a run stops are [`RlLimits`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RlHyper {
    /// Games per iteration (a multiple of 6 rotates every seating evenly).
    pub games: usize,
    /// Learner decisions per Adam step.
    pub batch: usize,
    /// Passes over an iteration's decisions.
    pub passes: usize,
    /// Adam learning rate (constant).
    pub learning_rate: f64,
    /// PPO clip range of the probability ratio.
    pub clip: f32,
    /// Weight of the value loss.
    pub value_weight: f32,
    /// Weight of the taken option's score loss.
    pub score_weight: f32,
    /// Weight of the taken option's win loss.
    pub win_weight: f32,
    /// Weight of the entropy bonus.
    pub entropy_weight: f32,
    /// Weight of the cross-entropy towards the reference policy.
    pub reference_weight: f32,
    /// A snapshot of the learner joins the pool every this many iterations.
    pub snapshot_every: usize,
    /// How many of the newest snapshots the pool holds.
    pub pool_snapshots: usize,
    /// Opponent kinds with their weights per game.
    pub pool: Vec<(PoolKind, u32)>,
    /// Run seed.
    pub seed: u64,
}

impl Default for RlHyper {
    fn default() -> RlHyper {
        RlHyper {
            games: 600,
            batch: 512,
            passes: 2,
            learning_rate: 1e-4,
            clip: 0.2,
            value_weight: 0.5,
            score_weight: 0.5,
            win_weight: 0.5,
            entropy_weight: 0.01,
            reference_weight: 0.05,
            snapshot_every: 10,
            pool_snapshots: 8,
            pool: parse_pool("easy=1,medium=2,strong=2,ref=1,snap=3").expect("valid default"),
            seed: 0,
        }
    }
}

/// Everything that determines a self-play run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RlRecipe {
    /// [`RL_FORMAT`].
    pub rl_format: u32,
    /// Encoder version of the model.
    pub encoder_version: u16,
    /// Phases the learner decides ([`PHASES`]).
    pub phases: u8,
    /// SHA-256 of the start model file (also the reference), hexadecimal.
    pub start_sha256: String,
    /// Hidden width of the start model.
    pub hidden: usize,
    /// Hidden layers of the start model; omitted when it is the minimum, so
    /// two-layer runs resume and compare as before.
    #[serde(
        default = "crate::train::min_layers",
        skip_serializing_if = "crate::train::is_min_layers"
    )]
    pub layers: usize,
    /// Hyperparameters.
    pub hyper: RlHyper,
}

/// Caps of one invocation; they never change the result of an iteration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RlLimits {
    /// No iteration starts after it.
    pub wall_time: Duration,
    /// The run stops after this many iterations in total.
    pub iterations: usize,
    /// CPUs for the games.
    pub cpus: NonZeroUsize,
    /// Parallel games; each gets `cpus / workers` analysis threads.
    pub workers: NonZeroUsize,
}

impl RlLimits {
    fn threads(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.cpus.get() / self.workers.get()).unwrap_or(NonZeroUsize::MIN)
    }
}

/// One learner decision of an iteration.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// The acting seat (bookkeeping, not a model input).
    pub seat: Seat,
    /// The taken action.
    pub action: Action,
    /// The acting seat's encoded view: the only model input.
    pub features: Vec<f32>,
    /// Action indices of the legal actions, in the view's order.
    pub legal: Vec<u16>,
    /// Position of the taken action in `legal`.
    pub taken: usize,
    /// Log-probability of the taken action under the acting policy.
    pub log_prob: f32,
    /// The acting network's value estimate.
    pub value: f32,
    /// Reference policy over `legal`.
    pub reference: Vec<f32>,
    /// The seat's final Seeger-Fabian points (0 if thrown in): the reward.
    pub reward: f32,
    /// Whether the seat's party won (false if thrown in).
    pub won: bool,
}

/// One game of an iteration from the learner's side.
#[derive(Clone, Debug, PartialEq)]
pub struct Played {
    /// The deal seed (bookkeeping, never a model input).
    pub deal_seed: u64,
    /// Every action of the game in order, for replays.
    pub actions: Vec<Action>,
    /// The opponent kind.
    pub kind: PoolKind,
    /// The learner's seats.
    pub learner: [bool; 3],
    /// Final Seeger-Fabian points per seat (zeros if thrown in).
    pub points: [i32; 3],
    /// Whether the deal was thrown in.
    pub thrown_in: bool,
    /// The learner's decisions in decision order.
    pub steps: Vec<Step>,
}

/// Results against one pool kind in an iteration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KindLog {
    /// Games against this kind.
    pub games: usize,
    /// Learner seat-games and their summed Seeger-Fabian points.
    pub learner_seats: usize,
    /// Sum of the learner's points.
    pub learner_points: i64,
    /// Opponent seat-games.
    pub opponent_seats: usize,
    /// Sum of the opponents' points.
    pub opponent_points: i64,
}

impl KindLog {
    /// Learner's mean points per seat-game minus the opponents'.
    pub fn difference(&self) -> f64 {
        let mean = |s: i64, n: usize| s as f64 / n.max(1) as f64;
        mean(self.learner_points, self.learner_seats)
            - mean(self.opponent_points, self.opponent_seats)
    }
}

/// Log of one iteration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IterationLog {
    /// Iteration number (from 1).
    pub iteration: usize,
    /// Games played.
    pub games: usize,
    /// Games thrown in.
    pub thrown_in: usize,
    /// Learner decisions trained on.
    pub decisions: usize,
    /// Results per pool kind, in [`PoolKind::ALL`] order.
    pub pool: Vec<(PoolKind, KindLog)>,
    /// Mean reward of the learner's decisions.
    pub mean_reward: f64,
    /// Mean clipped surrogate loss over the Adam steps.
    pub policy_loss: f64,
    /// Root mean squared value error before the update, in points.
    pub value_rmse: f64,
    /// Mean policy entropy over the Adam steps.
    pub entropy: f64,
    /// Mean KL divergence from the reference to the learner (nats).
    pub reference_kl: f64,
    /// Share of decisions whose probability ratio was clipped.
    pub clipped: f64,
    /// Seconds playing games.
    pub play_seconds: f64,
    /// Seconds updating the network.
    pub update_seconds: f64,
}

/// A learner snapshot in the pool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Iterations completed when it was taken.
    pub iteration: usize,
    /// File name in `OUT/snapshots`.
    pub file: String,
    /// SHA-256 of the model file, hexadecimal.
    pub sha256: String,
}

/// Progress of a run, written as `checkpoint/state.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RlState {
    /// The recipe the run was started with.
    pub recipe: RlRecipe,
    /// One log per completed iteration.
    pub logs: Vec<IterationLog>,
    /// Every snapshot taken, oldest first.
    pub snapshots: Vec<Snapshot>,
    /// Directory of the latest checkpoint and the SHA-256 of its weights
    /// and optimizer files.
    pub checkpoint: Option<(String, [String; 2])>,
}

/// Why a run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RlStop {
    /// The iteration cap was reached.
    Iterations,
    /// The wall-time cap was reached.
    WallTime,
    /// The progress callback stopped the run.
    Stopped,
}

/// The result of [`run`].
#[derive(Debug)]
pub struct RlRun {
    /// The current learner as a model file.
    pub model: Vec<u8>,
    /// Progress.
    pub state: RlState,
    /// Why it stopped.
    pub stop: RlStop,
}

/// The trainable network of model file `model`, on `device`.
///
/// # Errors
///
/// If the file has no valid header or its weights do not load.
pub fn load_net(model: &[u8], device: &Device) -> io::Result<PolicyValueNet> {
    let header = read_header(model).map_err(|e| io::Error::other(format!("{e:?}")))?;
    from_record(header.shape, model[HEADER_BYTES..].to_vec(), device)
}

/// The recipe of a run starting from model file `start`.
///
/// # Errors
///
/// If `start` is not a supported model file.
pub fn recipe(start: &[u8], hyper: RlHyper) -> io::Result<RlRecipe> {
    let header = read_header(start).map_err(|e| io::Error::other(format!("{e:?}")))?;
    Ok(RlRecipe {
        rl_format: RL_FORMAT,
        encoder_version: header.encoder_version,
        phases: PHASES.bits(),
        start_sha256: hex(&sha256(start)),
        hidden: header.shape.hidden,
        layers: header.shape.layers,
        hyper,
    })
}

fn load_model(bytes: &[u8]) -> io::Result<Arc<NeuralModel>> {
    NeuralModel::load(bytes, &sha256(bytes))
        .map(Arc::new)
        .map_err(|e| io::Error::other(format!("{e:?}")))
}

/// The frozen models an iteration plays with.
struct Models {
    learner: Arc<NeuralModel>,
    reference: Arc<NeuralModel>,
    snapshots: Vec<Arc<NeuralModel>>,
}

fn rules_of_thumb(seed: u64) -> Opponent {
    Opponent {
        profile: Level::Easy.profile(),
        seed,
    }
}

fn pool_player(
    kind: PoolKind,
    models: &Models,
    snapshot: usize,
    seed: u64,
) -> Box<dyn Player + Send + Sync> {
    let classic = |profile: Profile| Box::new(Opponent { profile, seed });
    match kind {
        PoolKind::Easy => classic(Level::Easy.profile()),
        PoolKind::Medium => classic(Level::Medium.profile()),
        PoolKind::Strong => classic(Level::Strong.profile()),
        PoolKind::Reference | PoolKind::Snapshot => {
            let model = if kind == PoolKind::Snapshot && !models.snapshots.is_empty() {
                models.snapshots[snapshot % models.snapshots.len()].clone()
            } else {
                models.reference.clone()
            };
            Box::new(NeuralOpponent::new(Ok(model), rules_of_thumb(seed)))
        }
    }
}

/// Draws the opponent kind of a game from the pool weights.
fn draw(pool: &[(PoolKind, u32)], r: u64) -> PoolKind {
    let total: u64 = pool.iter().map(|&(_, w)| u64::from(w)).sum();
    let mut x = r % total;
    for &(kind, w) in pool {
        if x < u64::from(w) {
            return kind;
        }
        x -= u64::from(w);
    }
    unreachable!("x < total")
}

/// One learner decision from `view` alone: samples an action from the
/// learner's masked policy with the uniform draw `u` in [0, 1) and records
/// the reference policy. Reward and win are filled in after the game.
///
/// # Errors
///
/// If an action has no index or a model gives a non-finite policy.
pub fn learner_step(
    view: &PlayerView,
    learner: &NeuralModel,
    reference: &NeuralModel,
    u: f32,
) -> io::Result<Step> {
    let indices: Vec<usize> = view
        .legal_actions
        .iter()
        .map(|&a| action_index(a).ok_or_else(|| io::Error::other("action without index")))
        .collect::<io::Result<_>>()?;
    if learner.header().encoder_version != reference.header().encoder_version {
        return Err(io::Error::other(
            "learner and reference use different encoders",
        ));
    }
    let features = learner.features(view);
    let out = learner.forward_all(features.clone());
    let probs = masked_softmax(&out.logits, &indices)
        .ok_or_else(|| io::Error::other("learner gave a non-finite policy"))?;
    let reference = masked_softmax(&reference.forward_all(features.clone()).logits, &indices)
        .ok_or_else(|| io::Error::other("reference gave a non-finite policy"))?;
    let mut taken = probs.len() - 1;
    let mut acc = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        acc += p;
        if u < acc {
            taken = i;
            break;
        }
    }
    Ok(Step {
        seat: view.seat,
        action: view.legal_actions[taken],
        features,
        legal: indices.iter().map(|&i| i as u16).collect(),
        taken,
        log_prob: probs[taken].max(f32::MIN_POSITIVE).ln(),
        value: out.value,
        reference,
        reward: 0.0,
        won: false,
    })
}

/// Plays game `game` of iteration `iteration` (from 0) of a run with
/// `hyper` and returns it from the learner's side.
fn play_game(
    hyper: &RlHyper,
    models: &Models,
    iteration: usize,
    game: usize,
) -> io::Result<Played> {
    let key = |k: u64| mix(mix(mix(hyper.seed ^ k, iteration as u64), game as u64), 0);
    let learner = learner_seats(game);
    let kind = draw(&hyper.pool, key(POOL_KEY));
    let snapshot = (key(POOL_KEY) >> 32) as usize;
    let seed_of = |seat: Seat| mix(key(OPPONENT_KEY), seat.index() as u64);
    let players: Vec<Box<dyn Player + Send + Sync>> = Seat::ALL
        .into_iter()
        .map(|s| {
            if learner[s.index()] {
                Box::new(rules_of_thumb(seed_of(s))) as Box<dyn Player + Send + Sync>
            } else {
                pool_player(kind, models, snapshot, seed_of(s))
            }
        })
        .collect();
    let mut rngs = Seat::ALL.map(|s| SplitMix(mix(key(SAMPLE_KEY), s.index() as u64)));
    let deal_seed = key(DEAL_KEY);
    let mut game = Game::new(deal_seed, RULES);
    let mut steps: Vec<Step> = Vec::new();
    let mut actions = Vec::new();
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let action = if learner[seat.index()] && PHASES.contains(view.phase) {
            let u = rngs[seat.index()].unit();
            let step = learner_step(&view, &models.learner, &models.reference, u)?;
            let action = step.action;
            steps.push(step);
            action
        } else {
            players[seat.index()]
                .decide(&view, None)
                .ok_or_else(|| io::Error::other("a player found no action"))?
                .action
        };
        game.apply(action)
            .map_err(|e| io::Error::other(format!("illegal action: {e:?}")))?;
        actions.push(action);
    }
    let (points, thrown_in, winners) = match game.result() {
        Some(r) => {
            let declarer = r.outcome.declarer;
            let won = Seat::ALL.map(|s| (s == declarer) == r.won);
            (r.seeger_fabian(), false, won)
        }
        None => ([0; 3], true, [false; 3]),
    };
    Ok(Played {
        deal_seed,
        actions,
        kind,
        learner,
        points,
        thrown_in,
        steps: steps
            .into_iter()
            .map(|mut s| {
                s.reward = points[s.seat.index()] as f32;
                s.won = winners[s.seat.index()];
                s
            })
            .collect(),
    })
}

/// Plays the games of iteration `iteration` with `limits.workers` games in
/// parallel and returns them in game order.
fn play_iteration(
    hyper: &RlHyper,
    models: &Models,
    iteration: usize,
    limits: &RlLimits,
) -> io::Result<Vec<Played>> {
    let next = AtomicUsize::new(0);
    let mut results: Vec<(usize, io::Result<Played>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..limits.workers.get())
            .map(|_| {
                scope.spawn(|| {
                    skat_analysis::with_max_threads(limits.threads(), || {
                        let mut done = Vec::new();
                        loop {
                            let g = next.fetch_add(1, Ordering::Relaxed);
                            if g >= hyper.games {
                                break done;
                            }
                            done.push((g, play_game(hyper, models, iteration, g)));
                        }
                    })
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("game worker"))
            .collect()
    });
    results.sort_by_key(|(g, _)| *g);
    results.into_iter().map(|(_, r)| r).collect()
}

/// Plays one game of iteration `iteration` of a run with `hyper`, the
/// learner being `learner` and the reference `reference` (tests and
/// inspection; a run plays its games through [`run`]).
///
/// # Errors
///
/// If a model gives an unusable output or a player no action.
pub fn play_one(
    hyper: &RlHyper,
    learner: &[u8],
    reference: &[u8],
    iteration: usize,
    game: usize,
) -> io::Result<Played> {
    let models = Models {
        learner: load_model(learner)?,
        reference: load_model(reference)?,
        snapshots: Vec::new(),
    };
    play_game(hyper, &models, iteration, game)
}

/// Statistics of an update.
#[derive(Default)]
struct UpdateStats {
    policy: f64,
    entropy: f64,
    reference_kl: f64,
    clipped: f64,
    steps: usize,
}

/// Mini-batch tensors of learner decisions.
struct StepBatch {
    features: Tensor<2>,
    mask: Tensor<2>,
    taken: Tensor<2>,
    reference: Tensor<2>,
    reference_entropy: Tensor<2>,
    log_prob: Tensor<2>,
    advantage: Tensor<2>,
    reward: Tensor<2>,
    won: Tensor<2>,
}

fn step_batch(steps: &[&Step], advantages: &[f32], device: &Device) -> StepBatch {
    let n = steps.len();
    let inputs = steps[0].features.len();
    let mut features = Vec::with_capacity(n * inputs);
    let mut mask = vec![-1e9f32; n * ACTIONS];
    let mut taken = vec![0.0f32; n * ACTIONS];
    let mut reference = vec![0.0f32; n * ACTIONS];
    let mut reference_entropy = Vec::with_capacity(n);
    for (b, s) in steps.iter().enumerate() {
        features.extend_from_slice(&s.features);
        let mut h = 0.0;
        for (i, &a) in s.legal.iter().enumerate() {
            let at = b * ACTIONS + a as usize;
            mask[at] = 0.0;
            reference[at] = s.reference[i];
            if s.reference[i] > 0.0 {
                h -= s.reference[i] * s.reference[i].ln();
            }
        }
        taken[b * ACTIONS + s.legal[s.taken] as usize] = 1.0;
        reference_entropy.push(h);
    }
    let column = |v: Vec<f32>| tensor(v, [n, 1], device);
    StepBatch {
        features: tensor(features, [n, inputs], device),
        mask: tensor(mask, [n, ACTIONS], device),
        taken: tensor(taken, [n, ACTIONS], device),
        reference: tensor(reference, [n, ACTIONS], device),
        reference_entropy: column(reference_entropy),
        log_prob: column(steps.iter().map(|s| s.log_prob).collect()),
        advantage: column(advantages.to_vec()),
        reward: column(steps.iter().map(|s| s.reward).collect()),
        won: column(steps.iter().map(|s| f32::from(u8::from(s.won))).collect()),
    }
}

fn scalar(t: Tensor<1>) -> f64 {
    f64::from(t.into_scalar::<f32>())
}

/// The loss of a mini-batch and its statistics.
fn loss(net: &PolicyValueNet, b: StepBatch, hyper: &RlHyper) -> (Tensor<1>, [f64; 4]) {
    let o = net.forward_all(b.features);
    let log_p = log_softmax(o.policy + b.mask, 1);
    let taken_log_p = (log_p.clone() * b.taken.clone()).sum_dim(1);
    let ratio = (taken_log_p - b.log_prob).exp();
    let unclipped = ratio.clone() * b.advantage.clone();
    let clipped = ratio.clone().clamp(1.0 - hyper.clip, 1.0 + hyper.clip) * b.advantage;
    let surrogate = unclipped.min_pair(clipped).mean().neg();
    let entropy = (log_p.clone().exp() * log_p.clone())
        .sum_dim(1)
        .neg()
        .mean();
    let cross = (b.reference * log_p).sum_dim(1).neg();
    let kl = (cross.clone() - b.reference_entropy).mean();
    let value = ((o.value - b.reward.clone()) / VALUE_SCALE)
        .powi_scalar(2)
        .mean();
    let score = (o.score * b.taken.clone()).sum_dim(1);
    let score = ((score - b.reward) / VALUE_SCALE).powi_scalar(2).mean();
    let x = (o.win * b.taken).sum_dim(1);
    let win =
        (x.clone().clamp_min(0.0) - x.clone() * b.won + (x.abs().neg().exp() + 1.0).log()).mean();
    let out_of_range = {
        let r = ratio.into_data().try_to_vec::<f32>().expect("f32");
        r.iter().filter(|&&r| (r - 1.0).abs() > hyper.clip).count() as f64 / r.len().max(1) as f64
    };
    let stats = [
        scalar(surrogate.clone()),
        scalar(entropy.clone()),
        scalar(kl),
        out_of_range,
    ];
    let total = surrogate
        + value * hyper.value_weight
        + score * hyper.score_weight
        + win * hyper.win_weight
        - entropy * hyper.entropy_weight
        + cross.mean() * hyper.reference_weight;
    (total, stats)
}

/// One PPO update over `steps`. Advantages: reward minus the acting
/// value, normalised over the iteration.
fn update(
    mut net: PolicyValueNet,
    optim: &mut ModuleOptimizer,
    steps: &[Step],
    hyper: &RlHyper,
    iteration: usize,
    device: &Device,
) -> (PolicyValueNet, UpdateStats) {
    let raw: Vec<f32> = steps
        .iter()
        .map(|s| (s.reward - s.value) / VALUE_SCALE)
        .collect();
    let n = raw.len().max(1) as f32;
    let mean = raw.iter().sum::<f32>() / n;
    let std = (raw.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / n).sqrt();
    let advantages: Vec<f32> = raw.iter().map(|a| (a - mean) / (std + 1e-6)).collect();
    let mut stats = UpdateStats::default();
    for pass in 0..hyper.passes {
        let mut rng = SplitMix(mix(
            mix(hyper.seed ^ ORDER_KEY, iteration as u64),
            pass as u64,
        ));
        let mut order: Vec<usize> = (0..steps.len()).collect();
        for i in (1..order.len()).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for chunk in order.chunks(hyper.batch.max(1)) {
            let batch: Vec<&Step> = chunk.iter().map(|&i| &steps[i]).collect();
            let adv: Vec<f32> = chunk.iter().map(|&i| advantages[i]).collect();
            let (total, [policy, entropy, kl, clipped]) =
                loss(&net, step_batch(&batch, &adv, device), hyper);
            stats.policy += policy;
            stats.entropy += entropy;
            stats.reference_kl += kl;
            stats.clipped += clipped;
            stats.steps += 1;
            let grads = GradientsParams::from_grads(total.backward(), &net);
            net = optim.step(hyper.learning_rate, net, grads);
        }
    }
    (net, stats)
}

/// One PPO update of model file `model` over `steps` with fresh Adam state
/// on `device` (an autodiff device), as iteration `iteration` of a run with
/// `hyper` would make it; returns the updated model file.
///
/// # Errors
///
/// If `model` is not a valid model file.
pub fn update_model(
    model: &[u8],
    steps: &[Step],
    hyper: &RlHyper,
    iteration: usize,
    device: &Device,
) -> io::Result<Vec<u8>> {
    let header = read_header(model).map_err(|e| io::Error::other(format!("{e:?}")))?;
    if steps
        .iter()
        .any(|s| s.features.len() != header.shape.inputs)
    {
        return Err(io::Error::other(
            "step features do not match the model encoder",
        ));
    }
    let mut optim = AdamConfig::new().init();
    let (net, _) = update(
        load_net(model, device)?,
        &mut optim,
        steps,
        hyper,
        iteration,
        device,
    );
    rl_model_file(net.valid(), &header)
}

fn rl_model_file(net: PolicyValueNet, start: &ModelHeader) -> io::Result<Vec<u8>> {
    let header = ModelHeader {
        phases: PHASES,
        ..*start
    };
    write_model(&header, net).map_err(|e| io::Error::other(format!("{e:?}")))
}

fn checkpoint_dir(out: &Path) -> PathBuf {
    out.join("checkpoint")
}

fn snapshot_dir(out: &Path) -> PathBuf {
    out.join("snapshots")
}

fn write_state(dir: &Path, state: &RlState) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    bytes.push(b'\n');
    write_atomic(&dir.join("state.json"), &bytes)
}

fn read_verified(path: &Path, expected: &str) -> io::Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    if hex(&sha256(&bytes)) != expected {
        return Err(io::Error::other(format!(
            "{} does not match its hash",
            path.display()
        )));
    }
    Ok(bytes)
}

/// Runs (or resumes) the self-play run in `out` with `recipe`, starting
/// from model file `start` (also the reference), training on `device` (an
/// autodiff device). After every iteration it writes a checkpoint and
/// passes the log to `progress`, which may stop the run by returning
/// `false`.
///
/// # Errors
///
/// On IO errors, a run directory with another recipe, a checkpoint or
/// snapshot that does not match its hash, a start model that does not
/// match the recipe, or a model giving unusable outputs.
pub fn run(
    out: &Path,
    recipe: &RlRecipe,
    start: &[u8],
    device: &Device,
    limits: RlLimits,
    mut progress: impl FnMut(&IterationLog) -> bool,
) -> io::Result<RlRun> {
    let began = Instant::now();
    if hex(&sha256(start)) != recipe.start_sha256 {
        return Err(io::Error::other(
            "the start model does not match the recipe",
        ));
    }
    let hyper = &recipe.hyper;
    let header = read_header(start).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let shape = header.shape;
    let dir = checkpoint_dir(out);
    let snaps = snapshot_dir(out);
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(&snaps)?;
    let adam = AdamConfig::new();
    let reference = load_model(start)?;

    let (mut net, mut optim, mut state) = match fs::read(dir.join("state.json")) {
        Ok(bytes) => {
            let state: RlState = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
            if state.recipe != *recipe {
                return Err(io::Error::other(format!(
                    "{} holds a run with another recipe",
                    out.display()
                )));
            }
            let (net, optim) = match &state.checkpoint {
                Some((name, [w, o])) => {
                    let weights = read_verified(&dir.join(name).join("weights.bpk"), w)?;
                    let optimizer = read_verified(&dir.join(name).join("optimizer.bpk"), o)?;
                    let optim: ModuleOptimizer = adam
                        .init()
                        .from_bytes(Bytes::from_bytes_vec(optimizer))
                        .map_err(io::Error::other)?;
                    (from_record(shape, weights, device)?, optim)
                }
                None => (load_net(start, device)?, adam.init()),
            };
            (net, optim, state)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let state = RlState {
                recipe: recipe.clone(),
                logs: Vec::new(),
                snapshots: Vec::new(),
                checkpoint: None,
            };
            (load_net(start, device)?, adam.init(), state)
        }
        Err(e) => return Err(e),
    };
    let mut pool: Vec<Arc<NeuralModel>> = Vec::new();
    for s in &state.snapshots {
        pool.push(load_model(&read_verified(
            &snaps.join(&s.file),
            &s.sha256,
        )?)?);
    }

    let stop = loop {
        let iteration = state.logs.len();
        if iteration >= limits.iterations {
            break RlStop::Iterations;
        }
        if began.elapsed() >= limits.wall_time {
            break RlStop::WallTime;
        }
        let current = rl_model_file(net.valid(), &header)?;
        if iteration > 0
            && hyper.snapshot_every > 0
            && iteration.is_multiple_of(hyper.snapshot_every)
            && state
                .snapshots
                .last()
                .is_none_or(|s| s.iteration != iteration)
        {
            let file = format!("iter-{iteration:05}.gcnn");
            write_atomic(&snaps.join(&file), &current)?;
            state.snapshots.push(Snapshot {
                iteration,
                file,
                sha256: hex(&sha256(&current)),
            });
            pool.push(load_model(&current)?);
        }
        let keep = hyper.pool_snapshots.min(pool.len());
        let models = Models {
            learner: load_model(&current)?,
            reference: reference.clone(),
            snapshots: pool[pool.len() - keep..].to_vec(),
        };
        let played_at = Instant::now();
        let games = play_iteration(hyper, &models, iteration, &limits)?;
        let play_seconds = played_at.elapsed().as_secs_f64();

        let mut kinds: Vec<(PoolKind, KindLog)> = PoolKind::ALL
            .iter()
            .map(|&k| (k, KindLog::default()))
            .collect();
        let mut steps = Vec::new();
        let mut thrown_in = 0;
        for g in games {
            thrown_in += usize::from(g.thrown_in);
            let log = &mut kinds
                .iter_mut()
                .find(|(k, _)| *k == g.kind)
                .expect("every kind")
                .1;
            log.games += 1;
            for seat in 0..3 {
                if g.learner[seat] {
                    log.learner_seats += 1;
                    log.learner_points += i64::from(g.points[seat]);
                } else {
                    log.opponent_seats += 1;
                    log.opponent_points += i64::from(g.points[seat]);
                }
            }
            steps.extend(g.steps);
        }
        let n = steps.len().max(1) as f64;
        let mean_reward = steps.iter().map(|s| f64::from(s.reward)).sum::<f64>() / n;
        let value_rmse = (steps
            .iter()
            .map(|s| f64::from(s.reward - s.value).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        let updated_at = Instant::now();
        let (next, stats) = update(net, &mut optim, &steps, hyper, iteration, device);
        net = next;
        let per = stats.steps.max(1) as f64;
        let log = IterationLog {
            iteration: iteration + 1,
            games: hyper.games,
            thrown_in,
            decisions: steps.len(),
            pool: kinds.into_iter().filter(|(_, l)| l.games > 0).collect(),
            mean_reward,
            policy_loss: stats.policy / per,
            value_rmse,
            entropy: stats.entropy / per,
            reference_kl: stats.reference_kl / per,
            clipped: stats.clipped / per,
            play_seconds,
            update_seconds: updated_at.elapsed().as_secs_f64(),
        };
        state.logs.push(log);

        // Each iteration's files go into their own directory; `state.json`,
        // written last and atomically, points at it with their hashes.
        let name = format!("iter-{:05}", iteration + 1);
        let weights = record_bytes(&net);
        let optimizer = optim.into_bytes().map_err(io::Error::other)?.to_vec();
        fs::create_dir_all(dir.join(&name))?;
        write_atomic(&dir.join(&name).join("weights.bpk"), &weights)?;
        write_atomic(&dir.join(&name).join("optimizer.bpk"), &optimizer)?;
        state.checkpoint = Some((
            name.clone(),
            [hex(&sha256(&weights)), hex(&sha256(&optimizer))],
        ));
        write_state(&dir, &state)?;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.file_name() != name.as_str() {
                fs::remove_dir_all(entry.path())?;
            }
        }
        if !progress(state.logs.last().expect("just pushed")) {
            break RlStop::Stopped;
        }
    };
    Ok(RlRun {
        model: rl_model_file(net.valid(), &header)?,
        state,
        stop,
    })
}
