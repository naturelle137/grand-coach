// SPDX-License-Identifier: EUPL-1.2

//! Round-trip benchmark using permitted Strong fixture views.
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use skat_analysis::{gpu_sampling::batch, sampling::HiddenCards};
use skat_core::PlayerView;
use std::time::{Duration, Instant};

fn median(mut f: impl FnMut()) -> Duration {
    let mut times = Vec::new();
    for _ in 0..21 {
        let start = Instant::now();
        f();
        times.push(start.elapsed());
    }
    times.sort();
    times[10]
}

fn main() {
    let mut states = Vec::new();
    for data in [
        include_str!("../../../../tools/nn-train/fixtures/selfplay/shard-00000.jsonl"),
        include_str!("../../../../tools/nn-train/fixtures/selfplay/shard-00001.jsonl"),
    ] {
        for line in data.lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            let view: PlayerView = serde_json::from_value(row["view"].clone()).unwrap();
            if row["teacher"]["status"].is_null() {
                continue;
            }
            let h = HiddenCards::from_view(&view).unwrap();
            if h.count() > 24 && h.slots().iter().all(|s| s.allowed == h.unseen()) {
                states.push(h);
            }
        }
    }
    println!("{} eligible Strong fixture views", states.len());
    for decisions in [1, 10, 100, 1000] {
        let jobs: Vec<_> = (0..decisions)
            .map(|i| (&states[i % states.len()], i as u64))
            .collect();
        let cpu = || {
            jobs.iter()
                .map(|&(h, seed)| {
                    let mut rng = ChaCha8Rng::seed_from_u64(seed);
                    (0..24).map(|_| h.sample(&mut rng)).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let gpu = || batch(&jobs).expect("RX 6900 XT job failed");
        assert_eq!(cpu(), gpu());
        let gpu_time = median(|| {
            std::hint::black_box(gpu());
        });
        let cpu_time = median(|| {
            std::hint::black_box(cpu());
        });
        println!(
            "{} worlds: CPU {cpu_time:?}, GPU round trip {gpu_time:?}",
            decisions * 24
        );
    }
}
