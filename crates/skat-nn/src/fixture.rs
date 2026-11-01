// SPDX-License-Identifier: EUPL-1.2

//! Tiny random-weight models for tests (feature `fixture`). They play
//! legal but meaningless Skat; their purpose is to exercise loading,
//! masking, fallbacks and the arena on every platform without a trained
//! model in the repository.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

use crate::encoder::ENCODER_VERSION;
use crate::model::{ModelHeader, Phases, write_model};
use crate::net::{NetShape, PolicyValueNet};

/// Hidden width of the fixture models.
pub const FIXTURE_HIDDEN: usize = 8;

/// A model file with weights drawn uniformly from [-0.5, 0.5) by a
/// `ChaCha8Rng` seeded with `seed`, covering `phases`. The same arguments
/// give the same weights on every platform.
pub fn fixture_model(seed: u64, phases: Phases) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let shape = NetShape::current(FIXTURE_HIDDEN);
    let net = PolicyValueNet::from_weights(shape, &burn::tensor::Device::flex(), || {
        (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32 - 0.5
    });
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape,
        phases,
    };
    write_model(&header, net).expect("a fixture model serialises")
}
