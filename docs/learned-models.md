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

## All-phase imitation preparation (#74)

The opt-in `train --all-phases` mode uses the existing shared v2 encoder
and 621-action policy head for Reizen, Hand/pickup, Drücken, Ansage and
card play including Kontra/Re. Drücken scores each unordered pair jointly:
496 global pair indices, masked to the 66 pairs in the declarer's twelve
cards. No new encoder, model format or runtime default is introduced.

This mode retains the recorded teacher action and legal list, including
analysed Kontra/Re when present in the source, instead of applying the
card-play trainer's #108 Kontra relabelling. Source manifests and independent
pre-play label validation still determine whether a dataset is suitable;
selecting the mode does not certify those labels. Policy/value/option target
settings otherwise follow the existing recipe; future scores and teacher
estimates remain targets only. This is imitation preparation, not an
outcome-label experiment or evidence that double-dummy bias is resolved.

The recipe records the opt-in and all five phase bits; cache keys separate
both modes, resumed runs reject a changed recipe, and exported models carry
the selected phase coverage. `evaluate` selects all-phase records when the
model header covers all phases. Its aggregate imitation metrics are not
bidding/pass/Einpassen rates, contract mix or role-separated calibration.
The existing [calibration runner](ai.md#calibration-protocol) now reports
these behavioural counts and their denominators alongside role scores when
playing complete games; reporting support does not supply production results.
The default card-play recipe and deployed weights remain unchanged. The
shipped Neural opponent still applies its configured #108 Kontra rule;
experimental inference of recorded Kontra actions does not enable them in
that opponent by default.

Tiny checked-in fixtures exercise training, model reload and legal inference
across the recorded phases. Scripted schema tests additionally exercise Hand,
pickup, discard, immediate Re and Ouvert reveal; they are not strategic
labels. No production all-phase model or strength measurement is delivered.
A production imitation run, independent label validation, model card and
behavioural calibration remain open in #74. The outcome-label pilot is
postponed because those prerequisites are absent and the Phase 2 no-go
allows no further training or arenas. Long compute requires separate
admission as a shell job; this tooling change does not grant it.

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

### Read-only campaign revalidation (#75)

Before considering any new self-play work, run
`python3 scripts/phase2/revalidate.py EVALUATION` against the frozen evaluation
folder containing `manifest.json`. It prints JSON and writes no archive files.
It verifies frozen model hashes and sizes, the campaign STOP marker, latest
checkpoint weights and optimiser hashes, and retained snapshot hashes for all
three shapes. Mutable original model paths are reported separately; their
mismatch does not invalidate the frozen copy (the original v1 deployment path
now contains the operational replacement).

The 2026-11-18 revalidation found all seven frozen models intact and verified
checkpoints 315/306/358 with 31/30/35 snapshots for 4×384/4×512/4×896.
This is artifact integrity evidence only, not deserialisation, resumability,
strength or completion of #75. No training was started. The Phase 2 compute
hold remains binding. Full fresh calibration, learning-curve/pool reporting
and the remaining issue acceptance evidence still require an admitted owner;
Issue #74 currently supplies preparation only, not a production pre-play model.

### Training learning curve (#75)

`python3 scripts/phase2/learning_curve.py TRAIN_LOG [WINDOW]` summarises a
campaign `train.log` read-only (no compute). The 2026-11-18 run over the three
campaign logs (100-iteration windows) shows mean training reward per deal
18.8-19.0 (iterations 1-100) rising to 19.5-19.7 (201-300) for 4×384, 4×512
and 4×896; KL to the reference grows from about 0.10 to 0.21-0.23. Against the
mixed pool the mean result versus Strong improves from about -3.0/-3.1/-3.7
to -1.7/-1.3/-1.8 (201-300); versus snapshots it stays near -2.5. These are
on-policy training statistics against a moving pool with high per-iteration
variance, not held-out strength. Selection on validation deals and a fresh
`calibrate` run remain under the compute hold.

Tiny corruption and missing-file fixtures run with the existing Phase 2
unittest discovery command in [AGENTS.md](../AGENTS.md).
