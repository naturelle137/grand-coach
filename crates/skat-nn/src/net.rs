// SPDX-License-Identifier: EUPL-1.2

//! The network: a small MLP with a policy head (one logit per action of
//! [`crate::actions`]), a value head (expected Seeger-Fabian points of
//! the acting seat) and two option heads (per action: expected
//! Seeger-Fabian points after it, and the logit of the probability that
//! the seat's party wins after it). The same definition is used for CPU
//! inference here and for training in `tools/nn-train` (D-11).

use burn::module::{Module, Param, ParamId};
use burn::nn::{Linear, LinearConfig, Relu};
use burn::tensor::{Device, Tensor, TensorData};

use crate::actions::ACTIONS;
use crate::encoder::FEATURES;

/// Layer sizes of a [`PolicyValueNet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetShape {
    /// Input features ([`FEATURES`] for the current encoder).
    pub inputs: usize,
    /// Width of every hidden layer.
    pub hidden: usize,
    /// Number of hidden layers (at least [`MIN_LAYERS`]).
    pub layers: usize,
    /// Policy outputs ([`ACTIONS`] for the current encoder).
    pub actions: usize,
}

/// Fewest hidden layers of a [`PolicyValueNet`] (the shape of every model
/// file written before the layer count was recorded).
pub const MIN_LAYERS: usize = 2;

/// Most hidden layers a model file may declare.
pub const MAX_LAYERS: usize = 8;

impl NetShape {
    /// The shape for the current encoder with [`MIN_LAYERS`] hidden layers
    /// of `hidden`.
    pub const fn current(hidden: usize) -> NetShape {
        NetShape {
            inputs: FEATURES,
            hidden,
            layers: MIN_LAYERS,
            actions: ACTIONS,
        }
    }

    /// This shape with `layers` hidden layers.
    pub const fn with_layers(self, layers: usize) -> NetShape {
        NetShape { layers, ..self }
    }

    /// Hidden layers after the second one.
    fn deeper(self) -> usize {
        self.layers.saturating_sub(MIN_LAYERS)
    }
}

/// Outputs of a [`PolicyValueNet`] for a batch.
#[derive(Clone, Debug)]
pub struct Outputs {
    /// Policy logits `[batch, actions]`.
    pub policy: Tensor<2>,
    /// Value `[batch, 1]`: expected Seeger-Fabian points of the seat.
    pub value: Tensor<2>,
    /// Option scores `[batch, actions]`: expected Seeger-Fabian points
    /// after each action.
    pub score: Tensor<2>,
    /// Option win logits `[batch, actions]`: logit of the probability that
    /// the seat's party wins after each action.
    pub win: Tensor<2>,
}

/// Hidden ReLU layers of equal width (`input`, `hidden`, then `deeper`),
/// then a policy, a value and two option heads. Field names are the keys of
/// the weights in a model file; a two-layer network has no `deeper` entry,
/// so files written before deeper networks existed load unchanged.
#[derive(Module, Debug)]
pub struct PolicyValueNet {
    input: Linear,
    hidden: Linear,
    deeper: Vec<Linear>,
    policy: Linear,
    value: Linear,
    score: Linear,
    win: Linear,
    activation: Relu,
}

impl PolicyValueNet {
    /// A network of `shape` with Burn's default initialisation on `device`.
    /// Its weights depend on the device's random state; load a record or use
    /// [`PolicyValueNet::from_weights`] for reproducible weights.
    pub fn new(shape: NetShape, device: &Device) -> PolicyValueNet {
        PolicyValueNet {
            input: LinearConfig::new(shape.inputs, shape.hidden).init(device),
            hidden: LinearConfig::new(shape.hidden, shape.hidden).init(device),
            deeper: (0..shape.deeper())
                .map(|_| LinearConfig::new(shape.hidden, shape.hidden).init(device))
                .collect(),
            policy: LinearConfig::new(shape.hidden, shape.actions).init(device),
            value: LinearConfig::new(shape.hidden, 1).init(device),
            score: LinearConfig::new(shape.hidden, shape.actions).init(device),
            win: LinearConfig::new(shape.hidden, shape.actions).init(device),
            activation: Relu,
        }
    }

    /// A network of `shape` whose parameters are taken in order from
    /// `next`: for each layer (input, the hidden layers, policy, value, score, win) its weight
    /// matrix row by row (`[inputs, outputs]`), then its bias.
    pub fn from_weights(
        shape: NetShape,
        device: &Device,
        mut next: impl FnMut() -> f32,
    ) -> PolicyValueNet {
        // Fixed parameter ids keep the serialised file reproducible.
        let mut id = 0u64;
        let mut layer = |inputs: usize, outputs: usize| {
            let mut linear = LinearConfig::new(inputs, outputs).init(device);
            let weight: Vec<f32> = (0..inputs * outputs).map(|_| next()).collect();
            let bias: Vec<f32> = (0..outputs).map(|_| next()).collect();
            let weight: Tensor<2> =
                Tensor::from_data(TensorData::new(weight, [inputs, outputs]), device);
            let bias: Tensor<1> = Tensor::from_data(TensorData::new(bias, [outputs]), device);
            linear.weight = Param::initialized(ParamId::from(id + 1), weight);
            linear.bias = Some(Param::initialized(ParamId::from(id + 2), bias));
            id += 2;
            linear
        };
        PolicyValueNet {
            input: layer(shape.inputs, shape.hidden),
            hidden: layer(shape.hidden, shape.hidden),
            deeper: (0..shape.deeper())
                .map(|_| layer(shape.hidden, shape.hidden))
                .collect(),
            policy: layer(shape.hidden, shape.actions),
            value: layer(shape.hidden, 1),
            score: layer(shape.hidden, shape.actions),
            win: layer(shape.hidden, shape.actions),
            activation: Relu,
        }
    }

    /// Every output for features `[batch, inputs]`.
    pub fn forward_all(&self, features: Tensor<2>) -> Outputs {
        let x = self.activation.forward(self.input.forward(features));
        let mut x = self.activation.forward(self.hidden.forward(x));
        for layer in &self.deeper {
            x = self.activation.forward(layer.forward(x));
        }
        Outputs {
            policy: self.policy.forward(x.clone()),
            value: self.value.forward(x.clone()),
            score: self.score.forward(x.clone()),
            win: self.win.forward(x),
        }
    }

    /// Policy logits `[batch, actions]` and values `[batch, 1]` for
    /// features `[batch, inputs]`.
    pub fn forward(&self, features: Tensor<2>) -> (Tensor<2>, Tensor<2>) {
        let o = self.forward_all(features);
        (o.policy, o.value)
    }
}
