# Offline learned models

## Shipped model and runtime

The optional Neural test table loads PPO v2 4×384 iteration 175 (634
permitted-view inputs, four hidden layers of width 384). Classic remains the
default. The policy masks illegal actions; shares are action preferences,
not win probabilities. The [model card](../assets/models/card-play.md) pins
recipe, provenance and limits. The [operational selection](neural-operational-selection.md)
records the revised narrow rule, all 18 comparisons and v1 rollback.

`skat-app` looks for `card-play.gcnn` in `GRAND_COACH_MODELS`, then
`assets/models` beside the executable, then the workspace assets. It caches
the first load result. A readable manifest must contain a matching SHA-256.
If the manifest cannot be read, the loader uses the file's own hash.
Burn runs inference on CPU. A missing or invalid model, unsupported phase
or unusable output triggers the configured Classic fallback.

## Phase 2

Phase 2 concluded on 2026-11-12 with a **no-go due to incomplete evidence**.
The historical retain-v1 outcome is superseded for optional deployment only
by the [separate operational selection](neural-operational-selection.md);
Classic stays the default. Of 18 predeclared held-out
comparisons, 17 timed out and only 4×896 versus Strong (HQ #401) completed.
The [evaluation card](phase2-outcome.md) records 232 verified chunks,
23,200/36,000 pair-deals and 12,800 missing pair-deals. These comparisons
reuse common underlying deals, so the totals are not independent samples.

The stopped `2026-11-10-rl-4x` PPO campaign refined supervised 4×384,
4×512 and 4×896 models against Classic levels, the frozen reference,
snapshots and mixed tables. Its frozen tuning shortlist and
[predeclared evaluation rules](phase2-evaluation.md) remain unchanged.
No candidate passed the historical shipping acceptance; 4×896 is additionally evaluation-only
under D-63's per-file size limit. The current runtime selects one filename;
additional weights need an explicit packaging and selection decision.

No further training or arenas are authorised by this outcome. Original data,
run logs and checkpoints remain outside git. Unresolved history, option-quality
and learning-efficacy evidence remains open in #75/#93 and the neural epic.

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
directory. Starting PPO from the bundled file now keeps v2.

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
