# Offline learned models

## Shipped model and runtime

The Neural test table loads one supervised v1 model. Its 294 inputs come
from the acting seat's `PlayerView`. Two hidden layers of width 512 feed a
policy over 621 actions, a seat value and per-action score and win estimates.
The table uses the policy with illegal actions masked. Policy shares describe
action preferences. The win head estimates the outcome for each option.

The original v1 evaluation placed it above Easy and below Medium on the
tested deals and profiles. The [model card](../assets/models/card-play.md)
records training provenance, hash, intervals and limited Null/Hand/Ouvert
coverage. Its Kontra measurements use the original policy. Current neural
opponents use the Classic Kontra rule by default. See [AI levels](ai.md#levels).

`skat-app` looks for `card-play.gcnn` in `GRAND_COACH_MODELS`, then
`assets/models` beside the executable, then the workspace assets. It caches
the first load result. A readable manifest must contain a matching SHA-256.
If the manifest cannot be read, the loader uses the file's own hash.
Burn runs inference on CPU. A missing or invalid model, unsupported phase
or unusable output triggers the configured Classic fallback.

## Phase 2

As of 2026-11-11 the v2 programme is in PPO refinement. Supervised training
on the T-2 through T-8 datasets produced multiple network shapes. A role
tournament compared supervised candidates on shared deals. The local
`2026-11-10-rl-4x` campaign starts from the supervised 4×384, 4×512 and
4×896 models (hidden layers × width). It trains against Classic levels,
the frozen reference, recent snapshots and mixed tables. Intermediate
arenas screen snapshots within and across shapes against their starts
and reference opponents. Data, run logs and checkpoints live outside git.

Phase 2 ends after reinforcement learning and final selection. Measure
the shortlisted models against v1 and Classic on fresh validation deals,
rotate seats, report uncertainty and inspect declarer and defender results
as well as latency. Retain one or two selected models in the repository
with their manifests and model cards. The current runtime selects one
filename. Additional weights require an explicit packaging and selection
decision before they can be offered in the app. The 4×896 start file is
about 18 MB, above the current D-63 limit of 10 MB per file. Revisit
that constraint if selecting a candidate of this size.

## Training data

`generate` records permitted views and teacher actions with the real game
engine. Strong supplies the labels. The default table uses the teacher on
every seat. The `easy-mix`, `rookie-mix`, `medium-mix` and `level-mix`
tables broaden the card-play positions while recording what the acting seat
actually played. Final scores and hidden-card targets are training labels.
Deal seeds and hidden cards must stay out of model inputs.

Split by deal (80/10/10) so decisions from the same game stay together.
Shards are resumable and checksummed. Re-run the same generation command
after interruption, then verify the completed shards. Compare generators
using the [throughput procedure](ai.md#experimental-teacher-throughput).

```sh
cargo run --locked --release -p nn-train -- generate DATA --deals 100 --table level-mix
cargo run --locked --release -p nn-train -- verify DATA
```

## Supervised training and diagnostics

New supervised runs encode raw decisions with v2 (634 features). It extends
the v1 layout with trick order, public voids, remaining cards, Augen,
play context and pass bids. The model header determines the layout during
evaluation and PPO. To train v2, start supervised training in a fresh output
directory. Starting PPO from the bundled file keeps v1.

```sh
cargo run --locked --release -p nn-train -- train SL DATA --hidden 512 --layers 4
cargo run --locked --release -p nn-train -- evaluate SL/model.gcnn DATA
cargo run --locked --release -p nn-train -- probe SL/model.gcnn DATA
```

For multiple shapes use `train --queue 2x512,4x512 --workers cpu,cpu`.
A Vulkan build supports `--device vulkan:discrete`. `--cache DIR` reuses
prepared data and `--data-on-device GB` keeps batches on the GPU when they
fit. `--patience N` enables learning-rate decay and early stopping.
Interrupted runs resume from the output checkpoint with the same recipe.

`--belief-weight X` trains an auxiliary hidden-card head. Its weights are
saved separately for `probe --belief FILE`. An earlier auxiliary-loss trial
at weight 0.5 lost playing strength. Check belief calibration separately from
arena results. Many decisions share a final score, so per-row value errors
within a game are correlated. Use deals as the independent evaluation unit.

## Self-play refinement

`rl` starts from supervised weights and learns from the acting seat's final
Seeger-Fabian points using clipped PPO. The start model also serves as a
frozen reference. `--pool` weights opponent kinds, and `--mixed` adds pairs
of different kinds. Kontra follows the rules of thumb. Reinforcement learning
updates card play and Re. Training uses the latest iteration, while arenas
evaluate frozen snapshots.

```sh
cargo run --locked --release -p nn-train -- rl RL --start SL/model.gcnn --minutes 720
cargo run --locked --release -p skat-ai --example calibrate -- tuning nn:RL/model.gcnn nn:SL/model.gcnn 200 6 0
```

`RL/STOP` requests a stop after the running iteration. Restart with the same
recipe and start model to restore weights, optimiser and snapshot pool.
Select on tuning deals and reserve a fresh block for final validation.
Use the [role tournament](ai.md#role-tournament) for mixed defender pairs.
