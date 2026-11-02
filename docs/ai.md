# Opponent AI: `skat-ai`

Strength levels of M4 (D-07, D-40) and the additional opponents Rookie,
Goldfish and Mephisto (D-41): profiles, information boundary, the
calibration protocol and its results. Details are in the code
(`crates/skat-ai`).

## Calibration protocol

Written down before the first validation run. The tuning suite had
already been used for pilot runs at that point; the validation suite had
not.

- **Runner**: `cargo run --locked --release -p skat-ai --example
  calibrate -- <suite> <A> <B> <deals> <parallel> <first>`, headless,
  with the real engine, analysis and opponents (`skat_ai::arena`).
- **Rules**: ISkO with Kontra/Re on (D-03, D-26).
- **Suites**: deal seeds come from `ChaCha8Rng` streams with separate
  keys for tuning and validation, so the two never share deals. Tuning
  uses only the tuning suite. Each validation attempt uses a fresh block
  of validation deals (attempt 1: deals 0–199, attempt 2: 200–399, …);
  every attempt is reported.
- **Opponent seeds**: from a third key mixed with suite, deal index, game
  and seat; never derived from a deal seed.
- **Schedule**: each deal is played six times (`arena::SEATINGS`): A
  alone on Vorhand, Mittelhand and Hinterhand against two copies of B,
  then B alone on each seat. Each profile holds each seat, and so each
  dealer position, equally often, both as the lone player and as a pair
  of defenders.
- **Metric**: Seeger-Fabian points per seat-game, Einpassen included
  with 0 points for every seat. Per deal, D = mean of A's 9 seat-games −
  mean of B's 9 seat-games. Declarer and defender games, win rates and
  points per role are reported as supporting information.
- **Sample size**: 200 deals (1,200 games) for each adjacent pair (Easy
  vs. Medium, Medium vs. Strong) on the validation suite.
- **Uncertainty**: deals are independent; standard error of the mean of D
  over deals and a 95 % interval (normal approximation, mean ± 1.96
  standard errors).
- **Pass criterion**: for each adjacent pair, the 95 % interval of
  stronger − weaker lies entirely above 0. Otherwise the result is
  reported as inconclusive (or reversed) and calibration continues on the
  tuning suite before a new validation attempt.

## Levels

| Level | Before card play | Card play, Kontra/Re | Budget |
|-------|------------------|----------------------|--------|
| Rookie | rules of thumb with mistakes | rules of thumb with mistakes | no search |
| Easy | rules of thumb | rules of thumb | no search |
| Medium | rules of thumb | `analyze` | 6 samples, 10⁸ nodes per task |
| Strong | rules of thumb | `analyze` | `Budget::INTERACTIVE` (24 samples) |

- The rules of thumb are `BaselineOpponent`: the shared continuation
  policy of `skat-analysis` (`policy`) plus simple card rules.
- An analysed decision picks the option with the highest utility: the
  seat's expected Seeger-Fabian points; a defender also counts half of the
  declarer's expected game points (so Kontra and denying Schneider count).
  Only options evaluated on the most distributions compete. Exact ties go
  to the rules-of-thumb action (a 0.5-point bonus), since double-dummy
  values only resolve scoring classes.
- `Profile` also supports selection noise (`temperature`) and analysed
  bidding and declaring. The shipped levels use neither: on the tuning
  suite, analysed bidding passed too often (the double-dummy defenders are
  perfect, so the analysis is pessimistic for the declarer; M3 itself
  rates pre-play estimates as coarse, D-39), and noise only weakened play.
- No level forgets observed history: every analysed decision uses the full
  view, as `skat-analysis` does.

## Rookie

`Level::Rookie` is Easy plus seeded mistakes (`Mistakes`,
`Profile::ROOKIE_MISTAKES`). Each kind of decision deviates from the rules
of thumb with its own probability:

| Decision | Probability | Deviation |
|----------|-------------|-----------|
| Bidding | 0.35 per game | bid limit moved 1–3 bidding values up or down (a passing hand may bid 18–22) |
| Announcement | 0.4 | second-best trump game by the rules' strength measure that still covers the bid |
| Drücken | 0.2 | two random cards other than Jacks |
| Kontra/Re | 0.15 | the rules' choice reversed |
| Card | 0.12 | a random legal card |

The bidding draw uses a seed fixed for the seat's whole bidding (opponent
seed and hand), so a misjudged limit stays misjudged; the other draws use
the decision seed. Every deviation is a legal action.

## Goldfish

Table mode `Opponents::Goldfish` in `skat-app`: one opponent, drawn per
series from the opponents' seed stream, plays `Profile::GOLDFISH`; the
other plays Medium. `Profile::GOLDFISH` is Strong with `forget = 0.5`:

- In every card-play decision each card of a completed trick before the
  last one is forgotten with probability 0.5 (`Opponent::forgotten`,
  seeded per decision).
- `skat_analysis::analyze_forgetful` samples the distributions as usual
  and then puts every forgotten card in the place of a random hidden card
  (another hand or the Skat, never a hand proven void in its suit); that
  card counts as played instead. The Goldfish thus believes some played
  cards, trumps included, are still out, and plays double-dummy in that
  belief. The rest of the view (own hand, last trick, trick in progress,
  counts) stays exact.
- Bidding and declaring follow the rules of thumb, as for Strong.

## Mephisto

Table mode `Opponents::Mephisto` in `skat-app`, decided by
`skat_ai::Mephisto`, which receives the whole `Game`:

| Phase | Computer declarer | Its computer partner | Against the human declarer |
|-------|-------------------|----------------------|----------------------------|
| Bidding, Hand, Drücken, announcement | Mephisto | – | – (both bid as Mephisto) |
| Card play, Kontra/Re | Mephisto | Strong | both defenders Mephisto |

- Every decision is `skat_analysis::analyze_known` on the real
  distribution (`World::from_omniscient`): one exact distribution, each
  option played out, card play double-dummy; the best option by utility
  is chosen. Before card play every option is its own task (parallel).
- Two Mephisto defenders solve the same double-dummy game, in which the
  defenders minimise together, so they play as one side.
- Bidding: a bid or hold is allowed only if Mephisto also wins when every
  other seat passes at once (checked exactly with `analyze_known` at the
  following Hand-or-pickup decision; binary search over the bidding
  values). As declarer Mephisto therefore only plays games it wins
  double-dummy, against any defence. Among the allowed options the
  analysis decides, with the other seats modelled by the rules of thumb.
- Node budget `Mephisto::BUDGET`: 10⁸ nodes per task, as
  `Budget::INTERACTIVE`. Without an estimate the rules of thumb decide.
- The GUI shows the Mephisto variant of the chosen card faces, the
  Mephisto back (flames in the gate to hell) and a red instead of the
  green table, only in this mode (D-41).
- God Mode (`Opponents::GodMode`, D-62) has the same opponents; there the
  human and the coach see every card, and the GUI shows the God Mode
  faces, back and a blue table.

## Neural table mode

Table mode `Opponents::Neural` in `skat-app` (preview of #73): both
opponents are `NeuralOpponent`s with the card-play model and Classic
Strong as fallback, each deciding from its own `PlayerView`. Pre-play and
every model fallback are Strong. `skat_app::neural_model` loads
`card-play.gcnn` once per process from `GRAND_COACH_MODELS`,
`assets/models` next to the executable or the workspace's
`assets/models`, checked against `model_sha256` of `card-play.json`;
without a model the mode plays Strong and the GUI says so.

## Information boundary and determinism

- `Opponent::decide(view, cancel)` takes one seat's `PlayerView` (D-23);
  this covers every level and the Goldfish. Only `Mephisto::decide` takes
  the whole `Game`.
  Its analysis input (`Opponent::analysis_input`) is that view, the
  profile's budget and a seed mixed from the opponent's seed and public
  facts of the view.
- `skat-app` draws opponent seeds from stream 1 of the series seed, the
  deals from stream 0; the arena uses separate keys. No opponent gets a
  deal seed.
- Same view, opponent seed and profile give the same decision, unless a
  `Cancel` stops the analysis. Fallback: with partial results the
  finished distributions decide; without any estimate (cancelled before
  the first distribution, exhausted node budget, inconsistent view) the
  rules of thumb decide. Every choice is a legal action of the view.
- The frontend computes opponent moves on a background thread
  (`Session::opponent_job`, `OpponentJob::run`, `Session::finish_opponent`);
  a move for an outdated position is rejected. Starting a new series
  cancels the computation. There is no deadline, so series stay
  reproducible.

Tests: `crates/skat-ai/tests/fairness.rs` swaps concealed cards (hands,
Skat, also in Hand and Ouvert games) while the deciding seat's view stays
identical and checks equal analysis inputs, forgotten cards and decisions
for every level and the Goldfish, including a known own Skat and revealed
Ouvert cards; `tests/levels.rs` covers profiles, reproducibility,
complete games and the fallbacks; `tests/variants.rs` the Rookie's
deviation rates, what the Goldfish forgets and that Mephisto never loses
a game it declares (3 games in `test`, 100 in `deep`). With every card
known and a node budget instead of a time limit, Mephisto's choices are
exact and reproducible, so the test tolerates no loss: a lost game is a
bug. Only a game in which the rules of thumb decided (exhausted budget)
may be lost; at most 1 in 20 games may contain such a fallback.
`skat-app/tests/series.rs` checks which decisions see every card in the
Mephisto mode (fixed seeds) and the Goldfish draw.

## Calibration results

Validation attempt 1 on 2026-10-29, deals 0–199 of the validation suite
(Easy vs. Strong: 0–99), reference hardware of
[analysis.md](analysis.md#latency-target-and-benchmark), 10 deals in
parallel. Differences in Seeger-Fabian points per seat-game, stronger
minus weaker:

| Pair | Deals | Difference | Std. error | 95 % interval | Verdict |
|------|-------|------------|------------|---------------|---------|
| Medium − Easy | 200 | +16.18 | 2.26 | [+11.74, +20.62] | pass |
| Strong − Medium | 200 | +6.76 | 1.75 | [+3.33, +10.19] | pass |
| Strong − Easy | 100 | +22.65 | 3.28 | [+16.23, +29.08] | (supporting) |

Supporting results per profile (each pair: 588 declarer and 1,176
defender seat-games per profile; 24 of 1,200 games thrown in):

| Pair: profile | Points/seat-game | Declarer won | Points/declarer game | Defender won | Points/defender game |
|---------------|------------------|--------------|----------------------|--------------|----------------------|
| Easy (vs. Medium) | 8.83 | 62.2 % | 4.0 | 28.7 % | 11.5 |
| Medium (vs. Easy) | 25.01 | 75.5 % | 49.8 | 33.5 % | 13.4 |
| Medium (vs. Strong) | 22.84 | 71.9 % | 50.1 | 24.7 % | 9.9 |
| Strong (vs. Medium) | 29.61 | 77.0 % | 69.6 | 26.3 % | 10.5 |

Both levels bid by the same rules, so seatings with equal bidding produce
equal declarers; the steps come from card play and Kontra/Re. Wall time:
4 min (Easy–Medium), 23 min (Medium–Strong), 9 min (Easy–Strong).

Rookie and Goldfish, same protocol and suite (deals 0–199, Goldfish
0–99), run after D-41:

| Pair | Deals | Difference | Std. error | 95 % interval |
|------|-------|------------|------------|---------------|
| Easy − Rookie | 200 | +19.70 | 2.16 | [+15.46, +23.94] |
| Medium − Goldfish profile | 100 | +14.00 | 3.95 | [+6.26, +21.73] |
| Strong − Goldfish profile | 100 | +17.09 | 3.62 | [+10.00, +24.18] |

The Goldfish profile alone is weaker than Medium; in the Goldfish mode it
sits next to a Medium opponent. Mephisto is not part of these pairings:
it is a table mode with full knowledge, and as declarer it never loses
(`tests/variants.rs`).

Tuning runs before (tuning suite, 30–60 deals each) compared analysed
bidding, analysed declaring, 4/6/24 samples and temperature 4; they are
not part of the evidence above.

Reproduce:

```sh
cargo run --locked --release -p skat-ai --example calibrate -- validation easy medium 200 10 0
cargo run --locked --release -p skat-ai --example calibrate -- validation medium strong 200 10 0
cargo run --locked --release -p skat-ai --example calibrate -- validation easy strong 100 10 0
```

## Latency

`cargo run --locked --release -p skat-ai --example latency [positions]`
times `Opponent::decide` on the benchmark positions of
[analysis.md](analysis.md#latency-target-and-benchmark) (reference
hardware, quiet machine). Decisions by rules of thumb take well under
1 ms. Analysed decisions, 30 positions per kind:

| Level | Decision | Median | p90 | Max |
|-------|----------|--------|-----|-----|
| Medium | Kontra | 0.13 s | 0.51 s | 1.5 s |
| Medium | Card play, trick 1 | 0.25 s | 1.1 s | 4.0 s |
| Medium | Card play, trick 4 / 8 | 4 ms / 1 ms | 10 ms / 1 ms | 19 ms / 2 ms |
| Strong | Kontra | 0.33 s | 2.5 s | 3.5 s |
| Strong | Card play, trick 1 | 0.74 s | 2.2 s | 5.4 s |
| Strong | Card play, trick 4 / 8 | 6 ms / 1 ms | 21 ms / 1 ms | 47 ms / 2 ms |

Rookie decides in well under 1 ms. Goldfish and Mephisto, 10 positions
per kind (same command):

| Opponent | Decision | Median | p90 | Max |
|----------|----------|--------|-----|-----|
| Goldfish | Kontra | 0.35 s | 2.5 s | 3.3 s |
| Goldfish | Card play, trick 1 / 4 / 8 | 1.1 s / 5 ms / 1 ms | 2.0 s / 15 ms / 2 ms | 5.4 s / 44 ms / 2 ms |
| Mephisto | Reizen | 0.89 s | 3.4 s | 3.6 s |
| Mephisto | Hand/pickup | 0.11 s | 0.22 s | 0.60 s |
| Mephisto | Drücken | 0.52 s | 1.4 s | 1.9 s |
| Mephisto | Announcement | 0.23 s | 0.31 s | 0.59 s |
| Mephisto | Kontra | 0.06 s | 0.34 s | 1.1 s |
| Mephisto | Card play, trick 1 / 4 / 8 | 62 ms / 3 ms / 1 ms | 0.18 s / 4 ms / 1 ms | 0.31 s / 7 ms / 1 ms |

The Goldfish costs what Strong costs (forgetting starts at trick 3).

All levels meet the M3 target (card play and Kontra/Re: median ≤ 1 s,
p90 ≤ 3 s). With only the first 10 positions per kind, Strong's first
trick had a median of 1.08 s (p90 2.1 s), slightly above the target; the
larger sample is reported as the measurement. Every decision completed.

## Learned engine runtime

Foundation of the learned engines of epic #67 (D-07, D-11), delivered
in issue #69. The first trained model, for card play, ships with #73
([Card-play network](#card-play-network)); Classic stays the default.

- **Crates**: `skat-nn` (between `skat-core` and `skat-analysis`) holds
  the encoder, the action space, the network and CPU inference; AI and
  coach share it. `tools/nn-train` trains outside the app.
- **Encoder** (`skat_nn::encode`, version `ENCODER_VERSION` = 1): 294
  numbers from one `PlayerView` only: own hand, own discarded Skat
  (declarer after Drücken), revealed Ouvert cards, cards of completed
  tricks by relative seat, the trick in progress, own position, phase,
  declarer, bidding per seat, contract, Kontra/Re and Augen per party.
  Seats are relative to the acting seat. Layout in `encoder.rs`.
- **Action space** (`skat_nn::actions`): 621 indices: 63 bid values,
  Hold, Pass, Hand, TakeSkat, 496 discard pairs, 22 announcements,
  32 cards, Kontra, KeinKontra, Re, KeinRe. Part of the encoder version.
- **Network** (`PolicyValueNet`): input, two hidden ReLU layers of equal
  width, a policy head (one logit per action), a value head (expected
  Seeger-Fabian points of the acting seat) and two option heads (#73):
  per action the expected Seeger-Fabian points after it and the logit of
  the probability that the seat's party wins after it. The same
  definition trains and infers. `NeuralModel::evaluate_with` picks the
  action by the policy (`Choice::Policy`, default) or by the highest
  expected points (`Choice::Score`); every evaluation carries the option
  estimates.
- **Model file** (format version 2; version 1 had no option heads): a
  24-byte header (magic `GCNN`, format and encoder version, layer sizes,
  covered phases) and the weights in Burn's burnpack format. The caller passes the bytes and their SHA-256 hash;
  `skat-nn` reads no files.
- **Safety**: illegal actions are masked out before the softmax, and the
  chosen action is checked against the view again before acting. A
  missing model, a wrong hash, an unknown format or encoder version, a
  shape that does not fit, corrupt weights, an uncovered phase or a
  non-finite output give a `Fallback` reason; `NeuralOpponent` then lets
  its Classic `Opponent` decide (deterministic, same seed) and reports
  the reason (`NeuralDecision::fallback`).
- **Estimates**: policy probabilities, the value and the option
  estimates are model estimates.
  They are not `Estimate`s of the analysis and never claim `exact`,
  Monte-Carlo samples or a reliable comparison.
- **Arena**: `Player` is implemented by `Opponent` and `NeuralOpponent`;
  `arena::play_with` and `arena::compare_with` seat any players, and
  `calibrate` accepts the profile `nn:<file>[@<level>]` (`nnq:` picks
  by the option scores). Tests use a
  seeded random-weight fixture model (`skat_nn::fixture_model`, feature
  `fixture`), which runs in CI on Linux and Windows.
- **Builds**: the app builds `skat-nn` with Burn's Flex backend only.
  `tools/nn-train` adds autodiff and optimisers, and Vulkan with its
  feature `vulkan` (off in the tests, [testing.md](testing.md)); in a
  workspace build Cargo unifies these features, so release binaries are
  built with `-p gui-egui`.

Smoke test of #69 on 2026-11-01 (reference hardware of
[analysis.md](analysis.md#latency-target-and-benchmark) plus an AMD
Radeon RX 6900 XT, Fedora 44 Distrobox, Rust 1.99.1, release builds from
an empty target directory):

| Item | Result |
|------|--------|
| Version | Burn 0.22.1 (pinned), declared MSRV 1.95 = workspace MSRV; every compiled dependency declares at most 1.95 (CI job `msrv` builds with 1.95) |
| CPU round trip | `nn-train smoke cpu 200 300`: 8,220 examples from 200 Easy games, loss 3.13 → 1.15 in 300 Adam steps, model file (254,748 bytes) and optimizer checkpoint round trips pass, 35 s |
| GPU round trip | `nn-train smoke vulkan 200 300` (feature `vulkan`) on "AMD Radeon RX 6900 XT (RADV NAVI21)": loss 3.13 → 1.19, round trips pass, 10 s |
| Arena | the GPU model against Easy (`calibrate -- tuning nn:<file> easy 20 4`): 120 games without an illegal action, −29.5 ± 11.0 points per seat-game; the CPU model −44.4 ± 10.7 (untrained-quality smoke models) |
| Build time | `grand-coach` 61 s → 97 s; `skat-nn` alone 55 s; `nn-train` 219 s |
| Binary size | `grand-coach` 19,841,720 → 19,838,696 bytes (it does not call inference yet); a minimal program that loads a model and plays a game with it: 4,635,872 bytes against 500,392 bytes for the same game loop on `skat-core` alone, so linked CPU inference adds about 4.1 MB |

## Training data

Teacher self-play for the learned engines (#70), generated by
`tools/nn-train` (`selfplay.rs`, `shards.rs`) with the real engine and
opponents; no second simulator.

```sh
cargo run --locked --release -p nn-train -- generate OUT_DIR --deals 100 --minutes 30 --max-mb 1024
cargo run --locked --release -p nn-train -- verify OUT_DIR
```

- **Games**: three Strong opponents per deal, ISkO with Kontra/Re.
  Every decision of every seat is recorded, so Reizen, Hand/pickup,
  Drücken, announcement, Kontra/Re and card play are all covered. Strong
  bids and declares by the rules of thumb, so those records carry no
  option values.
- **Record** (format version 1, one JSON line each): deal index and split,
  the acting seat's `PlayerView` (with phase and legal actions), the
  chosen action, Strong's option values (`OptionValue` per legal action)
  with the analysis status and fallback flag, and the seat's final
  Seeger-Fabian points (0 if thrown in). Records are raw; the encoder
  runs at training time. The view is the only model input: deal index,
  seeds and the final points are bookkeeping or targets.
- **Seeds**: deal and teacher seeds come from separate keys mixed with
  the run seed and deal index; teacher seeds never derive from the deal
  seed. The manifest records dataset and record format, encoder version,
  engine commit (`git describe`), profile, seed scheme, rules, deal count
  and shard size; a restart must match it.
- **Split**: by deal seed, 80 % train, 10 % validation, 10 % test, so
  every seat and variant of a deal lands in the same split.
- **Shards**: one per range of deals (`--shard-deals`, default 10):
  data written to a temporary file and renamed, then a `.done` file with
  size, record count and SHA-256. A shard without a matching `.done` is
  never read as valid; a restart skips complete shards and redoes the
  rest. Results do not depend on the worker count.
- **Caps**: every run has a wall-time cap (`--minutes`, default 30) and
  a disk cap (`--max-mb`, default 1024); no deal starts past either.
  The CPU budget (`--cpus`, default all) is split into `--workers` game
  workers (default 4) times `cpus / workers` analysis threads
  (`skat_analysis::with_max_threads`), so the analysis, which
  parallelises each decision, does not oversubscribe the CPU.
- **Fixture**: `tools/nn-train/fixtures/selfplay` holds two Strong deals
  (seed 0, one shard each, 101 records, 184 KB); the tests verify it and
  regenerate it identically. No larger dataset is committed.

Pilot on 2026-11-01 (reference hardware of
[analysis.md](analysis.md#latency-target-and-benchmark), 20 CPUs,
release build, seed 0, 100 deals in 10 shards; Rust 1.99.1):

| Item | 4 workers × 5 threads | 10 workers × 2 threads |
|------|-----------------------|------------------------|
| Time for 100 games | 210 s | 183 s |
| Games per hour | 1,718 | 1,963 |
| Records | 4,152 | 4,152 (byte-identical shards) |
| Bytes per game | 72,413 | 72,413 |
| Projection for 10,000 games | 5.8 h, 724 MB | 5.1 h, 724 MB |

Decisions per game: Reizen 7.34, Hand/pickup 1.00, Drücken 0.98,
announcement 1.00, Kontra/Re 1.20, card play 30.00. No deal was thrown
in; all 3,120 analysed decisions completed (1,032 decided by the rules of
thumb). Strong said Kontra in all 60 decisions that offered it and Re in
20 of 60: Kontra does not change a defender's Seeger-Fabian points, so
its option values tie and the teacher's Kontra choices carry little
signal for training.

## Card-play network

The first trained model (#73): `assets/models/card-play.gcnn` with its
training manifest `card-play.json` and model card `card-play.md`
(storage: D-63). It covers card play including Kontra/Re
(`Phases::SPIEL`); pre-play stays Classic.

- **Training** (`nn-train train`, `tools/nn-train/src/train.rs`): the
  card-play decisions of one or more self-play runs, split by deal as
  recorded. Targets: Strong's card for the policy (optionally soft
  targets from complete option values, off in the shipped recipe), the
  mean of the final Seeger-Fabian points and the chosen option's expected
  points for the value, and Strong's expected points and win chance per
  legal card for the option heads (when the analysis was complete).
  Mini-batch Adam on the CPU or with `--device vulkan`.
- **Reproducibility**: initial weights and example order come from the
  recipe's seed (SplitMix64), never from a device's random state. After
  every epoch a checkpoint (weights, Adam state, best weights, each with
  its SHA-256) is written and `state.json` points at it last; a restart
  with the same recipe resumes bit-identically, a different recipe or a
  corrupt checkpoint is refused. No epoch starts after the wall-time cap
  (`--minutes`). The output is the epoch with the lowest validation loss
  and `train.json`: recipe (data hashes, encoder version,
  hyperparameters, seed), trainer commit, coverage per split and kind,
  epoch logs, metrics per split, model size and hash.
- **Evaluation**: `nn-train evaluate MODEL DATA_DIR...` reports a model's
  metrics on every split, so models trained on different data compare on
  the same examples; strength comes from `calibrate` and timing from
  `latency` (`nn:<file>`).

Runs on 2026-11-01/02 (reference hardware of
[analysis.md](analysis.md#latency-target-and-benchmark), release
builds). Data: Strong self-play, engine commit `99c91300dee7`, 16 CPUs
(8 workers × 2 analysis threads), 1,986–2,032 games per hour; run seed
73 with 10,000 games, run seed 74 stopped by its wall-time cap after
7,590 games. 17,590 games gave 440,242 training, 54,292 validation and
53,820 test decisions (4.4 % with a Kontra or Re option). Of the 17,471
games played, 14,875 were Farbspiele, 2,584 Grand and 12 Null; 41 Hand
games, 4 Ouvert.

**Tuning** on 2,020 games of run 73 (30 epochs at most, validation
split): hidden widths 128, 256, 512 and 1,024 with learning rates 0.001
and 0.0003 reached 66.9–69.4 % agreement with Strong (512 at 0.001 best);
soft targets, value mixes 0 and 1, value weight 0.3 and batch 1,024 moved
it by at most 1.2 points. Recipe: width 512, learning rate 0.001, batch
256, value mix 0.5, 30 epochs.

**Reproduction**: the shipped recipe was trained three times on the CPU
with trainers built at three commits (the last `bf17a5678616`, recorded
in `card-play.json`); all three wrote the same model file, SHA-256
`ce64c83c…`, each in 30–45 minutes (other jobs ran alongside).

**Learning curve** (option heads on; models trained on the first 100,
250, 500 and 1,000 shards of run 73, all measured on its test split of
31,030 decisions):

| Games | Agreement with Strong | Option score RMSE | Best option = Strong's card |
|-------|----------------------|-------------------|-----------------------------|
| 1,000 | 65.0 % | 67.5 | 56.4 % |
| 2,500 | 68.6 % | 61.7 | 60.1 % |
| 5,000 | 71.7 % | 57.8 | 60.6 % |
| 10,000 | 73.6 % | 53.4 | 61.5 % |

Agreement still grows by about two points per doubling of the data.

**Option heads** (A/B on all 17,590 games, test split of 53,820
decisions): A without option heads (`--score-weight 0 --win-weight 0`),
B with them.

| Model | Agreement | Value RMSE | Option score RMSE | Best epoch |
|-------|-----------|------------|-------------------|------------|
| A | 75.0 % | 48.0 | – | 4 |
| B (shipped) | 75.6 % | 45.3 | 48.4 | 10 |

**Arena** (`calibrate`, validation suite, 200 deals × 6 games, Kontra/Re
on; the fallback for pre-play is the rules of thumb shared by Easy,
Medium and Strong; A − B in Seeger-Fabian points per seat-game with 95 %
interval). A, B and B by option scores were compared on deals 0–199;
the choice of B was then checked on fresh deals 200–399.

| A | B | Deals | A − B | 95 % interval |
|---|---|---|---|---|
| A (no option heads) | Easy | 0–199 | +2.94 | [−1.41, +7.29] |
| B | Easy | 0–199 | +6.85 | [+2.77, +10.93] |
| B by option scores (`nnq:`) | Easy | 0–199 | +4.05 | [+0.43, +7.67] |
| B | A | 0–199 | +4.19 | [−0.63, +9.00] |
| B | Easy | 200–399 | +6.71 | [+2.06, +11.37] |
| B | Medium | 0–199 | −12.46 | [−16.97, −7.96] |
| B | Strong | 0–199 | −18.13 | [−22.40, −13.85] |

All 8,400 games finished without an illegal action (the arena aborts on
one); 24 of each 1,200 games on deals 0–199 and 6 on deals 200–399 were
thrown in.

**Latency** (`latency 20 nn:<file>`, 20 positions per kind): card play and Kontra decisions
of the model take 0.14–0.15 ms median, at most 0.20 ms p90 and 0.32 ms
maximum on one thread (pre-play rows measure the fallback); Strong's
first card takes 0.97 s median. Measured while one training process ran
alongside.

These are measurements, not gates (D-07): the network plays card play
better than Easy and worse than Medium; Classic stays the default.
Limits are listed in the model card.

## Self-play refinement

Outcome learning for the card-play network (#75), `nn-train rl`
(`tools/nn-train/src/rl.rs`). It starts from a model file (the #73
network), which also serves as the frozen reference.

- **Algorithm**: proximal policy optimisation (clipped surrogate, clip
  0.2) with a Monte-Carlo baseline. The reward of a decision is the
  acting seat's final Seeger-Fabian points (0 if thrown in); the
  advantage is the reward minus the value the network predicted at the
  decision, normalised per iteration. Further terms: the value head on
  the reward, the option heads on the taken card's points and win (so
  `nnq:` and the option estimates stay meaningful), a small entropy
  bonus and a cross-entropy towards the reference policy. Adam with a
  constant learning rate on the CPU. No bootstrapping, no privileged
  critic: every head sees only the encoded `PlayerView` of the acting
  seat; no defender shares memory with the other.
- **Iteration**: play `--games` games (default 600) with the current
  learner sampling its card-play and Kontra/Re decisions from its policy,
  then `--passes` passes of mini-batch Adam over its decisions. The
  learner decides pre-play by the rules of thumb (the fallback of a
  card-play model); pre-play is not learned until #74.
- **Pool**: per game one kind is drawn by weight (`--pool`, default
  `easy=1,medium=2,strong=2,ref=1,snap=3`): Classic Easy, Medium or
  Strong, the reference model, or one of the newest learner snapshots
  (`--snapshot-every` 10 iterations, `--pool-snapshots` 8; greedy by
  their policy). Seats rotate with the game index like the arena's
  seatings: the learner alone on each seat against two copies of the
  opponent, or as a pair against one.
- **Seeds and resume**: deals, pool draws, opponent seeds and the
  learner's sampling come from separate keys mixed with the run seed, the
  iteration and the game; results do not depend on the worker count.
  Every schedule depends on the iteration number, never on wall time, so
  a run cut into segments ends with the same weights as one
  uninterrupted run of the same length. After each iteration a
  checkpoint (weights, Adam state, logs, snapshot list with hashes) is
  written; rerunning the same command resumes it, and another recipe,
  another start model or a corrupt checkpoint is refused. `--minutes`
  and `--iterations` cap a run, `OUT_DIR/STOP` ends it after the
  running iteration; `model.gcnn` and `rl.json` (recipe, trainer commit,
  per-iteration logs with results per pool kind, snapshots) are written
  at the end.
- **Selection**: snapshots are screened with `calibrate` on the tuning
  suite; the chosen one is measured on a fresh block of the validation
  suite against Classic and the #73 model. It replaces
  `assets/models/card-play.gcnn` only if it beats the #73 model there.
