// SPDX-License-Identifier: EUPL-1.2

//! Optional generator experiment. Ordinary analysis has no device dependency.
//! Each proof receives one sampled distribution at the start of a trick.

use super::{Cards, Node};
use crate::{Interrupted, Meter};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const LIMIT: u32 = 8192;

/// Packed input shared with the integer Vulkan threshold kernel.
pub type Job = [u32; 52];

/// A process-wide service selected once by the standalone generator.
pub trait Device: Sync + Send {
    /// A bounded Boolean proof and its transition count in the upper bits.
    /// Queue pressure or device loss returns control to the CPU.
    fn solve(&self, job: &dyn Fn() -> Job, reference: &dyn Fn() -> u32) -> Option<u32>;
    /// Total early root and eligible subtree times for one world.
    fn times(&self, roots: Duration, subtrees: Duration, misses: u64);
    /// Records a completed, sampled reference subtree for batched replay.
    fn observe(&self, job: Job, answer: bool, elapsed: Duration);
}

struct Config {
    depth: u8,
    profile: bool,
    device: Box<dyn Device>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Selects the frontier for one generator process. Profiling times a
/// deterministic subset of CPU cache misses from the first three tricks.
pub fn configure(depth: u8, profile: bool, device: Box<dyn Device>) -> Result<(), &'static str> {
    if !(2..=4).contains(&depth) {
        return Err("rest-tricks must be 2, 3 or 4");
    }
    CONFIG
        .set(Config {
            depth,
            profile,
            device,
        })
        .map_err(|_| "search backend already configured")
}

pub(super) struct Probe {
    config: Option<&'static Config>,
    early: bool,
    misses: u64,
    pub(super) root_time: Duration,
    subtree_time: Duration,
}

pub(super) enum Visit {
    Bound(bool),
    Trace(Option<(bool, Instant)>),
}

impl Probe {
    pub(super) fn new() -> Self {
        Self {
            config: CONFIG.get(),
            early: false,
            misses: 0,
            root_time: Duration::ZERO,
            subtree_time: Duration::ZERO,
        }
    }

    pub(super) fn start_root(&mut self, tricks_played: u8) -> Option<Instant> {
        self.early = tricks_played < 3;
        (self.early && self.config.is_some_and(|c| c.profile)).then(Instant::now)
    }

    pub(super) fn frontier(
        &mut self,
        n: &Node,
        cards: &Cards,
        decl: u8,
        target: u8,
        meter: &mut Meter,
    ) -> Result<Visit, Interrupted> {
        let Some(config) = self.config else {
            return Ok(Visit::Trace(None));
        };
        if n.hands[0].count_ones() != u32::from(config.depth) {
            return Ok(Visit::Trace(None));
        }
        if config.profile {
            if !self.early {
                return Ok(Visit::Trace(None));
            }
            self.misses += 1;
            // Systematic sampling within a world, without changing its search.
            return Ok(Visit::Trace(Some((self.misses % 256 == 1, Instant::now()))));
        }
        let reference = || {
            let mut visited = 0;
            let answer = cpu(n, cards, decl, target, &mut visited);
            (visited << 2) | answer.map_or(2, u32::from)
        };
        let result = config
            .device
            .solve(&|| pack(n, cards, decl, target), &reference)
            .unwrap_or_else(reference);
        meter.charge(result >> 2)?;
        Ok(match result & 3 {
            0 => Visit::Bound(false),
            1 => Visit::Bound(true),
            _ => Visit::Trace(None),
        })
    }

    pub(super) fn finish(
        &mut self,
        (sample, start): (bool, Instant),
        job: impl FnOnce() -> Job,
        answer: bool,
    ) {
        let elapsed = start.elapsed();
        self.subtree_time += elapsed;
        if sample && let Some(config) = self.config {
            config.device.observe(job(), answer, elapsed);
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        if let Some(c) = self.config
            && c.profile
        {
            c.device
                .times(self.root_time, self.subtree_time, self.misses);
        }
    }
}

pub(super) fn pack(n: &Node, cards: &Cards, decl: u8, target: u8) -> Job {
    // The GPU sees solver-local card IDs in descending strength, with a
    // class and rank table derived from the same rules as the CPU.
    let mut job = [0; 52];
    job[..3].copy_from_slice(&n.hands);
    job[3] = n.turn.into();
    job[10] = n.remaining.into();
    job[12] = decl.into();
    job[13] = target.into();
    job[14] = LIMIT | (1 << 31);
    for (class, &suit) in cards.suits.iter().enumerate() {
        job[47 + class] = suit;
        let mut bits = suit;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            job[15 + i] = class as u32 | ((31 - i as u32) << 3) | (u32::from(cards.points[i]) << 8);
        }
    }
    job
}

// Same move order and transition accounting as threshold.wgsl, including
// an unresolved proof at the cap. A fallback cannot change budget outcomes.
fn cpu(n: &Node, cards: &Cards, decl: u8, target: u8, visited: &mut u32) -> Option<bool> {
    if target == 0 {
        return Some(true);
    }
    if n.remaining < target {
        return Some(false);
    }
    let hand = n.hands[usize::from(n.turn)];
    let follow = hand & cards.suit[usize::from(n.led)];
    let mut moves = if n.count > 0 && follow != 0 {
        follow
    } else {
        hand
    };
    let wants = n.turn == decl;
    while moves != 0 {
        if *visited == LIMIT {
            return None;
        }
        *visited += 1;
        let mut child = *n;
        let gain = child.play(moves.trailing_zeros() as u8, cards, decl);
        if cpu(&child, cards, decl, target.saturating_sub(gain), visited)? == wants {
            return Some(wants);
        }
        moves &= moves - 1;
    }
    Some(!wants)
}
