// SPDX-License-Identifier: EUPL-1.2

//! Optional, bounded Vulkan proof service shared by the generator's games.

#[path = "../../../crates/skat-analysis/experiments/strong-gpu/gpu_batch.rs"]
mod gpu_batch;

use gpu_batch::Backend;
use skat_analysis::double_dummy::endgame::{self, Device, Job};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

const SHADER: &str =
    include_str!("../../../crates/skat-analysis/experiments/strong-gpu/threshold.wgsl");
const BATCH: usize = 256;
type Request = (Job, mpsc::Sender<u32>);

#[derive(Default)]
struct Stats {
    batches: AtomicU64,
    jobs: AtomicU64,
    gpu_ns: AtomicU64,
    total_ns: AtomicU64,
    peak: AtomicUsize,
    measured: AtomicU64,
    cpu_ns: AtomicU64,
    disabled: AtomicBool,
    roots: AtomicU64,
    subtrees: AtomicU64,
    misses: AtomicU64,
    samples: Mutex<Vec<(Job, bool, Duration)>>,
}

struct Service {
    tx: Option<mpsc::SyncSender<Request>>,
    waiting: AtomicUsize,
    max_waiting: usize,
    stats: Arc<Stats>,
}

impl Device for Service {
    fn solve(&self, job: &dyn Fn() -> Job, reference: &dyn Fn() -> u32) -> Option<u32> {
        let queue = self.tx.as_ref()?;
        if self.stats.disabled.load(Relaxed) {
            return None;
        }
        // Reserve CPU lanes for other ready worlds during a dispatch.
        if self.waiting.fetch_add(1, Relaxed) >= self.max_waiting {
            self.waiting.fetch_sub(1, Relaxed);
            return None;
        }
        let start = Instant::now();
        let (reply, result) = mpsc::channel();
        let value = queue.try_send((job(), reply)).ok().and_then(|()| {
            // Calibrate on the first real requests. Count work and verdict
            // together: scheduling and adapter loss must never shift budgets.
            let check = (self.stats.measured.load(Relaxed) < 1024).then(|| {
                let start = Instant::now();
                let expected = reference();
                self.stats.cpu_ns.fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
                expected
            });
            let actual = result.recv().ok()?;
            if let Some(expected) = check {
                assert_eq!(actual, expected, "device proof or transition count differs");
                if self.stats.measured.fetch_add(1, Relaxed) == 1023 &&
                    self.stats.gpu_ns.load(Relaxed) >= self.stats.cpu_ns.load(Relaxed) {
                    self.stats.disabled.store(true, Relaxed);
                    eprintln!("Vulkan proof dispatch took more time than its CPU work across 1024 real queries; continuing on CPU");
                }
            }
            Some(actual)
        });
        self.stats
            .total_ns
            .fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
        self.waiting.fetch_sub(1, Relaxed);
        value
    }

    fn times(&self, roots: Duration, subtrees: Duration, misses: u64) {
        self.stats.roots.fetch_add(roots.as_nanos() as u64, Relaxed);
        self.stats
            .subtrees
            .fetch_add(subtrees.as_nanos() as u64, Relaxed);
        self.stats.misses.fetch_add(misses, Relaxed);
    }

    fn observe(&self, job: Job, answer: bool, elapsed: Duration) {
        let mut samples = self.stats.samples.lock().unwrap();
        if samples.len() < 32768 {
            samples.push((job, answer, elapsed));
        }
    }
}

pub(crate) struct Run {
    stats: Arc<Stats>,
    replay: Option<Backend>,
}

fn adapter() -> Option<Backend> {
    Backend::new(
        SHADER,
        (size_of::<Job>() * BATCH) as u64,
        (4 * BATCH) as u64,
    )
}

impl Run {
    pub(crate) fn start(mode: &str, depth: u8, cpus: usize) -> Self {
        let stats = Arc::new(Stats::default());
        let profile = mode == "profile";
        let mut replay = None;
        let mut tx = None;
        match mode {
            "profile" => replay = adapter(),
            "cpu-endgame" => {}
            "vulkan" => {
                if let Some(gpu) = adapter() {
                    let (send, recv) = mpsc::sync_channel::<Request>(BATCH);
                    tx = Some(send);
                    let stats = Arc::clone(&stats);
                    std::thread::spawn(move || {
                        while let Ok(first) = recv.recv() {
                            let mut batch = vec![first];
                            batch.extend(recv.try_iter().take(BATCH - 1));
                            let jobs: Vec<_> = batch.iter().map(|r| r.0).collect();
                            let start = Instant::now();
                            let Some(results) =
                                gpu.run::<u32>(bytemuck::cast_slice(&jobs), jobs.len())
                            else {
                                break;
                            };
                            stats
                                .gpu_ns
                                .fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
                            stats.batches.fetch_add(1, Relaxed);
                            stats.jobs.fetch_add(jobs.len() as u64, Relaxed);
                            stats.peak.fetch_max(jobs.len(), Relaxed);
                            for ((_, reply), value) in batch.into_iter().zip(results) {
                                let _ = reply.send(value);
                            }
                        }
                    });
                } else {
                    eprintln!("Vulkan unavailable; using the CPU endgame solver");
                }
            }
            _ => panic!("unknown search backend {mode}"),
        }
        let service = Service {
            tx,
            waiting: AtomicUsize::new(0),
            max_waiting: (cpus / 2).max(1),
            stats: Arc::clone(&stats),
        };
        endgame::configure(depth, profile, Box::new(service)).expect("configure search backend");
        Self { stats, replay }
    }

    pub(crate) fn report(&self) {
        let s = &self.stats;
        if s.roots.load(Relaxed) > 0 {
            eprintln!(
                "early Augen search: {:.3} s; frontier misses: {}; subtree time: {:.3} s ({:.1}%)",
                s.roots.load(Relaxed) as f64 / 1e9,
                s.misses.load(Relaxed),
                s.subtrees.load(Relaxed) as f64 / 1e9,
                100.0 * s.subtrees.load(Relaxed) as f64 / s.roots.load(Relaxed) as f64
            );
        }
        if let Some(gpu) = &self.replay {
            let samples = s.samples.lock().unwrap();
            let mut done = 0;
            let mut saved = Duration::ZERO;
            let start = Instant::now();
            for chunk in samples.chunks(BATCH) {
                let jobs: Vec<_> = chunk.iter().map(|r| r.0).collect();
                let Some(results) = gpu.run::<u32>(bytemuck::cast_slice(&jobs), jobs.len()) else {
                    eprintln!("GPU replay interrupted");
                    return;
                };
                for (result, (_, expected, cpu_time)) in results.into_iter().zip(chunk) {
                    if result & 3 < 2 {
                        assert_eq!(
                            result & 3 == 1,
                            *expected,
                            "GPU disagrees with reference subtree"
                        );
                        done += 1;
                        saved += *cpu_time;
                    }
                }
            }
            eprintln!(
                "batched replay: {done}/{} proofs, CPU time of those subtrees {:.3} ms, Vulkan round trips {:.3} ms",
                samples.len(),
                saved.as_secs_f64() * 1000.,
                start.elapsed().as_secs_f64() * 1000.
            );
        }
        if s.batches.load(Relaxed) > 0 {
            eprintln!(
                "GPU: {} jobs / {} batches (peak {}), {:.3} s dispatch and readback, {:.3} s summed request latency",
                s.jobs.load(Relaxed),
                s.batches.load(Relaxed),
                s.peak.load(Relaxed),
                s.gpu_ns.load(Relaxed) as f64 / 1e9,
                s.total_ns.load(Relaxed) as f64 / 1e9
            );
        }
    }
}
