// SPDX-License-Identifier: EUPL-1.2

//! Model files and inference.
//!
//! A model file is a 24-byte header followed by the network's weights in
//! Burn's burnpack format. All numbers are little-endian.
//!
//! | Offset | Size | Field |
//! |--------|------|-------|
//! | 0 | 4 | magic `GCNN` |
//! | 4 | 2 | file format version ([`FORMAT_VERSION`]) |
//! | 6 | 2 | encoder version ([`ENCODER_VERSION`]) |
//! | 8 | 4 | input features |
//! | 12 | 4 | hidden width |
//! | 16 | 4 | policy actions |
//! | 20 | 1 | covered phases (bit 0 Reizen … bit 4 Spiel) |
//! | 21 | 1 | hidden layers; 0 means 2 (files written before it was recorded) |
//! | 22 | 2 | reserved, zero |
//!
//! The caller passes the bytes and their expected SHA-256 hash; this crate
//! reads no files (no IO below `skat-app`). Inference runs on Burn's
//! pure-Rust CPU backend Flex.

use burn::module::Module;
use burn::store::ModuleRecord;
use burn::tensor::{Bytes, Device, Tensor, TensorData};
use sha2::{Digest, Sha256};
use skat_core::{Action, Phase, PlayerView};

use crate::actions::{ACTIONS, action_index};
use crate::encoder::{ENCODER_VERSION, FEATURES, V1_FEATURES, encode, encode_v1, phase_index};
use crate::net::{MAX_LAYERS, MIN_LAYERS, NetShape, PolicyValueNet};

/// Version of the model file layout. Version 2 added the option heads
/// (expected points and win chance per action).
pub const FORMAT_VERSION: u16 = 2;

/// Size of the model file header; the weights follow it.
pub const HEADER_BYTES: usize = 24;

const MAGIC: [u8; 4] = *b"GCNN";
const HEADER: usize = HEADER_BYTES;
/// Upper bound for the hidden width, so a corrupt header cannot request a
/// huge allocation.
const MAX_HIDDEN: usize = 4096;

/// SHA-256 hash of a model file, as passed to [`NeuralModel::load`].
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// A set of decision phases a model covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Phases(u8);

impl Phases {
    /// Every decision phase.
    pub const ALL: Phases = Phases(0b1_1111);
    /// Card play only (including Kontra/Re).
    pub const SPIEL: Phases = Phases(0b1_0000);

    /// The set with bits `bits` (bit 0 Reizen … bit 4 Spiel); `None` if
    /// it is empty or has other bits.
    pub const fn from_bits(bits: u8) -> Option<Phases> {
        if bits == 0 || bits & !Phases::ALL.0 != 0 {
            None
        } else {
            Some(Phases(bits))
        }
    }

    /// The bits of the set.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// `true` if decisions of `phase` are covered.
    pub fn contains(self, phase: Phase) -> bool {
        phase_index(phase).is_some_and(|i| self.0 & (1 << i) != 0)
    }
}

/// What a model file declares about itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelHeader {
    /// Encoder version the model was trained for.
    pub encoder_version: u16,
    /// Layer sizes.
    pub shape: NetShape,
    /// Phases the model decides; others fall back to Classic.
    pub phases: Phases,
}

/// Why a model could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ModelError {
    /// No model was provided.
    Missing,
    /// The bytes do not have the expected SHA-256 hash.
    HashMismatch,
    /// Shorter than the header.
    Truncated,
    /// Not a model file.
    Magic,
    /// Unsupported file format version.
    Format {
        /// Version in the file.
        found: u16,
    },
    /// Trained for another encoder version.
    Encoder {
        /// Version in the file.
        found: u16,
    },
    /// Layer sizes that do not fit the current encoder, or an invalid
    /// hidden width.
    Shape {
        /// Input features in the file.
        inputs: u32,
        /// Hidden width in the file.
        hidden: u32,
        /// Policy actions in the file.
        actions: u32,
    },
    /// A number of hidden layers this build does not support.
    Layers {
        /// Byte in the file.
        found: u8,
    },
    /// Empty or unknown phase set.
    Phases {
        /// Bits in the file.
        bits: u8,
    },
    /// The weights cannot be read or do not fit the shape.
    Weights(String),
}

/// Why a decision falls back to the Classic engine (D-07). Inspectable, so
/// tests, the arena and later the coach can tell why.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Fallback {
    /// No usable model: missing, corrupt or incompatible.
    Model(ModelError),
    /// The model does not cover this phase.
    UnsupportedPhase(Phase),
    /// The view offers no action: not this seat's turn.
    NoDecision,
    /// A legal action has no index in the action space.
    UnknownAction(Action),
    /// The network produced a NaN or infinite logit or value.
    NonFinite,
    /// The chosen action is not legal in the view (checked again before
    /// acting).
    IllegalChoice(Action),
}

/// How a model picks its action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Choice {
    /// The most probable action of the policy head.
    #[default]
    Policy,
    /// The action with the highest expected points of the option heads.
    Score,
}

/// The option heads' estimate for one legal action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionEstimate {
    /// The action.
    pub action: Action,
    /// Expected Seeger-Fabian points of the acting seat after it.
    pub score: f32,
    /// Probability that the acting seat's party wins after it.
    pub win_chance: f32,
}

/// A model's choice for one decision. The probabilities, the value and the
/// option estimates are model estimates: never exact, never Monte-Carlo
/// samples.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// The chosen legal action by the [`Choice`]; ties go to the first in
    /// the view's order.
    pub action: Action,
    /// Probability of each legal action, in the view's order; illegal
    /// actions are masked out before normalisation.
    pub policy: Vec<(Action, f32)>,
    /// Estimated Seeger-Fabian points of the acting seat for this game.
    pub value: f32,
    /// Option estimates of each legal action, in the view's order.
    pub options: Vec<OptionEstimate>,
}

/// Raw network outputs for one feature vector.
#[derive(Clone, Debug, PartialEq)]
pub struct RawOutputs {
    /// One policy logit per action.
    pub logits: Vec<f32>,
    /// The value.
    pub value: f32,
    /// Expected points per action.
    pub score: Vec<f32>,
    /// Win logit per action.
    pub win: Vec<f32>,
}

/// Serialises `net` with `header` into a model file.
///
/// # Errors
///
/// If the header declares an unsupported number of hidden layers or Burn
/// cannot serialise the weights.
pub fn write_model(header: &ModelHeader, net: PolicyValueNet) -> Result<Vec<u8>, ModelError> {
    let layers = header.shape.layers;
    if !(MIN_LAYERS..=MAX_LAYERS).contains(&layers) {
        return Err(ModelError::Layers {
            found: u8::try_from(layers).unwrap_or(u8::MAX),
        });
    }
    let weights = net
        .into_record()
        .into_bytes()
        .map_err(|e| ModelError::Weights(e.to_string()))?;
    let mut out = Vec::with_capacity(HEADER + weights.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&header.encoder_version.to_le_bytes());
    for n in [
        header.shape.inputs,
        header.shape.hidden,
        header.shape.actions,
    ] {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    out.push(header.phases.bits());
    // Two layers are written as 0, so such files stay byte-identical to
    // those of builds that did not record the layer count.
    let layers = header.shape.layers;
    out.push(if layers == MIN_LAYERS {
        0
    } else {
        layers as u8
    });
    out.extend_from_slice(&[0; 2]);
    out.extend_from_slice(&weights);
    Ok(out)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Reads and checks the header of a model file.
///
/// # Errors
///
/// If the header is not one this build can use.
pub fn read_header(bytes: &[u8]) -> Result<ModelHeader, ModelError> {
    if bytes.len() < HEADER {
        return Err(ModelError::Truncated);
    }
    if bytes[..4] != MAGIC {
        return Err(ModelError::Magic);
    }
    let format = u16_at(bytes, 4);
    if format != FORMAT_VERSION {
        return Err(ModelError::Format { found: format });
    }
    let encoder = u16_at(bytes, 6);
    let expected_inputs = match encoder {
        1 => V1_FEATURES,
        ENCODER_VERSION => FEATURES,
        found => return Err(ModelError::Encoder { found }),
    };
    let (inputs, hidden, actions) = (u32_at(bytes, 8), u32_at(bytes, 12), u32_at(bytes, 16));
    let fits = inputs as usize == expected_inputs
        && actions as usize == ACTIONS
        && (1..=MAX_HIDDEN).contains(&(hidden as usize));
    if !fits {
        return Err(ModelError::Shape {
            inputs,
            hidden,
            actions,
        });
    }
    let phases = Phases::from_bits(bytes[20]).ok_or(ModelError::Phases { bits: bytes[20] })?;
    let layers = match bytes[21] {
        0 => MIN_LAYERS,
        n if (MIN_LAYERS + 1..=MAX_LAYERS).contains(&usize::from(n)) => usize::from(n),
        found => return Err(ModelError::Layers { found }),
    };
    Ok(ModelHeader {
        encoder_version: encoder,
        shape: NetShape {
            inputs: expected_inputs,
            ..NetShape::current(hidden as usize).with_layers(layers)
        },
        phases,
    })
}

/// A loaded model, ready for inference on the CPU.
#[derive(Debug)]
pub struct NeuralModel {
    header: ModelHeader,
    sha256: [u8; 32],
    net: PolicyValueNet,
    device: Device,
}

impl NeuralModel {
    /// Loads a model file. The bytes must have the hash `expected_sha256`,
    /// a header this build supports (format and encoder version, shape) and
    /// weights that fit it.
    ///
    /// # Errors
    ///
    /// Every reason the model cannot be used; the caller then falls back to
    /// Classic ([`Fallback::Model`]).
    pub fn load(bytes: &[u8], expected_sha256: &[u8; 32]) -> Result<NeuralModel, ModelError> {
        let hash = sha256(bytes);
        if &hash != expected_sha256 {
            return Err(ModelError::HashMismatch);
        }
        let header = read_header(bytes)?;
        let device = Device::flex();
        let record = ModuleRecord::from_bytes(Bytes::from_bytes_vec(bytes[HEADER..].to_vec()))
            .map_err(|e| ModelError::Weights(e.to_string()))?;
        let net = PolicyValueNet::new(header.shape, &device)
            .try_load_record(record)
            .map_err(|e| ModelError::Weights(e.to_string()))?;
        Ok(NeuralModel {
            header,
            sha256: hash,
            net,
            device,
        })
    }

    /// The header of the loaded file.
    pub fn header(&self) -> &ModelHeader {
        &self.header
    }

    /// The SHA-256 hash of the loaded file.
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    /// Encodes a permitted view with the layout recorded in this model.
    pub fn features(&self, view: &PlayerView) -> Vec<f32> {
        match self.header.encoder_version {
            1 => encode_v1(view),
            _ => encode(view),
        }
    }

    /// Raw network outputs for `features`: one logit per action and the
    /// value.
    ///
    /// # Panics
    ///
    /// If `features` does not match this model's input width.
    pub fn forward(&self, features: Vec<f32>) -> (Vec<f32>, f32) {
        let o = self.forward_all(features);
        (o.logits, o.value)
    }

    /// Every raw network output for `features`.
    ///
    /// # Panics
    ///
    /// If `features` does not match this model's input width.
    pub fn forward_all(&self, features: Vec<f32>) -> RawOutputs {
        let input: Tensor<2> = Tensor::from_data(
            TensorData::new(features, [1, self.header.shape.inputs]),
            &self.device,
        );
        let o = self.net.forward_all(input);
        let vec = |t: Tensor<2>| {
            t.into_data()
                .try_to_vec::<f32>()
                .expect("the network outputs f32")
        };
        RawOutputs {
            logits: vec(o.policy),
            value: vec(o.value)[0],
            score: vec(o.score),
            win: vec(o.win),
        }
    }

    /// The model's decision for `view`. Only the view is read (D-23).
    ///
    /// # Errors
    ///
    /// The reason to fall back to Classic: an uncovered phase, no decision
    /// to take, a non-finite output, or a choice that is not legal.
    pub fn evaluate(&self, view: &PlayerView) -> Result<Evaluation, Fallback> {
        self.evaluate_with(view, Choice::Policy)
    }

    /// The model's decision for `view`, picked by `choice`. Only the view
    /// is read (D-23).
    ///
    /// # Errors
    ///
    /// As [`NeuralModel::evaluate`]; a non-finite option estimate of a
    /// legal action also falls back.
    pub fn evaluate_with(&self, view: &PlayerView, choice: Choice) -> Result<Evaluation, Fallback> {
        if view.legal_actions.is_empty() {
            return Err(Fallback::NoDecision);
        }
        if !self.header.phases.contains(view.phase) {
            return Err(Fallback::UnsupportedPhase(view.phase));
        }
        let indices = view
            .legal_actions
            .iter()
            .map(|&a| action_index(a).ok_or(Fallback::UnknownAction(a)))
            .collect::<Result<Vec<usize>, Fallback>>()?;
        let raw = self.forward_all(self.features(view));
        if !raw.value.is_finite() {
            return Err(Fallback::NonFinite);
        }
        let policy = masked_softmax(&raw.logits, &indices).ok_or(Fallback::NonFinite)?;
        let options: Vec<OptionEstimate> = view
            .legal_actions
            .iter()
            .zip(&indices)
            .map(|(&action, &i)| OptionEstimate {
                action,
                score: raw.score[i],
                win_chance: 1.0 / (1.0 + (-raw.win[i]).exp()),
            })
            .collect();
        if options
            .iter()
            .any(|o| !o.score.is_finite() || !o.win_chance.is_finite())
        {
            return Err(Fallback::NonFinite);
        }
        let ranking: Vec<f32> = match choice {
            Choice::Policy => policy.clone(),
            Choice::Score => options.iter().map(|o| o.score).collect(),
        };
        Ok(Evaluation {
            action: choose(view, &ranking, &indices)?,
            policy: view.legal_actions.iter().copied().zip(policy).collect(),
            value: raw.value,
            options,
        })
    }
}

/// Softmax over the logits of the legal `indices` only (the mask is applied
/// before normalisation); `None` if a legal logit is not finite.
pub fn masked_softmax(logits: &[f32], indices: &[usize]) -> Option<Vec<f32>> {
    let legal: Vec<f32> = indices.iter().map(|&i| logits[i]).collect();
    if legal.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let max = legal.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = legal.iter().map(|&x| (x - max).exp()).collect();
    let total: f32 = exp.iter().sum();
    Some(exp.into_iter().map(|e| e / total).collect())
}

/// The legal action ranked highest (ties: first in the view's order),
/// checked once more against the view's legal actions.
fn choose(view: &PlayerView, policy: &[f32], indices: &[usize]) -> Result<Action, Fallback> {
    let mut best = 0;
    for (i, &p) in policy.iter().enumerate() {
        if p > policy[best] {
            best = i;
        }
    }
    let action = crate::actions::action_at(indices[best])
        .ok_or(Fallback::UnknownAction(view.legal_actions[best]))?;
    if view.legal_actions.contains(&action) {
        Ok(action)
    } else {
        Err(Fallback::IllegalChoice(action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_softmax_ignores_illegal_logits() {
        let logits = [5.0, f32::NAN, 1.0, f32::INFINITY, 1.0];
        let p = masked_softmax(&logits, &[0, 2, 4]).unwrap();
        assert_eq!(p.len(), 3);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(p[0] > p[1] && p[1] == p[2]);
        assert_eq!(masked_softmax(&logits, &[0, 1]), None);
        assert_eq!(masked_softmax(&logits, &[3]), None);
    }

    #[test]
    fn phases() {
        assert!(Phases::ALL.contains(Phase::Reizen));
        assert!(!Phases::SPIEL.contains(Phase::Druecken));
        assert!(!Phases::ALL.contains(Phase::Beendet));
        assert_eq!(Phases::from_bits(0), None);
        assert_eq!(Phases::from_bits(0b10_0000), None);
    }
}
