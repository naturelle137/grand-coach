// SPDX-License-Identifier: EUPL-1.2

//! The network: a small MLP with a policy head (one logit per action of
//! [`crate::actions`]) and a value head (expected Seeger-Fabian points of
//! the acting seat). The same definition is used for CPU inference here
//! and for training in `tools/nn-train` (D-11).

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
    /// Width of both hidden layers.
    pub hidden: usize,
    /// Policy outputs ([`ACTIONS`] for the current encoder).
    pub actions: usize,
}

impl NetShape {
    /// The shape for the current encoder with hidden layers of `hidden`.
    pub const fn current(hidden: usize) -> NetShape {
        NetShape {
            inputs: FEATURES,
            hidden,
            actions: ACTIONS,
        }
    }
}

/// Two hidden ReLU layers, then a policy and a value head.
#[derive(Module, Debug)]
pub struct PolicyValueNet {
    input: Linear,
    hidden: Linear,
    policy: Linear,
    value: Linear,
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
            policy: LinearConfig::new(shape.hidden, shape.actions).init(device),
            value: LinearConfig::new(shape.hidden, 1).init(device),
            activation: Relu,
        }
    }

    /// A network of `shape` whose parameters are taken in order from
    /// `next`: for each layer (input, hidden, policy, value) its weight
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
            policy: layer(shape.hidden, shape.actions),
            value: layer(shape.hidden, 1),
            activation: Relu,
        }
    }

    /// Policy logits `[batch, actions]` and values `[batch, 1]` for
    /// features `[batch, inputs]`.
    pub fn forward(&self, features: Tensor<2>) -> (Tensor<2>, Tensor<2>) {
        let x = self.activation.forward(self.input.forward(features));
        let x = self.activation.forward(self.hidden.forward(x));
        (self.policy.forward(x.clone()), self.value.forward(x))
    }
}
