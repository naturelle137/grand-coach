// SPDX-License-Identifier: EUPL-1.2

//! Supervised training of the card-play network (#73) on teacher self-play
//! records (#70).
//!
//! Examples are the card-play decisions (phase Spiel, Kontra/Re included)
//! of the training split. Targets:
//!
//! - **Policy**: the teacher's chosen action, optionally mixed with a soft
//!   distribution from the teacher's complete option values
//!   (`softmax(expected_score / soft_temperature)`, share `soft_weight`).
//! - **Value**: the seat's final Seeger-Fabian points, mixed with the
//!   expected score of the chosen option when the teacher's analysis is
//!   complete (share `value_mix`).
//! - **Options** (when the teacher's analysis is complete): for every legal
//!   action the teacher's expected Seeger-Fabian points (squared error,
//!   weight `score_weight`) and win chance (cross-entropy on the win logit,
//!   weight `win_weight`). Weights 0 train the policy and value only.
//!
//! The only model input is the encoded [`PlayerView`](skat_core::PlayerView)
//! of the record; final points and option values are targets.
//!
//! Training is reproducible on the CPU: the initial weights and the order
//! of the examples in each epoch come from the recipe's seed (SplitMix64),
//! never from a device's random state. After every epoch the run directory
//! gets a checkpoint (weights, Adam state, progress); a restart with the
//! same recipe resumes it and gives the same model as an uninterrupted
//! run. No epoch starts after the wall-time cap. The model with the lowest
//! validation loss is kept.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use burn::module::Module;
use burn::optim::{AdamConfig, GradientsParams, ModuleOptimizer};
use burn::store::ModuleRecord;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Bytes, Device, Tensor, TensorData};
use serde::{Deserialize, Serialize};
use skat_ai::Status;
use skat_core::Phase;
use skat_nn::{
    ACTIONS, ENCODER_VERSION, FEATURES, ModelHeader, NetShape, Phases, PolicyValueNet,
    action_index, encode, sha256, write_model,
};

use crate::selfplay::{DecisionKind, DecisionRecord, Split};
use crate::shards::{Manifest, read_manifest, read_shard};

/// Version of the training recipe and run-directory layout.
pub const TRAIN_FORMAT: u32 = 2;

/// Phases the trained model decides; everything else stays Classic.
pub const PHASES: Phases = Phases::SPIEL;

/// Scale of the value target in the loss (points per unit).
pub const VALUE_SCALE: f32 = 60.0;

/// Rows per forward pass when evaluating.
const EVAL_BATCH: usize = 4096;

/// Hyperparameters of a training run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hyper {
    /// Width of both hidden layers.
    pub hidden: usize,
    /// Examples per Adam step.
    pub batch: usize,
    /// Adam learning rate.
    pub learning_rate: f64,
    /// Maximum number of epochs.
    pub epochs: usize,
    /// Weight of the value loss against the policy loss.
    pub value_weight: f32,
    /// Share of the teacher's option value in the value target (0: final
    /// points only).
    pub value_mix: f32,
    /// Share of the soft teacher distribution in the policy target (0: the
    /// chosen action only).
    pub soft_weight: f32,
    /// Temperature of the soft teacher distribution in Seeger-Fabian
    /// points.
    pub soft_temperature: f32,
    /// Weight of the option score loss (expected points per action).
    pub score_weight: f32,
    /// Weight of the option win-chance loss.
    pub win_weight: f32,
    /// Seed of the initial weights and the example order.
    pub seed: u64,
}

impl Default for Hyper {
    fn default() -> Hyper {
        Hyper {
            hidden: 256,
            batch: 256,
            learning_rate: 1e-3,
            epochs: 20,
            value_weight: 1.0,
            value_mix: 0.5,
            soft_weight: 0.0,
            soft_temperature: 5.0,
            score_weight: 1.0,
            win_weight: 1.0,
            seed: 0,
        }
    }
}

/// One self-play run used as training data, identified by hashes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataSource {
    /// The run's manifest.
    pub manifest: Manifest,
    /// SHA-256 of the canonical JSON of the manifest, hexadecimal.
    pub manifest_sha256: String,
    /// Complete shards used.
    pub shards: u64,
    /// SHA-256 over the shard hashes in shard order, hexadecimal: changes
    /// with any record.
    pub data_sha256: String,
}

/// Everything that determines the trained model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// [`TRAIN_FORMAT`].
    pub train_format: u32,
    /// Encoder version of the features.
    pub encoder_version: u16,
    /// Phases of the examples and the model ([`PHASES`]).
    pub phases: u8,
    /// Training data.
    pub data: Vec<DataSource>,
    /// Hyperparameters.
    pub hyper: Hyper,
}

/// Metrics of a model on one split.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    /// Examples.
    pub examples: usize,
    /// Cross-entropy of the policy against its target.
    pub policy_loss: f64,
    /// Share of examples where the most probable legal action is the
    /// teacher's.
    pub agreement: f64,
    /// Root mean squared error of the value against its target, in
    /// points.
    pub value_rmse: f64,
    /// Legal actions with a teacher option value.
    pub options: usize,
    /// Root mean squared error of the option scores against the teacher's,
    /// in points.
    pub score_rmse: f64,
    /// Cross-entropy of the option win chances against the teacher's.
    pub win_loss: f64,
    /// Share of examples with teacher options where the action with the
    /// highest predicted score is the teacher's.
    pub score_agreement: f64,
    /// The training loss: policy loss plus the weighted value, score and
    /// win losses.
    pub loss: f64,
}

/// What happened in one epoch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpochLog {
    /// Epoch number, from 1.
    pub epoch: usize,
    /// Mean training loss over the epoch's batches.
    pub train_loss: f64,
    /// Validation metrics after the epoch.
    pub validation: Metrics,
    /// Wall time of the epoch in seconds (informational).
    pub seconds: f64,
}

/// Progress of a run, written after every epoch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// The recipe; a restart must use the same.
    pub recipe: Recipe,
    /// Completed epochs.
    pub logs: Vec<EpochLog>,
    /// Epoch of the best model so far (lowest validation loss).
    pub best_epoch: usize,
    /// Files of the last completed epoch.
    pub checkpoint: Option<Checkpoint>,
}

/// Where the files of a completed epoch are, with their SHA-256 hashes
/// (weights, Adam state, best weights so far).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Subdirectory of `checkpoint/`.
    pub dir: String,
    /// Hashes of `weights.bpk`, `optimizer.bpk`, `best.bpk`, hexadecimal.
    pub sha256: Vec<String>,
}

/// The verified files of the checkpoint `state` points at.
fn read_checkpoint(dir: &Path, state: &State) -> io::Result<[Vec<u8>; 3]> {
    let checkpoint = state
        .checkpoint
        .as_ref()
        .ok_or_else(|| io::Error::other("state without checkpoint"))?;
    let read = |i: usize, file: &str| -> io::Result<Vec<u8>> {
        let bytes = fs::read(dir.join(&checkpoint.dir).join(file))?;
        if checkpoint.sha256.get(i) != Some(&hex(&sha256(&bytes))) {
            return Err(io::Error::other(format!(
                "checkpoint file {file} is corrupt"
            )));
        }
        Ok(bytes)
    };
    Ok([
        read(0, "weights.bpk")?,
        read(1, "optimizer.bpk")?,
        read(2, "best.bpk")?,
    ])
}

/// The result of a finished run (`train.json`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainManifest {
    /// The recipe.
    pub recipe: Recipe,
    /// Engine commit of the trainer (`git describe`).
    pub engine_commit: String,
    /// Device the run trained on (`cpu` is the reproducible recipe).
    pub device: String,
    /// Examples per split and decision kind.
    pub coverage: Vec<Coverage>,
    /// Epoch logs.
    pub logs: Vec<EpochLog>,
    /// Epoch of the shipped model.
    pub best_epoch: usize,
    /// Metrics of the shipped model on the training split.
    pub train: Metrics,
    /// … on the validation split.
    pub validation: Metrics,
    /// … on the test split.
    pub test: Metrics,
    /// Size of the model file in bytes.
    pub model_bytes: usize,
    /// SHA-256 of the model file, hexadecimal.
    pub model_sha256: String,
    /// Why the run stopped: `finished` or `wall-time`.
    pub stop: String,
}

/// Examples of one decision kind in one split.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Split.
    pub split: Split,
    /// Kind.
    pub kind: DecisionKind,
    /// Examples.
    pub examples: usize,
}

/// Encoded examples, row-major and compact: legal actions and policy
/// targets are stored per legal action.
#[derive(Clone, Debug, Default)]
pub struct Examples {
    /// Feature rows, [`FEATURES`] each.
    pub features: Vec<f32>,
    /// Start of each row's legal actions in `legal`; one more than rows.
    pub offsets: Vec<usize>,
    /// Action indices of the legal actions.
    pub legal: Vec<u32>,
    /// Policy target per legal action (sums to 1 per row).
    pub target: Vec<f32>,
    /// Index of the teacher's action within the row's legal actions.
    pub chosen: Vec<u32>,
    /// Value target per row.
    pub value: Vec<f32>,
    /// `true` if the row has teacher option values.
    pub has_options: Vec<bool>,
    /// Teacher's expected points per legal action (0 without options).
    pub option_score: Vec<f32>,
    /// Teacher's win chance per legal action (0 without options).
    pub option_win: Vec<f32>,
}

/// Tensors of a batch.
struct Batch {
    features: Tensor<2>,
    /// 0 for legal, -1e9 for illegal actions.
    mask: Tensor<2>,
    policy: Tensor<2>,
    value: Tensor<2>,
    /// 1 for legal actions with a teacher option value, else 0.
    option_mask: Tensor<2>,
    score: Tensor<2>,
    win: Tensor<2>,
    /// Number of ones in `option_mask`.
    options: usize,
}

impl Examples {
    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.value.len()
    }

    fn new() -> Examples {
        Examples {
            offsets: vec![0],
            ..Examples::default()
        }
    }

    /// Adds `record` if it is a card-play decision.
    fn push(&mut self, r: &DecisionRecord, hyper: &Hyper) -> bool {
        if r.view.phase != Phase::Spiel || !PHASES.contains(r.view.phase) {
            return false;
        }
        let legal: Vec<u32> = r
            .view
            .legal_actions
            .iter()
            .map(|&a| action_index(a).expect("checked records have indices") as u32)
            .collect();
        let chosen = r
            .view
            .legal_actions
            .iter()
            .position(|&a| a == r.action)
            .expect("checked records choose a legal action");
        let complete = r.teacher.status == Some(Status::Complete)
            && r.teacher.options.len() == legal.len()
            && r.teacher.options.iter().all(|o| o.estimate.is_some());
        let wins: Vec<f64> = if complete {
            r.teacher
                .options
                .iter()
                .map(|o| o.estimate.expect("complete").win_chance)
                .collect()
        } else {
            vec![0.0; legal.len()]
        };
        let scores: Vec<f64> = if complete {
            r.teacher
                .options
                .iter()
                .map(|o| o.estimate.expect("complete").expected_score)
                .collect()
        } else {
            Vec::new()
        };
        let mut target = vec![0.0f32; legal.len()];
        target[chosen] = 1.0;
        if complete && hyper.soft_weight > 0.0 {
            let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let t = f64::from(hyper.soft_temperature.max(1e-3));
            let exp: Vec<f64> = scores.iter().map(|s| ((s - max) / t).exp()).collect();
            let total: f64 = exp.iter().sum();
            let w = hyper.soft_weight;
            for (x, e) in target.iter_mut().zip(exp) {
                *x = (1.0 - w) * *x + w * (e / total) as f32;
            }
        }
        let final_points = r.final_points as f32;
        let value = if complete {
            let m = hyper.value_mix;
            (1.0 - m) * final_points + m * scores[chosen] as f32
        } else {
            final_points
        };
        self.features.extend(encode(&r.view));
        self.has_options.push(complete);
        if complete {
            self.option_score.extend(scores.iter().map(|&x| x as f32));
        } else {
            self.option_score
                .extend(std::iter::repeat_n(0.0, legal.len()));
        }
        self.option_win.extend(wins.iter().map(|&x| x as f32));
        self.legal.extend(legal);
        self.target.extend(target);
        self.offsets.push(self.legal.len());
        self.chosen.push(chosen as u32);
        self.value.push(value);
        true
    }

    /// Tensors for the rows `rows`.
    fn batch(&self, rows: &[usize], device: &Device) -> Batch {
        let n = rows.len();
        let mut features = Vec::with_capacity(n * FEATURES);
        let mut mask = vec![-1e9f32; n * ACTIONS];
        let mut target = vec![0.0f32; n * ACTIONS];
        let mut option_mask = vec![0.0f32; n * ACTIONS];
        let mut score = vec![0.0f32; n * ACTIONS];
        let mut win = vec![0.0f32; n * ACTIONS];
        let mut value = Vec::with_capacity(n);
        let mut options = 0;
        for (b, &r) in rows.iter().enumerate() {
            features.extend_from_slice(&self.features[r * FEATURES..][..FEATURES]);
            for i in self.offsets[r]..self.offsets[r + 1] {
                let a = b * ACTIONS + self.legal[i] as usize;
                mask[a] = 0.0;
                target[a] = self.target[i];
                if self.has_options[r] {
                    option_mask[a] = 1.0;
                    score[a] = self.option_score[i];
                    win[a] = self.option_win[i];
                    options += 1;
                }
            }
            value.push(self.value[r]);
        }
        Batch {
            features: tensor(features, [n, FEATURES], device),
            mask: tensor(mask, [n, ACTIONS], device),
            policy: tensor(target, [n, ACTIONS], device),
            value: tensor(value, [n, 1], device),
            option_mask: tensor(option_mask, [n, ACTIONS], device),
            score: tensor(score, [n, ACTIONS], device),
            win: tensor(win, [n, ACTIONS], device),
            options,
        }
    }
}

fn tensor(values: Vec<f32>, shape: [usize; 2], device: &Device) -> Tensor<2> {
    Tensor::from_data(TensorData::new(values, shape), device)
}

/// Training, validation and test examples with their coverage.
#[derive(Clone, Debug, Default)]
pub struct Data {
    /// Training split.
    pub train: Examples,
    /// Validation split.
    pub validation: Examples,
    /// Test split.
    pub test: Examples,
    /// Examples per split and kind.
    pub coverage: Vec<Coverage>,
    /// The sources, by hash.
    pub sources: Vec<DataSource>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads every complete shard of the runs in `dirs` and encodes their
/// card-play decisions with the targets of `hyper`. Incomplete shards are
/// skipped (and counted in the source); corrupt ones are errors.
pub fn load(dirs: &[PathBuf], hyper: &Hyper) -> io::Result<Data> {
    let mut data = Data {
        train: Examples::new(),
        validation: Examples::new(),
        test: Examples::new(),
        ..Data::default()
    };
    let mut counts = std::collections::BTreeMap::<(u8, DecisionKind), usize>::new();
    for dir in dirs {
        let manifest = read_manifest(dir)?;
        let canonical = serde_json::to_vec(&manifest).map_err(io::Error::other)?;
        let mut hashes = Vec::new();
        let mut shards = 0;
        for index in 0..manifest.shards() {
            let records = match read_shard(dir, index) {
                Ok(r) => r,
                Err(crate::shards::ShardError::Incomplete) => continue,
                Err(e) => return Err(io::Error::other(format!("{}: {e}", dir.display()))),
            };
            let shard = fs::read(crate::shards::shard_path(dir, index))?;
            hashes.extend_from_slice(&sha256(&shard));
            shards += 1;
            for r in &records {
                crate::selfplay::check_record(r).map_err(io::Error::other)?;
                let (examples, split) = match r.split {
                    Split::Train => (&mut data.train, 0),
                    Split::Validation => (&mut data.validation, 1),
                    Split::Test => (&mut data.test, 2),
                };
                if examples.push(r, hyper) {
                    *counts
                        .entry((split, DecisionKind::of(&r.view)))
                        .or_default() += 1;
                }
            }
        }
        data.sources.push(DataSource {
            manifest,
            manifest_sha256: hex(&sha256(&canonical)),
            shards,
            data_sha256: hex(&sha256(&hashes)),
        });
    }
    data.coverage = counts
        .into_iter()
        .map(|((split, kind), examples)| Coverage {
            split: [Split::Train, Split::Validation, Split::Test][split as usize],
            kind,
            examples,
        })
        .collect();
    Ok(data)
}

/// SplitMix64 stream for the initial weights and the example order.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

const INIT_KEY: u64 = 0x696e_6974;
const ORDER_KEY: u64 = 0x6f72_6465;

/// The initial network: every weight and bias uniform in
/// ±1/sqrt(fan_in) (Burn's and PyTorch's default bound), drawn from the
/// seed.
pub fn initial(shape: NetShape, seed: u64) -> PolicyValueNet {
    let mut rng = SplitMix(seed ^ INIT_KEY);
    let layers = [
        (shape.inputs, shape.hidden),
        (shape.hidden, shape.hidden),
        (shape.hidden, shape.actions),
        (shape.hidden, 1),
        (shape.hidden, shape.actions),
        (shape.hidden, shape.actions),
    ];
    // Values in the order `from_weights` consumes them: per layer the
    // weight matrix, then the bias.
    let mut values = Vec::new();
    for (inputs, outputs) in layers {
        let bound = 1.0 / (inputs as f32).sqrt();
        for _ in 0..(inputs + 1) * outputs {
            values.push((rng.unit() * 2.0 - 1.0) * bound);
        }
    }
    let mut it = values.into_iter();
    PolicyValueNet::from_weights(shape, &Device::flex(), || it.next().expect("enough values"))
}

/// The order of the training rows in epoch `epoch` (from 0).
fn order(rows: usize, seed: u64, epoch: usize) -> Vec<usize> {
    let mut rng = SplitMix(seed ^ ORDER_KEY ^ (epoch as u64).wrapping_mul(0x1_0000_0001));
    let mut order: Vec<usize> = (0..rows).collect();
    for i in (1..rows).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

fn record_bytes(net: &PolicyValueNet) -> Vec<u8> {
    net.clone()
        .into_record()
        .into_bytes()
        .expect("weights serialise")
        .to_vec()
}

fn from_record(shape: NetShape, bytes: Vec<u8>, device: &Device) -> io::Result<PolicyValueNet> {
    let record =
        ModuleRecord::from_bytes(Bytes::from_bytes_vec(bytes)).map_err(io::Error::other)?;
    PolicyValueNet::new(shape, device)
        .try_load_record(record)
        .map_err(|e| io::Error::other(e.to_string()))
}

/// Losses of a batch, summed per row (`[n, 1]`), with the raw outputs.
struct Losses {
    /// Policy cross-entropy.
    policy: Tensor<2>,
    /// Scaled squared value error.
    value: Tensor<2>,
    /// Scaled squared option score errors, summed over the row's options.
    score: Tensor<2>,
    /// Option win cross-entropies, summed over the row's options.
    win: Tensor<2>,
    /// Masked policy logits.
    logits: Tensor<2>,
    /// Predicted option scores.
    scores: Tensor<2>,
    /// Option entries in the batch.
    options: usize,
}

impl Losses {
    /// The training loss of the batch.
    fn total(self, hyper: &Hyper) -> Tensor<1> {
        let per_option = 1.0 / self.options.max(1) as f32;
        let mut loss = self.policy.mean() + self.value.mean() * hyper.value_weight;
        if hyper.score_weight > 0.0 {
            loss = loss + self.score.sum() * (hyper.score_weight * per_option);
        }
        if hyper.win_weight > 0.0 {
            loss = loss + self.win.sum() * (hyper.win_weight * per_option);
        }
        loss
    }
}

fn losses(net: &PolicyValueNet, b: Batch) -> Losses {
    let o = net.forward_all(b.features);
    let logits = o.policy + b.mask;
    let policy = (log_softmax(logits.clone(), 1) * b.policy).sum_dim(1).neg();
    let value = ((o.value - b.value) / VALUE_SCALE).powi_scalar(2);
    let score = (((o.score.clone() - b.score) / VALUE_SCALE).powi_scalar(2)
        * b.option_mask.clone())
    .sum_dim(1);
    // Binary cross-entropy with logits, numerically stable:
    // max(x, 0) - x t + ln(1 + e^-|x|).
    let x = o.win;
    let bce = x.clone().clamp_min(0.0) - x.clone() * b.win + (x.abs().neg().exp() + 1.0).log();
    let win = (bce * b.option_mask).sum_dim(1);
    Losses {
        policy,
        value,
        score,
        win,
        logits,
        scores: o.score,
        options: b.options,
    }
}

fn sum(t: Tensor<2>) -> f64 {
    t.into_data()
        .try_to_vec::<f32>()
        .expect("f32")
        .iter()
        .map(|&x| f64::from(x))
        .sum()
}

/// Index within `legal` of the highest entry of `row` (ties: first).
fn best(row: &[f32], legal: &[u32]) -> usize {
    let mut best = 0;
    for (i, &a) in legal.iter().enumerate() {
        if row[a as usize] > row[legal[best] as usize] {
            best = i;
        }
    }
    best
}

/// Metrics of `net` (on the CPU, no autodiff) on `examples`.
pub fn evaluate(net: &PolicyValueNet, examples: &Examples, hyper: &Hyper) -> Metrics {
    let device = Device::flex();
    let rows = examples.rows();
    let (mut policy_sum, mut value_sum, mut score_sum, mut win_sum) = (0.0, 0.0, 0.0, 0.0);
    let (mut agree, mut score_agree, mut options, mut with_options) = (0, 0, 0, 0);
    let all: Vec<usize> = (0..rows).collect();
    for chunk in all.chunks(EVAL_BATCH) {
        let l = losses(net, examples.batch(chunk, &device));
        options += l.options;
        policy_sum += sum(l.policy);
        value_sum += sum(l.value);
        score_sum += sum(l.score);
        win_sum += sum(l.win);
        let logits = l.logits.into_data().try_to_vec::<f32>().expect("f32");
        let scores = l.scores.into_data().try_to_vec::<f32>().expect("f32");
        for (b, &r) in chunk.iter().enumerate() {
            let legal = &examples.legal[examples.offsets[r]..examples.offsets[r + 1]];
            let chosen = examples.chosen[r] as usize;
            agree += usize::from(best(&logits[b * ACTIONS..][..ACTIONS], legal) == chosen);
            if examples.has_options[r] {
                with_options += 1;
                score_agree +=
                    usize::from(best(&scores[b * ACTIONS..][..ACTIONS], legal) == chosen);
            }
        }
    }
    let n = rows.max(1) as f64;
    let per_option = options.max(1) as f64;
    let policy_loss = policy_sum / n;
    let value_mse = value_sum / n;
    let score_mse = score_sum / per_option;
    let win_loss = win_sum / per_option;
    Metrics {
        examples: rows,
        policy_loss,
        agreement: agree as f64 / n,
        value_rmse: value_mse.sqrt() * f64::from(VALUE_SCALE),
        options,
        score_rmse: score_mse.sqrt() * f64::from(VALUE_SCALE),
        win_loss,
        score_agreement: score_agree as f64 / with_options.max(1) as f64,
        loss: policy_loss
            + f64::from(hyper.value_weight) * value_mse
            + f64::from(hyper.score_weight) * score_mse
            + f64::from(hyper.win_weight) * win_loss,
    }
}

/// Caps of one invocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrainLimits {
    /// No epoch starts after it.
    pub wall_time: Duration,
}

/// Why a run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainStop {
    /// All epochs done.
    Finished,
    /// The wall-time cap was reached.
    WallTime,
    /// The progress callback stopped the run.
    Stopped,
}

/// The result of [`train`].
#[derive(Debug)]
pub struct Trained {
    /// The best model's file.
    pub model: Vec<u8>,
    /// Progress (complete if `stop` is `Finished`).
    pub state: State,
    /// Why it stopped.
    pub stop: TrainStop,
}

fn checkpoint_dir(out: &Path) -> PathBuf {
    out.join("checkpoint")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

/// The model file of `net`.
pub fn model_file(net: PolicyValueNet, hidden: usize) -> Vec<u8> {
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape: NetShape::current(hidden),
        phases: PHASES,
    };
    write_model(&header, net).expect("model serialises")
}

/// Trains (or resumes) the run in `out` with `recipe` on `data`, on
/// `device` (an autodiff device). Writes a checkpoint after every epoch
/// and passes its log to `progress`, which may stop the run after the
/// epoch by returning `false`.
pub fn train(
    out: &Path,
    recipe: &Recipe,
    data: &Data,
    device: &Device,
    limits: TrainLimits,
    mut progress: impl FnMut(&EpochLog) -> bool,
) -> io::Result<Trained> {
    let start = Instant::now();
    let hyper = &recipe.hyper;
    let shape = NetShape::current(hyper.hidden);
    let dir = checkpoint_dir(out);
    fs::create_dir_all(&dir)?;
    let adam = AdamConfig::new();

    let (mut net, mut optim, mut state, mut best) = match fs::read(dir.join("state.json")) {
        Ok(bytes) => {
            let state: State = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
            if state.recipe != *recipe {
                return Err(io::Error::other(format!(
                    "{} holds a run with another recipe",
                    out.display()
                )));
            }
            let [weights, optimizer, best] = read_checkpoint(&dir, &state)?;
            let net = from_record(shape, weights, device)?;
            let optim: ModuleOptimizer = adam
                .init()
                .from_bytes(Bytes::from_bytes_vec(optimizer))
                .map_err(io::Error::other)?;
            (net, optim, state, best)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let initial = initial(shape, hyper.seed);
            let bytes = record_bytes(&initial);
            let net = from_record(shape, bytes.clone(), device)?;
            let state = State {
                recipe: recipe.clone(),
                logs: Vec::new(),
                best_epoch: 0,
                checkpoint: None,
            };
            (net, adam.init(), state, bytes)
        }
        Err(e) => return Err(e),
    };

    let mut stop = TrainStop::Finished;
    let rows = data.train.rows();
    for epoch in state.logs.len()..hyper.epochs {
        if start.elapsed() >= limits.wall_time {
            stop = TrainStop::WallTime;
            break;
        }
        let began = Instant::now();
        let (mut sum, mut batches) = (0.0f64, 0usize);
        for chunk in order(rows, hyper.seed, epoch).chunks(hyper.batch.max(1)) {
            let loss = losses(&net, data.train.batch(chunk, device)).total(hyper);
            sum += f64::from(loss.clone().into_scalar::<f32>());
            batches += 1;
            let grads = GradientsParams::from_grads(loss.backward(), &net);
            net = optim.step(hyper.learning_rate, net, grads);
        }
        let current = net.valid();
        let validation = evaluate(&current, &data.validation, hyper);
        let improved = state
            .logs
            .get(state.best_epoch.wrapping_sub(1))
            .is_none_or(|b| validation.loss < b.validation.loss);
        state.logs.push(EpochLog {
            epoch: epoch + 1,
            train_loss: sum / batches.max(1) as f64,
            validation,
            seconds: began.elapsed().as_secs_f64(),
        });
        let go_on = progress(state.logs.last().expect("just pushed"));
        if improved {
            state.best_epoch = epoch + 1;
            best = record_bytes(&current);
        }
        // Each epoch's files go into their own directory; `state.json`,
        // written last and atomically, points at it with their hashes. An
        // interrupted checkpoint leaves the previous state valid.
        let name = format!("epoch-{:04}", epoch + 1);
        let files = [
            ("weights.bpk", record_bytes(&net)),
            (
                "optimizer.bpk",
                optim.into_bytes().map_err(io::Error::other)?.to_vec(),
            ),
            ("best.bpk", best.clone()),
        ];
        fs::create_dir_all(dir.join(&name))?;
        state.checkpoint = Some(Checkpoint {
            dir: name.clone(),
            sha256: files.iter().map(|(_, b)| hex(&sha256(b))).collect(),
        });
        for (file, bytes) in &files {
            write_atomic(&dir.join(&name).join(file), bytes)?;
        }
        let mut bytes = serde_json::to_vec_pretty(&state).map_err(io::Error::other)?;
        bytes.push(b'\n');
        write_atomic(&dir.join("state.json"), &bytes)?;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.file_name() != name.as_str() {
                fs::remove_dir_all(entry.path())?;
            }
        }
        if !go_on && state.logs.len() < hyper.epochs {
            stop = TrainStop::Stopped;
            break;
        }
    }
    let best_net = from_record(shape, best, &Device::flex())?;
    Ok(Trained {
        model: model_file(best_net, hyper.hidden),
        state,
        stop,
    })
}

/// Metrics of a model file's network on `examples`.
pub fn evaluate_model(model: &[u8], examples: &Examples, hyper: &Hyper) -> io::Result<Metrics> {
    let header = skat_nn::read_header(model).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let net = from_record(
        header.shape,
        model[skat_nn::HEADER_BYTES..].to_vec(),
        &Device::flex(),
    )?;
    Ok(evaluate(&net, examples, hyper))
}

/// Hex form of a hash, for manifests.
pub fn hex_hash(bytes: &[u8]) -> String {
    hex(&sha256(bytes))
}
