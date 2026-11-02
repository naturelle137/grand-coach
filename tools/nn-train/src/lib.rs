// SPDX-License-Identifier: EUPL-1.2

//! Library part of the training tool (not part of the app): teacher
//! self-play records ([`selfplay`]) and their sharded, resumable storage
//! ([`shards`]), issue #70, and supervised training of the card-play
//! network on them ([`train`]), issue #73. The binary `nn-train` drives
//! them.

pub mod selfplay;
pub mod shards;
pub mod train;
