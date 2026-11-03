# Strong Monte Carlo sampling on the RX 6900 XT

The experiment moves the draw of hidden cards in `analysis::own_worlds`
to Vulkan. One WGSL invocation reproduces one CPU sample. Decisions with
the same allowed places for every unseen card have a single possible
split of the capacities. The shader consumes that split's random draw,
then reproduces the ChaCha8 draws and Fisher–Yates swaps in the order
used by `HiddenCards::sample`. It returns the card masks for the hidden
places. A possible rejection draw retries the decision through the CPU
sampler, since rejection can shift the random stream of subsequent samples.
Constrained distributions, exact enumeration and double-dummy evaluation
use the existing Rust code.

The host sends 48 bytes per decision and reads back 16 bytes per sample.
One background worker reuses the device, pipeline and buffers and collects
requests already queued by other games. Each requesting game needs its
worlds before evaluating its next move. The worker submits immediately
when one decision is ready.

## Reproduce

Use a disposable checkout of this branch with an RX 6900 XT and Vulkan.
The patch connects the experiment to the generator and adds the benchmark
target. Run from the repository root, with a fresh output directory for
each generator run. Prebuild both release configurations to exclude build
time and run the generators sequentially.

```sh
git apply crates/skat-analysis/experiments/strong-gpu/enable.patch
cargo run --locked --release -p skat-analysis --example gpu_samples \
  --features serde,gpu-sampling-bench
cargo build --locked --release -p nn-train
cargo build --locked --release -p nn-train --features skat-analysis/gpu-sampling-bench
cargo run --locked --release -p nn-train --features skat-analysis/gpu-sampling-bench -- \
  generate GPU_DIR --deals 10000 --shard-deals 1 --seed 0 --cpus 20 --workers 4 --minutes 10
cargo run --locked --release -p nn-train -- \
  generate CPU_DIR --deals 10000 --shard-deals 1 --seed 0 --cpus 20 --workers 4 --minutes 10
cargo run --locked --release -p nn-train -- verify GPU_DIR
cargo run --locked --release -p nn-train -- verify CPU_DIR
```

The sample benchmark uses 41 eligible views from the two checked-in Strong
deals. It repeats views with different decision seeds for larger batches,
checks every returned world against the CPU and times 21 warm runs per
batch size. CPU timings include RNG setup and construction of all worlds.
GPU timings include job packing, queueing, upload, dispatch, readback and
reconstruction of the same worlds. Adapter and pipeline initialization
are excluded from the microbenchmark and included in the generator run.

| Samples | Independent decisions | CPU median | GPU median |
|---:|---:|---:|---:|
| 24 | 1 | 31.8 µs | 116.6 µs |
| 240 | 10 | 290.1 µs | 133.5 µs |
| 2,400 | 100 | 2.366 ms | 0.185 ms |
| 24,000 | 1,000 | 22.556 ms | 0.788 ms |

## Complete Strong games

Measured on 2026-11-03 at `27c68241d114` with an i9-10900K (20 logical
CPUs) and an AMD Radeon RX 6900 XT (RADV NAVI21, Mesa 26.2.4). An initial
CPU run that overlapped compilation was discarded. The two measured
generators ran sequentially, GPU first, for ten minutes each plus the time
to finish deals already in progress. Rates use actual elapsed time.

| Generator | Elapsed | Complete deals | With card play | Records | Deals/hour |
|---|---:|---:|---:|---:|---:|
| CPU | 608.5 s | 326 | 323 | 13,874 | 1,929 |
| CPU with GPU sampler | 609.3 s | 322 | 319 | 13,686 | 1,903 |

The measured throughput difference was −1.36% in one pair of runs. Each
included three passed-in deals. Both datasets passed `nn-train verify`.
All 322 common deal files were byte-identical, including option estimates,
actions, final points and the three incomplete teacher analyses. After
4,096 successful GPU decisions the worker had issued 4,091 submissions.
Its largest batch then held four decisions (96 samples).

A separate instrumented CPU run on the first 16 deals timed 502 Strong
analyses with the same worker settings. Summing wall time per decision,
drawing or enumerating worlds took 11.6 ms of 116.5 s (0.010%). Evaluating
the options across those worlds took 115.8 s (99.4%). These are accumulated
decision times across concurrent games. Within the evaluation tasks,
99.98% of the summed task time was spent inside `Solver::solve`.

The exact double-dummy solver visits about 2.08 billion search nodes
over those 16 deals. A thread per world could reuse search results across
sibling options as the CPU does, but would need both recursive searches
(`max_augen` and `forced`) expressed with explicit stacks, move ordering,
the per-task node budget and three transposition tables with 128-bit keys.
Changing pruning or table reuse can change which distributions finish
within the budget. Batching independent games supplies more worlds without
removing that porting work. The measured sampler therefore stays an
experiment outside the regular build.
