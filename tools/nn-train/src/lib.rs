// SPDX-License-Identifier: EUPL-1.2

//! Library part of the training tool (not part of the app): teacher
//! self-play records ([`selfplay`]) and their sharded, resumable storage
//! ([`shards`]), issue #70, and supervised training of the card-play
//! network on them ([`train`]), issue #73, and its self-play refinement
//! from game outcomes ([`rl`]), issue #75. Training data is held compactly
//! ([`features`]), can be cached on disk ([`cache`]) and, for GPU training,
//! kept in device memory ([`resident`]). The binary `nn-train` drives
//! them.

pub mod cache;
pub mod features;
pub mod probe;
pub mod resident;
pub mod rl;
pub mod selfplay;
pub mod shards;
pub mod train;
