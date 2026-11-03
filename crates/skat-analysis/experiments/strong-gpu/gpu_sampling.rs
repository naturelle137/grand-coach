// SPDX-License-Identifier: EUPL-1.2

//! Experimental RX 6900 XT sampler for single-group hidden distributions.
use crate::sampling::{HiddenCards, Place, World};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use skat_core::CardSet;
use std::sync::{OnceLock, mpsc};

/// Number of distributions in the largest experiment.
const CAP: usize = 24_000;

#[path = "gpu_batch.rs"]
mod gpu_batch;
use gpu_batch::Backend;

// Game workers may submit while the preceding batch executes. Drain only
// requests already waiting, so a lone decision never waits to fill a batch.
type Reply = mpsc::Sender<Option<Vec<[u32; 4]>>>;
type Request = (Vec<[u32; 12]>, Reply);

fn submit(jobs: Vec<[u32; 12]>) -> Option<Vec<[u32; 4]>> {
    static QUEUE: OnceLock<Option<mpsc::Sender<Request>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let gpu = Backend::new(include_str!("sample.wgsl"),48 * (CAP/24) as u64, 16 * CAP as u64)?;
        let (tx, rx) = mpsc::channel::<Request>();
        std::thread::spawn(move || {
            let mut pending = None;
            let (mut completed, mut submissions, mut largest) = (0, 0, 0);
            while let Some((mut jobs, first)) = pending.take().or_else(|| rx.recv().ok()) {
                let mut replies = vec![(jobs.len() * 24, first)];
                while let Ok((more, reply)) = rx.try_recv() {
                    if jobs.len() + more.len() > CAP / 24 {
                        pending = Some((more, reply));
                        break;
                    }
                    replies.push((more.len() * 24, reply));
                    jobs.extend(more);
                }
                let rows = gpu.run(bytemuck::cast_slice(&jobs),jobs.len()*24);
                if rows.is_some() {
                    let previous = completed;
                    completed += jobs.len();
                    submissions += 1;
                    largest = largest.max(jobs.len());
                    if previous == 0 || previous / 1024 != completed / 1024 {
                        eprintln!("GPU sampler: {completed} decisions in {submissions} submissions, largest {largest}");
                    }
                }
                let mut start = 0;
                for (len, reply) in replies {
                    let _ = reply.send(rows.as_ref().map(|r| r[start..start+len].to_vec()));
                    start += len;
                }
            }
        });
        Some(tx)
    }).as_ref()?;
    let (tx, rx) = mpsc::channel();
    queue.send((jobs, tx)).ok()?;
    rx.recv().ok()?
}

/// One job per independent decision, each with its own original seed and 24 samples.
pub fn batch(inputs: &[(&HiddenCards, u64)]) -> Option<Vec<Vec<World>>> {
    if inputs.is_empty() || inputs.len() > CAP / 24 {
        return None;
    }
    let jobs = inputs
        .iter()
        .map(|&(h, seed)| {
            let slots = h.slots();
            if h.count() <= 24
                || h.count() > u32::MAX.into()
                || slots.iter().any(|s| s.allowed != h.unseen())
            {
                return None;
            }
            let key = ChaCha8Rng::seed_from_u64(seed).get_seed();
            let mut job = [0; 12];
            for (i, k) in key.as_chunks::<4>().0.iter().enumerate() {
                job[i] = u32::from_le_bytes(*k);
            }
            job[8..].copy_from_slice(&[
                h.unseen().bits(),
                slots[0].size.into(),
                slots.get(1).map_or(0, |s| s.size.into()),
                24,
            ]);
            Some(job)
        })
        .collect::<Option<Vec<_>>>()?;
    let rows = submit(jobs)?;
    inputs
        .iter()
        .zip(rows.as_chunks::<24>().0)
        .map(|(&(h, _), rows)| {
            rows.iter()
                .map(|row| {
                    if row[3] != 0 {
                        return None;
                    }
                    let mut world = h.known();
                    for (slot, &bits) in h.slots().iter().zip(row) {
                        let mask = CardSet::from_bits(bits);
                        match slot.place {
                            Place::Seat(s) => world.hands[s.index()] |= mask,
                            Place::Skat => world.skat |= mask,
                        }
                    }
                    Some(world)
                })
                .collect()
        })
        .collect()
}

pub(crate) fn sample(h: &HiddenCards, samples: usize, seed: u64) -> Option<Vec<World>> {
    if samples != 24 {
        return None;
    }
    batch(&[(h, seed)])?.pop()
}
