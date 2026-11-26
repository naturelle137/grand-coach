// SPDX-License-Identifier: EUPL-1.2

//! Godot frontend of Grand-Coach (M9, D-11, D-12).
//!
//! [`adapter`] is the Godot-free boundary: JSON text in, JSON text out, built
//! only on `skat-app`. [`node`] wraps it in a gdext class for GDScript. Game
//! behaviour never lives here (AGENTS.md hard rules).

pub mod adapter;
pub mod node;
