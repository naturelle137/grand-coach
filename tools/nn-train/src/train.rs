// SPDX-License-Identifier: EUPL-1.2

//! Supervised training of the card-play network (#73) on teacher self-play
//! records (#70).
//!
//! Examples are the card-play decisions (phase Spiel, Re included) of the
//! training split. Kontra follows the rules of thumb (#108): a decision
//! where they say Kontra is left out; otherwise the Kontra actions are
//! removed and a Kontra label of the teacher becomes the card it would have
//! chosen without Kontra ([`skat_ai::kontra_by_rules`] on its option
//! values; without values the decision is left out). Targets:
//!
//! - **Policy**: the teacher's chosen action, optionally mixed with a soft
//!   distribution from the teacher's complete option values
//!   (`softmax(expected_score / soft_temperature)`, share `soft_weight`).
//! - **Value**: the seat's final Seeger-Fabian points, mixed with the
//!   expected score of the chosen option when the teacher's analysis is
//!   complete (share `value_mix`). In card play with an Easy player
//!   (`Table::EasyMix`) the final points reflect Easy's play, not the
//!   teacher's, so a complete analysis gives the value alone.
//!
//! The policy label is always the teacher's: at an Easy seat its analysis
//! labels the position Easy reached, Easy's own action is never a target.
//! - **Options** (when the teacher's analysis is complete): for every legal
//!   action the teacher's expected Seeger-Fabian points (squared error,
//!   weight `score_weight`) and win chance (cross-entropy on the win logit,
//!   weight `win_weight`). Weights 0 train the policy and value only.
//! - **Belief** (auxiliary, `belief_weight` > 0): for every card the seat
//!   cannot see, its true place (next seat, the seat after, the Skat),
//!   from a replay of the deal. A training-only head on the shared hidden
//!   layer learns it with a cross-entropy, so the trunk has to infer hidden
//!   hands from the earlier decisions in the view. The model file holds
//!   only the [`PolicyValueNet`]; the head is saved next to it
//!   (`belief.bpk`) for `nn-train probe`. Model selection ignores it.
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
//!
//! With a [`Plateau`] schedule the run lowers the learning rate when the
//! validation loss stagnates: it continues from the best weights with a
//! smaller rate, and stops early once the last allowed rate stagnates too.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use burn::module::{Module, Param, ParamId};
use burn::nn::{Linear, LinearConfig};
use burn::optim::{AdamConfig, GradientsParams, ModuleOptimizer};
use burn::store::ModuleRecord;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Bytes, Device, Tensor, TensorData};
use serde::{Deserialize, Serialize};
use skat_ai::{KontraRule, Status, kontra_by_rules, kontra_rule};
use skat_core::{OmniscientView, Phase, PlayerView};
use skat_nn::{
    ACTIONS, ENCODER_VERSION, MIN_LAYERS, ModelHeader, NetShape, Phases, PolicyValueNet,
    action_index, encode, sha256, unseen_cards, write_model,
};

use crate::features::FeatureCodes;
use crate::resident::ResidentRows;
use crate::selfplay::{DecisionKind, DecisionRecord, Player, Roles, Split, Teacher};
use crate::shards::{Manifest, read_manifest, read_shard_hashed, replay_deal};

/// Version of the training recipe and run-directory layout.
pub const TRAIN_FORMAT: u32 = 3;

/// Phases the trained model decides; everything else stays Classic.
pub const PHASES: Phases = Phases::SPIEL;

/// Scale of the value target in the loss (points per unit).
pub const VALUE_SCALE: f32 = 60.0;

/// Rows per forward pass when evaluating.
const EVAL_BATCH: usize = 4096;

/// Places of a hidden card for the belief head, relative to the acting
/// seat: the next seat, the seat after it, the Skat.
pub const BELIEF_PLACES: usize = 3;

/// Outputs of the belief head: [`BELIEF_PLACES`] logits per card.
pub const BELIEF_OUTPUTS: usize = 32 * BELIEF_PLACES;

/// Belief target of a card the acting seat can see.
pub const NOT_HIDDEN: u8 = u8::MAX;

/// The true place of every card `view.seat` cannot see
/// ([`unseen_cards`]), [`NOT_HIDDEN`] for the others. `all` is the full
/// state the view was taken from (a training target, never an input).
pub fn belief_target(view: &PlayerView, all: &OmniscientView) -> [u8; 32] {
    let mut target = [NOT_HIDDEN; 32];
    for card in unseen_cards(view) {
        let place = if all.skat.contains(card) {
            2
        } else {
            let seat = (0..3)
                .find(|&s| all.hands[s].contains(card))
                .expect("every unplayed card is in a hand or the Skat");
            (seat + 3 - view.seat.index()) % 3 - 1
        };
        target[usize::from(card.index())] = place as u8;
    }
    target
}

/// Hyperparameters of a training run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hyper {
    /// Width of every hidden layer.
    pub hidden: usize,
    /// Number of hidden layers; omitted from the recipe when it is
    /// [`MIN_LAYERS`], so recipes of two-layer runs read and compare as
    /// before.
    #[serde(default = "min_layers", skip_serializing_if = "is_min_layers")]
    pub layers: usize,
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
    /// Weight of the auxiliary belief loss (0: no belief head); omitted
    /// from the recipe when 0, so earlier recipes read and compare as
    /// before.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub belief_weight: f32,
    /// Seed of the initial weights and the example order.
    pub seed: u64,
    /// Learning-rate schedule on a validation plateau (none: constant
    /// rate until the epoch or wall-time cap); omitted from the recipe
    /// when absent, so earlier recipes read and compare as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plateau: Option<Plateau>,
}

/// Reduce-on-plateau with early stopping. After `patience` epochs without
/// a lower validation loss, counted from the best epoch or the last decay
/// (whichever is later), the run continues from the best weights (Adam
/// state kept) with the learning rate multiplied by `decay`, at most
/// `decays` times; the next plateau stops the run. `decays` 0 is plain
/// early stopping.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plateau {
    /// Epochs without improvement that make a plateau (at least 1).
    pub patience: usize,
    /// Factor of the learning rate per decay, in (0, 1).
    pub decay: f64,
    /// Maximum number of decays.
    pub decays: usize,
}

impl Default for Hyper {
    fn default() -> Hyper {
        Hyper {
            hidden: 256,
            layers: MIN_LAYERS,
            batch: 256,
            learning_rate: 1e-3,
            epochs: 20,
            value_weight: 1.0,
            value_mix: 0.5,
            soft_weight: 0.0,
            soft_temperature: 5.0,
            score_weight: 1.0,
            win_weight: 1.0,
            belief_weight: 0.0,
            seed: 0,
            plateau: None,
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
    /// win losses (without the belief loss).
    pub loss: f64,
    /// Cross-entropy of the belief head in bits per hidden card; omitted
    /// (0) without a belief head.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub belief_bits: f64,
}

/// What happened in one epoch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpochLog {
    /// Epoch number, from 1.
    pub epoch: usize,
    /// Mean training loss over the epoch's batches.
    pub train_loss: f64,
    /// Learning rate of the epoch (0 in logs written before schedules).
    #[serde(default)]
    pub learning_rate: f64,
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
    /// Epochs after which the learning rate was decayed ([`Plateau`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decays: Vec<usize>,
    /// Files of the last completed epoch.
    pub checkpoint: Option<Checkpoint>,
}

impl State {
    /// Learning rate of the next epoch.
    pub fn learning_rate(&self) -> f64 {
        let hyper = &self.recipe.hyper;
        let decay = hyper.plateau.map_or(1.0, |p| p.decay);
        let decays = i32::try_from(self.decays.len()).unwrap_or(i32::MAX);
        hyper.learning_rate * decay.powi(decays)
    }

    /// Completed epochs since the best epoch or the last decay, whichever
    /// is later.
    pub fn stale_epochs(&self) -> usize {
        let since = self
            .decays
            .last()
            .map_or(self.best_epoch, |&d| d.max(self.best_epoch));
        self.logs.len().saturating_sub(since)
    }

    /// Whether the schedule has reached a plateau.
    fn plateau(&self) -> Option<Plateau> {
        self.recipe
            .hyper
            .plateau
            .filter(|p| self.stale_epochs() >= p.patience)
    }

    /// Whether the run stops: a plateau after the last allowed decay.
    fn exhausted(&self) -> bool {
        self.plateau()
            .is_some_and(|p| self.decays.len() >= p.decays)
    }
}

/// Where the files of a completed epoch are, with their SHA-256 hashes
/// (weights, Adam state, best weights so far).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Subdirectory of `checkpoint/`.
    pub dir: String,
    /// Hashes of `weights.bpk`, `optimizer.bpk`, `best.bpk` and, with a
    /// belief head, `belief.bpk` and `best-belief.bpk`, hexadecimal.
    pub sha256: Vec<String>,
}

/// Files of a checkpoint, in the order of [`Checkpoint::sha256`]; the
/// belief files only with a belief head.
const CHECKPOINT_FILES: [&str; 5] = [
    "weights.bpk",
    "optimizer.bpk",
    "best.bpk",
    "belief.bpk",
    "best-belief.bpk",
];

/// The verified files of the checkpoint `state` points at: the first
/// `count` of [`CHECKPOINT_FILES`].
fn read_checkpoint(dir: &Path, state: &State, count: usize) -> io::Result<Vec<Vec<u8>>> {
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
    if checkpoint.sha256.len() != count {
        return Err(io::Error::other("checkpoint has other files"));
    }
    CHECKPOINT_FILES[..count]
        .iter()
        .enumerate()
        .map(|(i, file)| read(i, file))
        .collect()
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
    /// Why the run stopped: `finished`, `wall-time`, `stopped` or
    /// `plateau`.
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

/// Encoded examples, row-major and compact: feature rows as one-byte
/// codes ([`FeatureCodes`], lossless), legal actions and policy targets
/// per legal action.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Examples {
    /// Feature rows, [`skat_nn::FEATURES`] each.
    pub features: FeatureCodes,
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
    /// Belief targets ([`belief_target`]), 32 per row; empty unless the
    /// examples were loaded with them.
    pub belief: Vec<u8>,
    /// Who played the acting seat, per row (bookkeeping for reports).
    pub player: Vec<Player>,
    /// Card-play roles of the row's game, per row (bookkeeping).
    pub roles: Vec<Roles>,
}

/// Tensors of a batch.
pub(crate) struct Batch {
    pub(crate) features: Tensor<2>,
    /// 0 for legal, -1e9 for illegal actions.
    pub(crate) mask: Tensor<2>,
    pub(crate) policy: Tensor<2>,
    pub(crate) value: Tensor<2>,
    /// 1 for legal actions with a teacher option value, else 0.
    pub(crate) option_mask: Tensor<2>,
    pub(crate) score: Tensor<2>,
    pub(crate) win: Tensor<2>,
    /// Number of ones in `option_mask`.
    pub(crate) options: usize,
    /// One-hot belief targets `[batch, BELIEF_OUTPUTS]` (zero for cards
    /// the seat sees) and the number of hidden cards, if loaded.
    pub(crate) belief: Option<(Tensor<2>, usize)>,
}

/// `record` with Kontra by the rules of thumb (#108): `Some(None)` if the
/// decision offers no Kontra, `Some(Some(_))` without the Kontra actions and
/// with a card label, `None` if the rules say Kontra or no card label is
/// known.
fn without_kontra(record: &DecisionRecord) -> Option<Option<DecisionRecord>> {
    let view = match kontra_rule(&record.view) {
        KontraRule::Free => return Some(None),
        KontraRule::Rules(_) => return None,
        KontraRule::Without(view) => view,
    };
    let action = if view.legal_actions.contains(&record.action) {
        record.action
    } else {
        kontra_by_rules(&record.view, &record.teacher.options)?
    };
    let options = record
        .teacher
        .options
        .iter()
        .filter(|o| view.legal_actions.contains(&o.action))
        .cloned()
        .collect();
    Some(Some(DecisionRecord {
        view,
        action,
        teacher: Teacher {
            options,
            ..record.teacher.clone()
        },
        ..record.clone()
    }))
}

impl Examples {
    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.value.len()
    }

    pub(crate) fn new() -> Examples {
        Examples {
            offsets: vec![0],
            ..Examples::default()
        }
    }

    /// Adds `record` if it is a card-play decision, with its belief target
    /// if given.
    pub(crate) fn push(
        &mut self,
        r: &DecisionRecord,
        hyper: &Hyper,
        belief: Option<[u8; 32]>,
    ) -> bool {
        if r.view.phase != Phase::Spiel || !PHASES.contains(r.view.phase) {
            return false;
        }
        let relabelled;
        let r = match without_kontra(r) {
            Some(Some(record)) => {
                relabelled = record;
                &relabelled
            }
            Some(None) => r,
            None => return false,
        };
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
            let m = if r.roles().weak() > 0 {
                1.0
            } else {
                hyper.value_mix
            };
            (1.0 - m) * final_points + m * scores[chosen] as f32
        } else {
            final_points
        };
        self.features.push_row(&encode(&r.view));
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
        self.player.push(r.player());
        self.roles.push(r.roles());
        if let Some(belief) = belief {
            self.belief.extend(belief);
        }
        true
    }

    /// The rows for which `keep(row)` holds, in order.
    pub fn select(&self, keep: impl Fn(usize) -> bool) -> Examples {
        let mut out = Examples::new();
        out.features = FeatureCodes::with_table_of(&self.features);
        let with_belief = self.belief.len() == 32 * self.rows();
        for r in (0..self.rows()).filter(|&r| keep(r)) {
            let span = self.offsets[r]..self.offsets[r + 1];
            out.features.push_row_of(&self.features, r);
            out.legal.extend_from_slice(&self.legal[span.clone()]);
            out.target.extend_from_slice(&self.target[span.clone()]);
            out.option_score
                .extend_from_slice(&self.option_score[span.clone()]);
            out.option_win.extend_from_slice(&self.option_win[span]);
            out.offsets.push(out.legal.len());
            out.chosen.push(self.chosen[r]);
            out.value.push(self.value[r]);
            out.has_options.push(self.has_options[r]);
            out.player.push(self.player[r]);
            out.roles.push(self.roles[r]);
            if with_belief {
                out.belief.extend_from_slice(&self.belief[32 * r..][..32]);
            }
        }
        out
    }

    /// Tensors for the rows `rows`.
    pub(crate) fn batch(&self, rows: &[usize], inputs: usize, device: &Device) -> Batch {
        let n = rows.len();
        let mut features = Vec::with_capacity(n * inputs);
        let mut mask = vec![-1e9f32; n * ACTIONS];
        let mut target = vec![0.0f32; n * ACTIONS];
        let mut option_mask = vec![0.0f32; n * ACTIONS];
        let mut score = vec![0.0f32; n * ACTIONS];
        let mut win = vec![0.0f32; n * ACTIONS];
        let mut value = Vec::with_capacity(n);
        let mut options = 0;
        let with_belief = self.belief.len() == 32 * self.rows();
        let mut belief = vec![0.0f32; if with_belief { n * BELIEF_OUTPUTS } else { 0 }];
        let mut hidden = 0;
        for (b, &r) in rows.iter().enumerate() {
            self.features.decode_into(r, inputs, &mut features);
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
            if with_belief {
                for (c, &place) in self.belief[32 * r..][..32].iter().enumerate() {
                    if place != NOT_HIDDEN {
                        belief[b * BELIEF_OUTPUTS + c * BELIEF_PLACES + usize::from(place)] = 1.0;
                        hidden += 1;
                    }
                }
            }
        }
        Batch {
            features: tensor(features, [n, inputs], device),
            mask: tensor(mask, [n, ACTIONS], device),
            policy: tensor(target, [n, ACTIONS], device),
            value: tensor(value, [n, 1], device),
            option_mask: tensor(option_mask, [n, ACTIONS], device),
            score: tensor(score, [n, ACTIONS], device),
            win: tensor(win, [n, ACTIONS], device),
            options,
            belief: with_belief.then(|| (tensor(belief, [n, BELIEF_OUTPUTS], device), hidden)),
        }
    }
}

pub(crate) fn tensor(values: Vec<f32>, shape: [usize; 2], device: &Device) -> Tensor<2> {
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

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads every complete shard of the runs in `dirs` and encodes their
/// card-play decisions with the targets of `hyper`, with belief targets if
/// `hyper.belief_weight` > 0. Incomplete shards are skipped (and counted in
/// the source); corrupt ones are errors.
pub fn load(dirs: &[PathBuf], hyper: &Hyper) -> io::Result<Data> {
    load_with(dirs, hyper, hyper.belief_weight > 0.0)
}

/// The belief target of each record of `records` (one shard, deals in
/// order), from a replay of its deal.
pub fn belief_targets(
    manifest: &Manifest,
    records: &[DecisionRecord],
) -> Result<Vec<[u8; 32]>, String> {
    let mut targets = Vec::with_capacity(records.len());
    let mut rest = records;
    while let Some(first) = rest.first() {
        let len = rest.iter().take_while(|r| r.deal == first.deal).count();
        let (rows, next) = rest.split_at(len);
        replay_deal(manifest, first.deal, rows, |r, game| {
            targets.push(belief_target(&r.view, &game.omniscient_view()));
            Ok(())
        })?;
        rest = next;
    }
    Ok(targets)
}

/// [`load`], with belief targets if `beliefs`.
pub fn load_with(dirs: &[PathBuf], hyper: &Hyper, beliefs: bool) -> io::Result<Data> {
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
            let (records, digest) = match read_shard_hashed(dir, index) {
                Ok(r) => r,
                Err(crate::shards::ShardError::Incomplete) => continue,
                Err(e) => return Err(io::Error::other(format!("{}: {e}", dir.display()))),
            };
            hashes.extend_from_slice(&digest);
            shards += 1;
            let targets = if beliefs {
                belief_targets(&manifest, &records)
                    .map_err(|e| io::Error::other(format!("{}: {e}", dir.display())))?
            } else {
                Vec::new()
            };
            for (i, r) in records.iter().enumerate() {
                crate::selfplay::check_record(r).map_err(io::Error::other)?;
                let (examples, split) = match r.split {
                    Split::Train => (&mut data.train, 0),
                    Split::Validation => (&mut data.validation, 1),
                    Split::Test => (&mut data.test, 2),
                };
                if examples.push(r, hyper, targets.get(i).copied()) {
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
pub(crate) struct SplitMix(pub(crate) u64);

impl SplitMix {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub(crate) fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

const INIT_KEY: u64 = 0x696e_6974;
const ORDER_KEY: u64 = 0x6f72_6465;
const BELIEF_KEY: u64 = 0x6265_6c66;

/// First parameter id of the belief head, far above the network's.
const BELIEF_PARAM: u64 = 1_000_000;

/// The network being trained: the model plus the optional training-only
/// belief head on its last hidden layer. Without the head it has exactly
/// the network's parameters, so checkpoints of earlier runs resume.
#[derive(Module, Debug)]
pub struct TrainNet {
    /// The model.
    pub net: PolicyValueNet,
    /// The belief head (`hidden → BELIEF_OUTPUTS`), if trained.
    pub belief: Option<Linear>,
}

/// The initial belief head for `hidden` units: uniform in ±1/sqrt(hidden),
/// drawn from the seed.
pub fn initial_belief(hidden: usize, seed: u64, device: &Device) -> Linear {
    let mut rng = SplitMix(seed ^ BELIEF_KEY);
    let bound = 1.0 / (hidden as f32).sqrt();
    let mut draw =
        |n: usize| -> Vec<f32> { (0..n).map(|_| (rng.unit() * 2.0 - 1.0) * bound).collect() };
    let weight = draw(hidden * BELIEF_OUTPUTS);
    let bias = draw(BELIEF_OUTPUTS);
    let mut linear = LinearConfig::new(hidden, BELIEF_OUTPUTS).init(device);
    linear.weight = Param::initialized(
        ParamId::from(BELIEF_PARAM + 1),
        Tensor::from_data(TensorData::new(weight, [hidden, BELIEF_OUTPUTS]), device),
    );
    linear.bias = Some(Param::initialized(
        ParamId::from(BELIEF_PARAM + 2),
        Tensor::from_data(TensorData::new(bias, [BELIEF_OUTPUTS]), device),
    ));
    linear
}

/// A belief head for `hidden` units from its record bytes.
pub fn belief_from_record(hidden: usize, bytes: Vec<u8>, device: &Device) -> io::Result<Linear> {
    let record =
        ModuleRecord::from_bytes(Bytes::from_bytes_vec(bytes)).map_err(io::Error::other)?;
    LinearConfig::new(hidden, BELIEF_OUTPUTS)
        .init(device)
        .try_load_record(record)
        .map_err(|e| io::Error::other(e.to_string()))
}

/// Belief logits `[batch, BELIEF_OUTPUTS]` of the head for trunk
/// activations `x`, as log-probabilities over the places of each card
/// (`[batch, 32, BELIEF_PLACES]`).
pub fn belief_log_probs(head: &Linear, x: Tensor<2>) -> Tensor<3> {
    let n = x.dims()[0];
    log_softmax(head.forward(x).reshape([n, 32, BELIEF_PLACES]), 2)
}

/// The initial network: every weight and bias uniform in
/// ±1/sqrt(fan_in) (Burn's and PyTorch's default bound), drawn from the
/// seed.
pub fn initial(shape: NetShape, seed: u64) -> PolicyValueNet {
    let mut rng = SplitMix(seed ^ INIT_KEY);
    let mut layers = vec![(shape.inputs, shape.hidden)];
    layers.extend((1..shape.layers).map(|_| (shape.hidden, shape.hidden)));
    layers.extend([
        (shape.hidden, shape.actions),
        (shape.hidden, 1),
        (shape.hidden, shape.actions),
        (shape.hidden, shape.actions),
    ]);
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

pub(crate) fn record_bytes<M: Module>(net: &M) -> Vec<u8> {
    net.clone()
        .into_record()
        .into_bytes()
        .expect("weights serialise")
        .to_vec()
}

pub(crate) fn from_record(
    shape: NetShape,
    bytes: Vec<u8>,
    device: &Device,
) -> io::Result<PolicyValueNet> {
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
    /// Belief cross-entropy summed per row (`[n, 1]`, nats) and the number
    /// of hidden cards, with a head and targets.
    belief: Option<(Tensor<2>, usize)>,
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
        if let Some((belief, hidden)) = self.belief
            && hyper.belief_weight > 0.0
        {
            loss = loss + belief.sum() * (hyper.belief_weight / hidden.max(1) as f32);
        }
        loss
    }
}

fn losses(net: &PolicyValueNet, head: Option<&Linear>, b: Batch) -> Losses {
    let x = net.trunk(b.features);
    let belief = match (head, b.belief) {
        (Some(head), Some((target, hidden))) => {
            let n = target.dims()[0];
            let target = target.reshape([n, 32, BELIEF_PLACES]);
            let ce = (belief_log_probs(head, x.clone()) * target)
                .sum_dim(2)
                .sum_dim(1)
                .reshape([n, 1])
                .neg();
            Some((ce, hidden))
        }
        _ => None,
    };
    let o = net.heads(x);
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
        belief,
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

/// Metrics of `net` (no autodiff, on the device the net lives on) on `examples`.
pub fn evaluate(net: &PolicyValueNet, examples: &Examples, hyper: &Hyper) -> Metrics {
    evaluate_with(net, None, examples, hyper)
}

/// [`evaluate`], with the belief cross-entropy of `head` if given and the
/// examples have belief targets.
pub fn evaluate_with(
    net: &PolicyValueNet,
    head: Option<&Linear>,
    examples: &Examples,
    hyper: &Hyper,
) -> Metrics {
    // A net trained on the GPU is evaluated there; a CPU net gets Flex.
    let device = net
        .devices()
        .into_iter()
        .next()
        .unwrap_or_else(Device::flex);
    let inputs = net.inputs();
    let rows = examples.rows();
    let (mut policy_sum, mut value_sum, mut score_sum, mut win_sum) = (0.0, 0.0, 0.0, 0.0);
    let (mut agree, mut score_agree, mut options, mut with_options) = (0, 0, 0, 0);
    let (mut belief_sum, mut hidden) = (0.0, 0);
    let all: Vec<usize> = (0..rows).collect();
    for chunk in all.chunks(EVAL_BATCH) {
        let l = losses(net, head, examples.batch(chunk, inputs, &device));
        options += l.options;
        if let Some((belief, n)) = l.belief {
            belief_sum += sum(belief);
            hidden += n;
        }
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
        belief_bits: belief_sum / hidden.max(1) as f64 / std::f64::consts::LN_2,
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
    /// The validation loss stagnated after the last allowed learning-rate
    /// decay ([`Plateau`]).
    Plateau,
}

/// The result of [`train`].
#[derive(Debug)]
pub struct Trained {
    /// The best model's file.
    pub model: Vec<u8>,
    /// Record bytes of the best model's belief head, if trained.
    pub belief: Option<Vec<u8>>,
    /// Progress (complete if `stop` is `Finished`).
    pub state: State,
    /// Why it stopped.
    pub stop: TrainStop,
}

fn checkpoint_dir(out: &Path) -> PathBuf {
    out.join("checkpoint")
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

/// Serde default of a layer count: [`MIN_LAYERS`].
pub(crate) const fn min_layers() -> usize {
    MIN_LAYERS
}

/// Whether a weight is 0 (then it is not serialised).
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
pub(crate) fn is_zero(x: &f32) -> bool {
    *x == 0.0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
fn is_zero_f64(x: &f64) -> bool {
    *x == 0.0
}

/// Whether a layer count is [`MIN_LAYERS`] (then it is not serialised).
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
pub(crate) const fn is_min_layers(layers: &usize) -> bool {
    *layers == MIN_LAYERS
}

/// The model file of `net` with layer sizes `shape`.
pub fn model_file(net: PolicyValueNet, shape: NetShape) -> Vec<u8> {
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape,
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
    progress: impl FnMut(&EpochLog) -> bool,
) -> io::Result<Trained> {
    train_with(out, recipe, data, None, device, limits, progress)
}

/// [`train`], with the training split's feature rows resident on `device`
/// if `resident` is given ([`ResidentRows`], uploaded from
/// `data.train.features`). The recipe and checkpoints do not depend on it;
/// a run may resume with or without it.
pub fn train_with(
    out: &Path,
    recipe: &Recipe,
    data: &Data,
    resident: Option<&ResidentRows>,
    device: &Device,
    limits: TrainLimits,
    mut progress: impl FnMut(&EpochLog) -> bool,
) -> io::Result<Trained> {
    let start = Instant::now();
    let hyper = &recipe.hyper;
    if recipe.encoder_version != ENCODER_VERSION {
        return Err(io::Error::other(
            "training recipe has another encoder version",
        ));
    }
    if let Some(p) = hyper.plateau
        && (p.patience == 0 || !(p.decay > 0.0 && p.decay < 1.0))
    {
        return Err(io::Error::other(
            "plateau schedule needs a patience of at least 1 and a decay in (0, 1)",
        ));
    }
    let shape = NetShape::current(hyper.hidden).with_layers(hyper.layers);
    if let Some(r) = resident
        && (r.rows() != data.train.rows() || r.inputs() != shape.inputs)
    {
        return Err(io::Error::other(
            "resident rows do not match the training split and network",
        ));
    }
    let dir = checkpoint_dir(out);
    fs::create_dir_all(&dir)?;
    let adam = AdamConfig::new();

    let with_belief = hyper.belief_weight > 0.0;
    let files = if with_belief { 5 } else { 3 };
    let (mut net, mut optim, mut state, mut best, mut best_belief) =
        match fs::read(dir.join("state.json")) {
            Ok(bytes) => {
                let state: State = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                if state.recipe != *recipe {
                    return Err(io::Error::other(format!(
                        "{} holds a run with another recipe",
                        out.display()
                    )));
                }
                let mut checkpoint = read_checkpoint(&dir, &state, files)?.into_iter();
                let mut next = || checkpoint.next().expect("checked count");
                let net = from_record(shape, next(), device)?;
                let optim: ModuleOptimizer = adam
                    .init()
                    .from_bytes(Bytes::from_bytes_vec(next()))
                    .map_err(io::Error::other)?;
                let best = next();
                let (belief, best_belief) = if with_belief {
                    let head = belief_from_record(shape.hidden, next(), device)?;
                    (Some(head), Some(next()))
                } else {
                    (None, None)
                };
                (TrainNet { net, belief }, optim, state, best, best_belief)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let initial = initial(shape, hyper.seed);
                let bytes = record_bytes(&initial);
                let net = from_record(shape, bytes.clone(), device)?;
                let belief = with_belief.then(|| initial_belief(shape.hidden, hyper.seed, device));
                let best_belief = belief.as_ref().map(record_bytes);
                let state = State {
                    recipe: recipe.clone(),
                    logs: Vec::new(),
                    best_epoch: 0,
                    decays: Vec::new(),
                    checkpoint: None,
                };
                (
                    TrainNet { net, belief },
                    adam.init(),
                    state,
                    bytes,
                    best_belief,
                )
            }
            Err(e) => return Err(e),
        };

    let mut stop = TrainStop::Finished;
    let rows = data.train.rows();
    for epoch in state.logs.len()..hyper.epochs {
        if state.exhausted() {
            stop = TrainStop::Plateau;
            break;
        }
        if start.elapsed() >= limits.wall_time {
            stop = TrainStop::WallTime;
            break;
        }
        let began = Instant::now();
        let learning_rate = state.learning_rate();
        let (mut sum, mut batches) = (0.0f64, 0usize);
        for chunk in order(rows, hyper.seed, epoch).chunks(hyper.batch.max(1)) {
            let batch = match resident {
                Some(r) => {
                    let mut grouped = chunk.to_vec();
                    r.group(&mut grouped);
                    r.batch(&data.train, &grouped, device)
                }
                None => data.train.batch(chunk, shape.inputs, device),
            };
            let loss = losses(&net.net, net.belief.as_ref(), batch).total(hyper);
            sum += f64::from(loss.clone().into_scalar::<f32>());
            batches += 1;
            let grads = GradientsParams::from_grads(loss.backward(), &net);
            net = optim.step(learning_rate, net, grads);
        }
        let current = net.valid();
        let validation = evaluate_with(
            &current.net,
            current.belief.as_ref(),
            &data.validation,
            hyper,
        );
        let improved = state
            .logs
            .get(state.best_epoch.wrapping_sub(1))
            .is_none_or(|b| validation.loss < b.validation.loss);
        state.logs.push(EpochLog {
            epoch: epoch + 1,
            train_loss: sum / batches.max(1) as f64,
            learning_rate,
            validation,
            seconds: began.elapsed().as_secs_f64(),
        });
        let go_on = progress(state.logs.last().expect("just pushed"));
        if improved {
            state.best_epoch = epoch + 1;
            best = record_bytes(&current.net);
            best_belief = current.belief.as_ref().map(record_bytes);
        }
        // On a plateau with decays left, continue from the best weights
        // with a smaller rate; the checkpoint below already holds them.
        if state
            .plateau()
            .is_some_and(|p| state.decays.len() < p.decays)
        {
            state.decays.push(epoch + 1);
            net.net = from_record(shape, best.clone(), device)?;
            if let Some(head) = &best_belief {
                net.belief = Some(belief_from_record(shape.hidden, head.clone(), device)?);
            }
        }
        // Each epoch's files go into their own directory; `state.json`,
        // written last and atomically, points at it with their hashes. An
        // interrupted checkpoint leaves the previous state valid.
        let name = format!("epoch-{:04}", epoch + 1);
        let mut files = vec![
            record_bytes(&net.net),
            optim.into_bytes().map_err(io::Error::other)?.to_vec(),
            best.clone(),
        ];
        if let (Some(head), Some(best_head)) = (&net.belief, &best_belief) {
            files.extend([record_bytes(head), best_head.clone()]);
        }
        let files: Vec<(&str, Vec<u8>)> = CHECKPOINT_FILES.into_iter().zip(files).collect();
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
        if state.exhausted() && state.logs.len() < hyper.epochs {
            stop = TrainStop::Plateau;
            break;
        }
    }
    let best_net = from_record(shape, best, &Device::flex())?;
    Ok(Trained {
        model: model_file(best_net, shape),
        belief: best_belief,
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

/// A model file's network and, if given, its belief head (record bytes).
pub fn read_model(
    model: &[u8],
    belief: Option<&[u8]>,
) -> io::Result<(PolicyValueNet, Option<Linear>)> {
    let header = skat_nn::read_header(model).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let device = Device::flex();
    let net = from_record(
        header.shape,
        model[skat_nn::HEADER_BYTES..].to_vec(),
        &device,
    )?;
    let head = belief
        .map(|b| belief_from_record(header.shape.hidden, b.to_vec(), &device))
        .transpose()?;
    Ok((net, head))
}

/// Hex form of a hash, for manifests.
pub fn hex_hash(bytes: &[u8]) -> String {
    hex(&sha256(bytes))
}

#[cfg(test)]
mod tests {
    use skat_ai::{BaselineOpponent, Budget, Opponent, Profile};
    use skat_core::{Action, Game, RuleConfig};

    use super::*;
    use crate::selfplay::teach;

    /// A record of a teacher that analyses Kontra (the pre-#108 teacher).
    fn record(view: &PlayerView) -> DecisionRecord {
        let teacher = Opponent {
            profile: Profile {
                budget: Some(Budget {
                    samples: 2,
                    nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
                }),
                search_kontra: true,
                ..Profile::RULES
            },
            seed: 1,
        };
        let (action, teacher) = teach(view, &teacher).unwrap();
        DecisionRecord {
            deal: 0,
            split: Split::Train,
            view: view.clone(),
            action,
            played: None,
            lineup: None,
            teacher,
            final_points: 0,
        }
    }

    #[test]
    fn kontra_labels_follow_the_rules_of_thumb() {
        let (mut relabelled, mut dropped, mut free) = (0, 0, 0);
        for seed in 0..200 {
            let mut game = Game::new(seed, RuleConfig { kontra_re: true });
            while let Some(seat) = game.current_seat() {
                let view = game.view(seat);
                let rules = BaselineOpponent.decide(&view).unwrap();
                if view.phase == Phase::Spiel && view.legal_actions.contains(&Action::Kontra) {
                    let r = record(&view);
                    match without_kontra(&r) {
                        None => {
                            assert_eq!(rules, Action::Kontra);
                            dropped += 1;
                        }
                        Some(Some(n)) => {
                            assert!(!n.view.legal_actions.contains(&Action::Kontra));
                            assert!(n.view.legal_actions.contains(&n.action));
                            assert_eq!(n.teacher.options.len(), n.view.legal_actions.len());
                            if r.action == Action::Kontra {
                                assert_eq!(
                                    Some(n.action),
                                    kontra_by_rules(&r.view, &r.teacher.options)
                                );
                                relabelled += 1;
                            } else {
                                assert_eq!(n.action, r.action);
                            }
                        }
                        Some(None) => panic!("a Kontra decision passed unchanged"),
                    }
                } else if view.phase == Phase::Spiel && free < 5 {
                    let r = record(&view);
                    assert!(matches!(without_kontra(&r), Some(None)));
                    free += 1;
                }
                game.apply(rules).unwrap();
            }
            if relabelled >= 3 && dropped >= 1 {
                return;
            }
        }
        panic!("too few Kontra decisions: {relabelled} relabelled, {dropped} dropped");
    }
}
