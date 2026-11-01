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
//! | 21 | 3 | reserved, zero |
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
use crate::encoder::{ENCODER_VERSION, FEATURES, encode, phase_index};
use crate::net::{NetShape, PolicyValueNet};

/// Version of the model file layout.
pub const FORMAT_VERSION: u16 = 1;

const MAGIC: [u8; 4] = *b"GCNN";
const HEADER: usize = 24;
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

/// A model's choice for one decision. The probabilities and the value are
/// model estimates: never exact, never Monte-Carlo samples.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// The chosen legal action: the most probable one, ties to the first
    /// in the view's order.
    pub action: Action,
    /// Probability of each legal action, in the view's order; illegal
    /// actions are masked out before normalisation.
    pub policy: Vec<(Action, f32)>,
    /// Estimated Seeger-Fabian points of the acting seat for this game.
    pub value: f32,
}

/// Serialises `net` with `header` into a model file.
///
/// # Errors
///
/// If Burn cannot serialise the weights.
pub fn write_model(header: &ModelHeader, net: PolicyValueNet) -> Result<Vec<u8>, ModelError> {
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
    out.extend_from_slice(&[0; 3]);
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
    if encoder != ENCODER_VERSION {
        return Err(ModelError::Encoder { found: encoder });
    }
    let (inputs, hidden, actions) = (u32_at(bytes, 8), u32_at(bytes, 12), u32_at(bytes, 16));
    let fits = inputs as usize == FEATURES
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
    Ok(ModelHeader {
        encoder_version: encoder,
        shape: NetShape::current(hidden as usize),
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

    /// Raw network outputs for `features`: one logit per action and the
    /// value.
    ///
    /// # Panics
    ///
    /// If `features` does not hold [`FEATURES`] numbers.
    pub fn forward(&self, features: Vec<f32>) -> (Vec<f32>, f32) {
        let input: Tensor<2> =
            Tensor::from_data(TensorData::new(features, [1, FEATURES]), &self.device);
        let (logits, value) = self.net.forward(input);
        let logits = logits
            .into_data()
            .try_to_vec::<f32>()
            .expect("the network outputs f32");
        let value = value
            .into_data()
            .try_to_vec::<f32>()
            .expect("the network outputs f32");
        (logits, value[0])
    }

    /// The model's decision for `view`. Only the view is read (D-23).
    ///
    /// # Errors
    ///
    /// The reason to fall back to Classic: an uncovered phase, no decision
    /// to take, a non-finite output, or a choice that is not legal.
    pub fn evaluate(&self, view: &PlayerView) -> Result<Evaluation, Fallback> {
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
        let (logits, value) = self.forward(encode(view));
        if !value.is_finite() {
            return Err(Fallback::NonFinite);
        }
        let policy = masked_softmax(&logits, &indices).ok_or(Fallback::NonFinite)?;
        Ok(Evaluation {
            action: choose(view, &policy, &indices)?,
            policy: view.legal_actions.iter().copied().zip(policy).collect(),
            value,
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

/// The most probable legal action (ties: first in the view's order),
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
