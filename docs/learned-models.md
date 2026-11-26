# Offline learned models

## Shipped model and runtime

The optional Neural test table loads PPO v2 4×384 iteration 175 (634
permitted-view inputs, four hidden layers of width 384). Classic remains the
default. The policy masks illegal actions; shares are action preferences,
not win probabilities. The [model card](../assets/models/card-play.md) pins
recipe, provenance and limits. The [operational selection](neural-operational-selection.md)
records the revised narrow rule, all 18 comparisons and v1 rollback.

`skat-app` looks for `card-play.gcnn` in `GRAND_COACH_MODELS`, then
`assets/models` beside the executable, then the workspace assets only when
running from its `target` directory. For a custom Cargo target directory, set
`GRAND_COACH_MODELS` to the workspace's `assets/models`. It caches
the first load result. A readable manifest must contain a matching SHA-256.
If the manifest cannot be read, the loader uses the file's own hash.
Burn runs inference on CPU. A missing or invalid model, unsupported phase
or unusable output triggers the configured Classic fallback.

## Supported phases, levels and limits

| Aspect | Supported in the package | Not supported |
|--------|--------------------------|---------------|
| Phases | Card play in Spiel including Re | Bidding, Skat selection, game announcement and Kontra decisions stay Classic |
| Engines | Neural (Easy, Medium, Strong) and Hybrid, chosen per table | Rookie is Classic-only; Classic stays the default |
| Runtime | Offline CPU inference from the acting seat's `PlayerView` | GPU, network, first-run download, model server |
| Failure | Missing, corrupt, format/encoder-incompatible or disabled model: translated notice, Classic level of the seat plays on, user data unchanged | No partial or silent use of a rejected model |

Every learned level and engine keeps its experimental label. Removal needs
calibration that supports the level: adjacent-level ordering and Classic
equivalence are not established (see [ai.md](ai.md#neural-engine-selection)),
and Phase 2 ended as a no-go. Neural estimates are model preferences, not
exact values or win probabilities. Target-machine (six-core, integrated
graphics) latency, RSS and package size are unmeasured because the hardware
is unavailable to agents; i9-10900K figures are proxies only.

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

## History and void evidence (#93)

Encoder v2 keeps the 294 v1 inputs as a prefix and adds chronological play
order and public follow-class voids (`PlayerView::proven_voids()`; Buben
count as Trumpf in Farbspiel/Grand, as their suit in Null). Raw records are
re-encoded at load time, so existing shards are used unchanged. The runtime
accepts v1 weights only with their original v1 encoder and input shape; it
does not relabel them as v2.

Measured on 2026-11-18 (source `c0718db`), on `T-8-11k` (verified, 1,089
shards, 461,003 records, manifests untouched). Neither compared supervised
model saw this set (v1: T-2/T-3; v2: T-2..T-5). Teacher labels are Strong.

| Model (2×512, supervised) | Test agreement | Test policy loss | Test value RMSE |
|---|---|---|---|
| `v1-sl-2x512-01` (294 inputs) | 75.05 % | 0.6533 | 45.6 |
| `v2-sl-2x512-01` (634 inputs) | 79.65 % | 0.5381 | 40.5 |

Both are over 34,876 test rows; train/validation rows agree within 0.4
points. Teacher-option agreement is 60.2 % (v1) against 58.3 % (v2): v2
is better at the teacher's chosen card and slightly worse at its argmax
over option scores, so option quality is not uniformly improved. Gains
combine history features with more training data and the larger input;
this table does not isolate them.

Ablation (`nn-train probe`, v2, 30,029 validation rows of 959 deals): zeroing
the `play_order` block raises policy loss by +0.0247 and lowers agreement
by 1.5 points (+0.120 loss at eight completed tricks); zeroing `voids`
gives +0.0217 and −1.1 points (+0.050 at eight tricks). Both effects grow
with the number of completed tricks and are far below unseen-card or Trumpf
blocks. The model therefore uses the added evidence, modestly. Zeroing is a
distribution shift, not proof of correct strategy. Seeded arena strength
against v1 is in the [operational selection](neural-operational-selection.md).
Raw outputs: `grand-coach-data/studies/2026-11-18-history-93/`.

The original Strong-only `T-2-10k` is a representative sample of the
historical format-1 teacher data, not of all possible Skat play. On source
`29fea4e`, `nn-train verify` replayed all 1,000 shards (424,247 records,
none incomplete), checking views, deal splits, actions and final scores.
The manifest SHA-256 was `fcfe0ca63e8012252a0b24d305045ecfc3d0eeee410ac593224d6478408c2a14`;
the ordered shard-hash aggregate was
`1f5f82f231ce5efbf32adeaf3aac81dce9dc3c9661e1f08c1f99f1d4d91fb04d`,
matching the retained v2 training recipe. Evaluation with the v2 model
loaded the original files and encoded 248,098 train, 31,934 validation and
30,958 test card-play decisions. This was training data, not a held-out
quality measurement. The manifest and model hashes were unchanged afterwards.

A separate completed `calibrate` validation run on indices 9000000–9000199
compared these same supervised models over 200 deals and six seat allocations
per deal: v2 minus v1 was +3.45 Seeger-Fabian points per seat-game, 95% interval
[+0.49, +6.41]. Twelve of 1,200 games were thrown in; no Null game occurred.
The 200 distinct deal seeds did not collide with the declared seed ranges in
the v2 supervised recipe (which include v1's training sets); unrecorded data
and equal deals from different seeds were not excluded. This single comparison
does not isolate the history features or establish general strength. The
previous Phase 2 matrix and its no-go remain unchanged. Commands and outputs
are retained in HQ `var/jobs/2108/tmp/`.

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

## Higher-budget teacher (#71)

`generate --teacher-samples 96|384` labels with Strong's profile but 96 or
384 hypothetical deals per analysis instead of 24 (profile names
`strong-s96`, `strong-s384` in the manifest). It is a training-only option of
`nn-train`: `Level::Strong` and its golden decisions are unchanged and no
public level exists. The teacher draws its samples from the acting seat's
`PlayerView` like Strong; the ignored test
`high_budget_teacher_uses_only_permitted_information` substitutes hidden
cards and checks that records do not change. More samples reduce sampling
noise only, not the double-dummy defender bias, and do not improve pre-play
labels.

Generation cost, measured once with `generate --deals 6 --shard-deals 6
--seed 71 --cpus 4 --workers 4` (teacher table, same 6 deals, 269 records,
4 CPUs): Strong (24) 26 s, 96 samples 96 s (3.7x),
384 samples 454 s (17x), i.e. about 828, 226 and 48 games/hour. Only
one run per setting was timed; the record bytes are about equal.

A bounded follow-up on 2026-11-19 (source `d78626f011e5`) generated 100
teacher-only deals with 96 samples, seed 710000, four CPUs and a 90-minute
cap: 846 s, 4,209 records, 7,486,859 bytes, three incomplete teacher analyses.
All ten shards verified. One run under shared machine load is not a stable
throughput estimate. A CPU 2×512 model trained from scratch on these records
in 21 s (20 epochs, best epoch 8; 2,411 train, 240 validation and 363 test
card decisions). Its encoder is v2; the archived #73 model uses v1 and was
trained on 17,590 deals. The models and data hashes, recipe and raw logs are
in `grand-coach-data/studies/2026-11-19-teacher-71/`.

`calibrate validation` compared the new model (A, greedy policy at Easy) with
archived supervised #73 (B) on 200 validation deals, indices 9300000–9300199,
six seat allocations per deal and Kontra/Re on. A minus B was **−25.44
Seeger-Fabian points per seat-game**, nominal 95 % interval [−29.89, −20.98];
no games were thrown in and no Null contracts occurred. The 100-deal model is
weaker in this comparison. The unequal training volumes, encoder versions and
three incomplete analyses prevent attributing the difference to teacher
budget. It does not justify replacing the currently shipped optional PPO model;
Classic remains the default engine. No claim of a general teacher-quality
ordering follows from this bounded run.

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

`--preplay-weight W` (default 1, omitted from the recipe then) multiplies the
policy loss of all rows other than plain card play; `--rare-weight R`
additionally scales rows with a rare teacher choice (Hand, Null, Schneider/
Schwarz/Ouvert; [measured](ai.md#rare-pre-play-classes-and-the-learned-kontrare-head-74)).
`--discard-weight D` (default 1) additionally scales Drücken rows only, on top
of `--preplay-weight`; [measured](ai.md#drücken-specific-loss-weight-74).
`--hand-opportunity-weight` and `--announcement-opportunity-weight` (default 1)
scale all policy losses where Hand or an extended announcement is legal,
respectively, including teacher choices of other legal actions. They combine
with the selected-class factors; their effect on class performance is unmeasured.
`--hand-weight` and `--announcement-weight` independently multiply the loss
for teacher-selected Hand and Schneider/Schwarz/Ouvert announcements on top
of these weights. Both default to 1 and cannot create missing teacher labels.
`--hand-declaration-weight` (default 1) additionally weights all teacher
declarations after Hand, including plain games, independently of the Hand
pickup choice and the announced-stage factors. `--null-weight` (default 1)
independently weights teacher-selected Null declarations, including Null Ouvert,
on top of the applicable Hand and announcement factors. `--hand-null-weight`
(default 1) independently scales Null declarations after Hand, including
Null Ouvert, on top of both factors. `--kontra-weight` (default 1) scales
teacher-selected Kontra and Re calls; `--re-weight` (default 1) additionally
scales Re calls, which are rarer, on top of it. `--hand-kontra-weight`
(default 1) additionally scales either call in Hand games; `--null-kontra-weight` (default 1) does so in Null
games. None creates missing
labels.
`--hand-announcement-weight` (default 1) additionally scales those
announcements when made after Hand, on top of the announcement factor.
`--schneider-weight`, `--schwarz-weight` and `--ouvert-weight` add separate
factors (default 1) for the highest announced stage, respectively. They apply
on top of the announcement factor and do not create or validate labels.
Their effects on playing strength are unmeasured; training and arena holds
remain. `evaluate` reports teacher agreement per phase and exact-action
agreement within each teacher category. Results: [ai.md](ai.md#improved-imitation-recipe-74).

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
`nn-train validate-labels DIR` provides independent pre-play label validation
(rule recomputation from stored views, leak and pair checks). It also counts
Kontra opportunities, teacher and Classic rule choices and their overlap. It also
counts teacher choices and legal opportunities for each pre-play category,
both overall and separately for the deal-separated train, validation and test splits.
For Kontra and Re it flags missing positive or negative labels in each split and
counts incomplete analyses, fallbacks and empty option lists. These diagnostics
do not validate strategic quality or establish playing strength. The
[pre-play model card](pre-play-model-card.md) records its first small
result. A first bounded imitation run and behavioural calibration are recorded in
[ai.md](ai.md#first-all-phase-imitation-run-74): the model is weaker than
Classic and is not shipped. An improved recipe is measured in
[ai.md](ai.md#improved-imitation-recipe-74): still not shipped. The outcome-label pilot stays postponed until an
imitation model is competitive. Compute on this PC is admitted for bounded
shell jobs; the earlier Phase 2 compute hold is superseded.

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
strength or completion of #75. No training was started. Full fresh calibration, learning-curve/pool reporting
and the remaining issue acceptance evidence still require an admitted owner;
Issue #74 supplies an experimental opt-in pre-play engine, not a strength-qualified model.

### Training learning curve (#75)

`python3 scripts/phase2/learning_curve.py TRAIN_LOG [WINDOW]` summarises a
campaign `train.log` read-only (no compute). The 2026-11-18 run over the three
campaign logs (100-iteration windows) shows mean training reward per deal
18.8-19.0 (iterations 1-100) rising to 19.5-19.7 (201-300) for 4×384, 4×512
and 4×896; KL to the reference grows from about 0.10 to 0.21-0.23. Against the
mixed pool the mean result versus Strong improves from about -3.0/-3.1/-3.7
to -1.7/-1.3/-1.8 (201-300); versus snapshots it stays near -2.5. These are
on-policy training statistics against a moving pool with high per-iteration
variance, not held-out strength.

### Fresh calibration and checkpoint selection (#75)

Measured 2026-11-18 with the unchanged `calibrate` example (`nn:…@easy`,
Kontra/Re on, six seat allocations per deal, paired A − B SF points per
seat-game, nominal 95 % intervals). The v1 file is the supervised #73 model.
Seeds: validation suite indices 9000000–9000299 (selection) and 9100000–9100599
(final); both lie above every index in the Phase 2 seed audit
(prior arena max 720399, held-out 8000000–8001999). Raw outputs stay in
`grand-coach-data/studies/2026-11-18-selfplay-75/`.

| Block | Comparison | Deals | A − B | 95 % interval |
|---|---|---:|---:|---|
| selection | 4×384 it175 − 4×512 it224 | 300 | -0.65 | [-2.67, +1.37] |
| selection | 4×384 − v1 | 300 | +12.83 | [+9.89, +15.76] |
| selection | 4×512 − v1 | 300 | +13.56 | [+10.31, +16.82] |
| final | 4×384 − Easy | 600 | +18.21 | [+15.74, +20.69] |
| final | 4×384 − Medium | 600 | +5.41 | [+3.64, +7.18] |
| final | 4×384 − Strong | 600 | -0.83 | [-2.53, +0.87] |
| final | 4×384 − v1 | 600 | +13.27 | [+11.19, +15.35] |

Selection rule, fixed before the final block: keep the smaller shipped 4×384
iteration 175 unless 4×512 beats it with a lower bound above 0 on the selection
block. It did not (inconclusive), and 4×896 stays evaluation-only (D-63), so
the shipped checkpoint is unchanged. It is clearly stronger than the supervised
v1 model and Easy/Medium, and statistically indistinguishable from Strong; no
superiority over Strong is claimed. Deal counts of that block are below the predeclared
2000 (see the 2,000-deal matrix below), the tuning-selected checkpoint was not re-searched, and roles, contracts
and PPO causality are not analysed here. The [model card](../assets/models/card-play.md)
links this record.

### 30-minute pilot (#75)

Measured 2026-11-18 on a shared, loaded workstation (load average 20-30 on 20
cores; the job was limited to `--cpus 4`, 4 parallel games × 1 analysis thread)
with the campaign recipe (`--games 1500 --batch 512 --passes 2 --lr 1e-4
--clip 0.2 --reference-weight 0.01 --snapshot-every 10 --pool-snapshots 8
--seed 0`, the campaign `--pool` and `--mixed` weights), start model supervised
4×384 and `--minutes 30`. The wall cap ended the run after four iterations
(about 22,400 decisions each); play took 683/505/313/545 s and the update
32/13/4/4 s, i.e. 5-12 min per iteration, strongly load dependent. Mean
training reward per deal +19.0/+17.7/+20.1/+18.5, KL to the reference
0.0042/0.0097/0.0150/0.0182, clipped fraction 4.7/1.6/1.6/1.2 %.

Iterations 1-3 reproduce the stopped campaign's first three log lines exactly
(same rewards, pool results and KL), so generation is reproducible from the seed
independent of CPU count and machine load. A four-iteration pilot says nothing
about strength; no checkpoint of it was evaluated. Resume after interruption
was not exercised here (covered by the existing tests). Raw logs and the
checkpoint stay in `grand-coach-data/studies/2026-11-18-selfplay-75-pilot/`.

### 2,000-deal matrix (#75)

Measured 2026-11-18 with the same `calibrate` binary and settings on a fresh
block, validation indices 9200000-9201999 (above every earlier index), 2000
deals × 6 seat allocations, A = shipped 4×384 iteration 175
(sha256 `6eeb3359…def75`):

| Comparison | Deals | A − B | 95 % interval |
|---|---:|---:|---|
| A − Strong | 2000 | +1.00 | [+0.17, +1.84] |
| A − v1 (supervised #73) | 2000 | +13.61 | [+12.47, +14.74] |

The clear gain over v1 is confirmed at the full size. Against Strong the
interval now excludes zero narrowly, whereas the earlier 600-deal block gave
-0.83 [-2.53, +0.87]; the two blocks disagree within noise, so only a small
advantage over Strong at best is supported, not a stable ranking. Neither
Medium/Easy nor the 4×512 and 4×896 shapes were re-run at 2000 deals, roles,
contracts and PPO causality are not analysed, and the checkpoint remains the
one selected on the earlier selection block. The model stays the shipped
default learned model; v1 remains the fallback comparison. Raw outputs:
`grand-coach-data/studies/2026-11-18-selfplay-75-matrix/`.

Tiny corruption and missing-file fixtures run with the existing Phase 2
unittest discovery command in [AGENTS.md](../AGENTS.md).
