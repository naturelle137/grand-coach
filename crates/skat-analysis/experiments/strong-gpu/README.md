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

## Decision latency

On the same machine, one interactive analysis at a time used the first
30 positions in each early-play and Kontra/Re class from the existing
`skat-ai` latency benchmark. Both searches saw exactly the same worlds,
node budget and legal options. The reference uses the original solver.
The experimental column includes the coordinator that publishes live
analysis snapshots. Times are seconds, measured from the permitted view
to all final option estimates, before UI rendering.

| Decision | Worlds | CPU median / p90 | Experimental median / p90 | Experimental max |
|---|---:|---:|---:|---:|
| First trick | 6 | 0.249 / 1.159 | 0.245 / 1.052 | 4.036 |
| First trick | 24 | 0.742 / 2.242 | 0.709 / 1.932 | 5.578 |
| Kontra/Re | 6 | 0.128 / 0.507 | 0.122 / 0.401 | 1.793 |
| Kontra/Re | 24 | 0.309 / 2.483 | 0.301 / 2.357 | 3.355 |

All 120 comparisons matched in final option estimates and status. At 24
samples the new bound reduced searched nodes by 8.5% in the first trick
and 4.9% for Kontra/Re. Of the 60 decisions at this budget, 42 needed a
preview. All 42 callbacks arrived at 200 ms (whole milliseconds), with
1–23 fully evaluated worlds. The others finished before the deadline.

For larger budgets only the first position of each class was used. These
measurements describe individual positions and do not estimate p90.

```sh
cargo run --locked --release -p skat-ai --example latency -- 30 experimental 6
cargo run --locked --release -p skat-ai --example latency -- 30 experimental 24
cargo run --locked --release -p skat-ai --example latency -- 1 experimental 240
cargo run --locked --release -p skat-ai --example latency -- 1 experimental 2400
```

| Decision | Worlds | CPU | Experimental | Complete worlds at 200 ms |
|---|---:|---:|---:|---:|
| First trick | 240 | 11.85 s | 11.03 s | 1 |
| First trick | 2,400 | 118.11 s | 117.22 s | 1 |
| Kontra/Re | 240 | 0.37 s | 0.35 s | 167 |
| Kontra/Re | 2,400 | 3.70 s | 2.95 s | 150 |

All four final option vectors and statuses matched. The 2,400-world
first-trick example still searched 7.99 billion nodes with the bound.
The one-second p90 goal was missed even at 24 worlds. The subsequent
[playing-strength validation](../../../../docs/ai.md#experimental-versus-strong)
used 200 independent deals.

## GPU threshold probe

In a disposable checkout with `enable.patch` applied, run:

```sh
cargo run --locked --release -p skat-analysis --example threshold_probe \
  --features serde,gpu-sampling-bench
```

The probe asks whether the declarer can force at least 61 Augen after a
legal card play. It draws hidden cards from the acting player's view in
the checked-in Kreuz fixture at tricks 1, 4 and 8. It cycles through the
legal cards and samples with replacement to hold the batch sizes fixed.
A GPU invocation runs Boolean minimax on one position using an explicit
search stack. The kernel reports unresolved when it reaches its work cap.
Every conclusive answer is checked against the CPU threshold solver.
The hybrid timing includes packing, transfers, dispatch, readback and
the original CPU search for unresolved queries. The CPU-only column uses
the same positions and thread budget. These timings cover one threshold
per position. A full Strong decision needs all its options and additional
thresholds or trick objectives.

| Trick | Batch | GPU cap | Proved on GPU | CPU only | Hybrid |
|---:|---:|---:|---:|---:|---:|
| 1 | 24 | 8,192 | 0 | 230.86 ms | 255.68 ms |
| 1 | 240 | 8,192 | 0 | 1,229.94 ms | 1,271.13 ms |
| 1 | 2,400 | 8,192 | 0 | 11,581.88 ms | 12,085.96 ms |
| 4 | 24 | 8,192 | 16 | 1.21 ms | 18.34 ms |
| 4 | 240 | 8,192 | 172 | 3.75 ms | 36.47 ms |
| 4 | 2,400 | 8,192 | 1,685 | 27.02 ms | 51.51 ms |
| 8 | 24 | 8,192 | 24 | 0.28 ms | 0.42 ms |
| 8 | 240 | 8,192 | 240 | 0.67 ms | 0.37 ms |
| 8 | 2,400 | 8,192 | 2,400 | 1.01 ms | 0.65 ms |

At the smaller cap of 512 transitions, the opening still had zero GPU
proofs. In the eighth trick every query finished on the GPU at both caps.
This late-game saving is under a millisecond on the tested batch.
The bounded GPU search uses neither the CPU solver's transposition tables
nor its move ordering. The probe remains outside the interactive path.

## Reproduce the teacher comparison

Use a clean disposable checkout of this branch and a fresh output directory.
The standalone measurement patch selects Experimental for every seat and
records its final option values. Both teachers use 24 samples.

```sh
bench=$(mktemp -d)
cargo build --locked --release -p nn-train
cp target/release/nn-train "$bench/strong"
git apply crates/skat-analysis/experiments/strong-gpu/dataset-teacher.patch
cargo build --locked --release -p nn-train
cp target/release/nn-train "$bench/experimental"
git apply -R crates/skat-analysis/experiments/strong-gpu/dataset-teacher.patch

for teacher in strong experimental; do
    "$bench/$teacher" generate "$bench/$teacher-fixed" --deals 100 --shard-deals 10 \
        --minutes 30 --seed 0 --cpus 20 --workers 4 --max-mb 1024
    "$bench/$teacher" verify "$bench/$teacher-fixed"
done
for teacher in experimental strong; do
    "$bench/$teacher" generate "$bench/$teacher-timed" --deals 10000 --shard-deals 1 \
        --minutes 10 --seed 0 --cpus 20 --workers 4 --max-mb 1024
    "$bench/$teacher" verify "$bench/$teacher-timed"
done
```

See [training-data throughput](../../../../docs/ai.md#experimental-teacher-throughput)
for results.
