// SPDX-License-Identifier: EUPL-1.2

//! Neural network runtime shared by the opponent AI and the coach (D-07,
//! D-11): the permitted-view [`encoder`], the fixed action space
//! ([`actions`]), the [`net`] definition and model loading and inference
//! ([`NeuralModel`]).
//!
//! Information boundary (D-23): the only game input is one seat's
//! [`PlayerView`](skat_core::PlayerView); never the `Game`, a deal seed or
//! other seats' cards. Models arrive as bytes with their expected SHA-256
//! hash; this crate does no IO. Every problem (missing, corrupt or
//! incompatible model, uncovered phase, non-finite output, illegal choice)
//! is returned as a [`Fallback`] reason, and the caller decides with the
//! Classic engine instead. Model outputs are estimates; they never claim
//! exactness or Monte-Carlo samples.

pub mod actions;
pub mod encoder;
#[cfg(feature = "fixture")]
mod fixture;
mod model;
pub mod net;

pub use actions::{ACTIONS, action_at, action_index};
pub use encoder::{ENCODER_VERSION, FEATURES, encode};
#[cfg(feature = "fixture")]
pub use fixture::{FIXTURE_HIDDEN, fixture_model};
pub use model::{
    Evaluation, FORMAT_VERSION, Fallback, ModelError, ModelHeader, NeuralModel, Phases,
    masked_softmax, read_header, sha256, write_model,
};
pub use net::{NetShape, PolicyValueNet};

#[cfg(test)]
mod tests {
    /// Models are shared between threads (arena workers, the app's
    /// opponent thread).
    #[test]
    fn model_is_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<crate::NeuralModel>();
    }
}
