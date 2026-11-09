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

### Role tournament

`calibrate` compares two profiles. To rank many players by role, the
example `tournament -- CONFIG OUT.tsv <suite> <deals> <parallel> <first>`
plays configured constellations (`match DECLARER DEFENDER DEFENDER`, players
as in `calibrate`) on the same suite deals. All Classic levels except
Rookie and every model fallback bid and declare by the same rules of thumb,
so a deal's declarer seat and contract do not depend on the seating: the
runner finds them with three Easy players, skips thrown-in deals and plays
each match once per deal with its declarer on that seat. Mixed defender
pairs swap seats between even and odd deals. A game whose declarer or
contract differs is flagged. Seeds depend on suite, deal and seat only, so
all matches of a deal share random numbers and differences are paired.
Results are one tab-separated row per game, appended; a restart skips the
games already written.

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

### Experimental opponents

Select **Experimental** in the new-series dialog to play against two
permitted-view opponents with Strong's bidding, declaring, sampling,
selection and node budget. The temporary mode starts at 24 worlds.
In suit and Grand play the CPU solver tightens its existing cashing
bound. For every guaranteed declarer trick each defender must contribute
a card, so the cheapest possible such cards from both hands give a safe
additional lower bound on Augen. The live coach uses this bound too and
can show a provisional result after 200 ms during card play.

The [latency measurements and GPU threshold probe](../crates/skat-analysis/experiments/strong-gpu/README.md#decision-latency)
give the sample counts and measured limits. The calibration tool accepts
`experimental` as a side. See the [paired result](#experimental-versus-strong).

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

### Experimental versus Strong

On 2026-11-03, commit `65132f6`, both opponents used 24 samples and
100,000,000 nodes per task. The release calibration ran 200 validation
deals (12,000–12,199), six seatings per deal, with Kontra/Re on. All
1,200 games had a declarer. With ten deals in parallel the run took
34 min 29 s.

| Pair | Deals | Difference per seat-game | Std. error | 95% interval | Verdict |
|---|---:|---:|---:|---:|---|
| Experimental − Strong | 200 | +1.83 | 1.92 | [−1.94, +5.59] | inconclusive |

| Opponent | Points/seat-game | Declarer games | Declarer won | Points/declarer game | Defender games | Defenders won | Points/defender game |
|---|---:|---:|---:|---:|---:|---:|---:|
| Experimental | 28.51 | 600 | 74.2% | 64.1 | 1,200 | 26.8% | 10.7 |
| Strong | 26.69 | 600 | 72.7% | 59.0 | 1,200 | 26.3% | 10.5 |

```sh
cargo run --locked --release -p skat-ai --example calibrate -- validation experimental strong 200 10 12000
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
- **Encoder**. New supervised networks take 634 numbers from the acting
  seat's `PlayerView` (v2). The first 294 are the frozen v1 layout,
  including the hand, known Skat, Ouvert reveal, played cards by seat,
  ordered current trick, position, bidding, contract and trick Augen per
  party. For each card in a completed trick, v2 records its play number
  from 1 to 30 (scaled by 30, zero until played). Every three successive
  numbers form one trick. The per-seat card sets identify who led it.
- A failed follow in a completed or current trick marks its led class void
  for that relative seat. Classic and the encoder use the same public
  evidence. Buben count as Trumpf in Farbspiel and Grand and follow their
  printed suit in Null. A clear flag means no observed failure to follow.
- Card masks identify the unlocated cards and all Trumpf of the contract.
  Five groups have explicit counts for each side suit and Trumpf, and
  separate Augen totals: all unplayed cards (including Skat), own hand,
  known Skat, another seat's exposed hand, and unlocated cards. The last
  four partition the first. Until a contract is announced the counts use
  printed suits. The unknown Skat stays pooled with concealed hands.
  Augen in the current trick and Augen won in tricks by each relative seat
  are separate. Only the declarer after Drücken can count the known Skat
  towards their secured Augen. The layout and scaling are in `encoder.rs`.
- How earlier decisions were taken, the evidence about hidden hands that
  card sets do not show: for each card of a completed trick whether it was
  led, followed, trumped or discarded (follow classes of the contract), and
  whether it fell into a trick won by its player's own party (schmieren)
  or by the other party; per relative seat the bid standing when it
  passed, an upper bound on its hand.
- Raw decision shards already store ordered tricks, winners and the
  current view. Training encodes these rows on load, keeping labels and
  deal splits. The recipe records its active encoder separately from
  each source manifest's generation-time version. Runtime inference,
  evaluation and PPO use the input layout of their model header.
- **Action space** (`skat_nn::actions`): 621 indices: 63 bid values,
  Hold, Pass, Hand, TakeSkat, 496 discard pairs, 22 announcements,
  32 cards, Kontra, KeinKontra, Re, KeinRe. Part of the encoder version.
- **Network** (`PolicyValueNet`): input, hidden ReLU layers of equal
  width (two by default; `nn-train train --layers N` for up to eight), a
  policy head (one logit per action), a value head (expected
  Seeger-Fabian points of the acting seat) and two option heads (#73):
  per action the expected Seeger-Fabian points after it and the logit of
  the probability that the seat's party wins after it. The same
  definition trains and infers. `NeuralModel::evaluate_with` picks the
  action by the policy (`Choice::Policy`, default) or by the highest
  expected points (`Choice::Score`); every evaluation carries the option
  estimates.
- **Model file** (format version 2; version 1 had no option heads): a
  24-byte header (magic `GCNN`, format and encoder version, layer sizes,
  covered phases, number of hidden layers with 0 meaning two, so files
  written before it was recorded stay valid) and the weights in Burn's burnpack format. The caller passes the bytes and their SHA-256 hash;
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

- **Games**: three Strong opponents per deal, ISkO with Kontra/Re
  (`--table teacher`, the default), or Easy players in card play
  (`--table easy-mix`, [below](#easy-mix-tables)).
  Every decision of every seat is recorded, so Reizen, Hand/pickup,
  Drücken, announcement, Kontra/Re and card play are all covered. Strong
  bids and declares by the rules of thumb, so those records carry no
  option values.
- **Record** (format version 2, one JSON line each): deal index and split,
  the acting seat's `PlayerView` (with phase and legal actions), the
  teacher's chosen action, Strong's option values (`OptionValue` per legal action)
  with the analysis status and fallback flag, and the seat's final
  Seeger-Fabian points (0 if thrown in). Easy-mix card play adds the
  seat lineup and, at an Easy seat, the action Easy actually played;
  format-1 records read as teacher-only. Records are raw; the encoder
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

### Easy-mix tables

Strong-only games stay in the positions Strong steers into, and the
network meets other positions against weaker players or when it errs.
`--table easy-mix` widens the positions without teaching weak play:

- Strong bids and declares at every seat. Once card play starts, the
  run draws the card-play roles (declarer, defender after the declarer,
  second defender), each Strong or Easy, from its own seed key,
  independent of the cards. The declarer's seat follows from the deal,
  so all 24 combinations of seat and role occur. Bidding by Strong keeps
  the roles independent of the lineup, so they can be weighted.
- Weights: lineups with exactly one Easy player weigh 2, the other five
  1 (`EASY_MIX_WEIGHTS`). One weak player beside two strong seats is the
  trainer's table: a learner declaring against strong defenders (`E|TT`)
  or defending beside a strong partner (`T|ET`, `T|TE`).
- At an Easy seat Strong still analyses and labels the decision. Easy's
  action only moves the game on. A good Easy move scores high in
  Strong's option values, a mistake low; Easy's choice is never a target.
- Training uses Strong's option score alone as the value target of
  mixed card play, because the final points reflect Easy's play.
  `--soft-weight` makes the policy learn from all option values, not only
  the best one. `evaluate` splits the test metrics by who held the seat
  and by roles.
- `verify` replays Easy's actions and checks every lineup against the
  run's draw.

Further mixed tables work the same way (bidding by Strong, roles drawn
from their own key, Strong labels at every seat, `verify` replays the
weaker players' actions):

| `--table` | Card-play role players |
|-----------|------------------------|
| `rookie-mix` | Rookie or Strong; each of the 8 lineups equally likely (0 to 3 Rookies) |
| `medium-mix` | Medium or Strong; as `rookie-mix` |
| `level-mix` | Easy, Medium or Strong per role, independently (27 lineups) |

No table seats Mephisto. Mephisto decides with every card known, so its
bids and cards would carry information about hidden hands that no real
opponent gives (the belief head would learn it), and positions it reaches
are labelled by Strong's model of ordinary opponents. Mephisto stays a
table mode and an evaluation opponent only.

Pilot on 2026-11-05 (20 CPUs, 10 workers × 2 threads, seed 0, 100
deals, release build, engine `bac189f` plus this change): 4,118 records
in 40 s, 8,929 deals/hour (Strong-only on the same commit: 8,098),
73,037 bytes per deal; roles `T|TT` 9, `T|TE` 22, `T|ET` 20, `T|EE` 6,
`E|TT` 23, `E|TE` 7, `E|ET` 8, `E|EE` 5 deals; 247 Easy card-play
actions differed from Strong's label. Strong still analyses every
card-play decision, so throughput stays about the same; each timing is a
single run.

A classic Strong sampler trial on the RX 6900 XT reached 1,903 deals/hour
with GPU draws and 1,929 on CPU in ten-minute generator runs. See the
[experiment and reproduction steps](../crates/skat-analysis/experiments/strong-gpu/README.md).

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

### Experimental teacher throughput

On 2026-11-03 the release generator ran sequentially on an i9-10900K
with seed 0, four game workers and five analysis threads per worker.
Each seat used the same teacher and 24 samples. Experimental used the
CPU search bound. Times include generation and shard writes. The
ten-minute cap lets games already in progress finish.

| Workload | Strong | Experimental | Throughput gain |
|---|---:|---:|---:|
| Same 100 deals, ten deals/shard | 3 min 27.2 s | 3 min 08.4 s | +10.0% |
| Ten-minute cap, one deal/shard | 319 deals in 604.6 s | 340 deals in 607.2 s | +6.1% |
| Deals/hour at the ten-minute rate | 1,899 | 2,016 | |
| Projected time for 10,000 deals at that rate | 5 h 16 min | 4 h 58 min | |

All four datasets passed verification for their completed shards. The
fixed runs produced 4,152 byte-identical records each. The timed runs
produced 13,541 and 14,431 records respectively. Of their 319 common
deals, 318 matched byte for byte. On deal 239 Strong exhausted its node
budget on the opening lead. Experimental completed the search on the
same view and chose a different card. Across the timed runs Strong
exhausted the budget on three decisions and Experimental on two.
Each timing is a single run.
See the [measurement patch and commands](../crates/skat-analysis/experiments/strong-gpu/README.md#reproduce-the-teacher-comparison).

### Faster Strong search

`generate` uses the compact CPU Augen solver for Strong. The teacher still
evaluates 24 sampled worlds and records every legal option. `verify` also
replays each deal and checks every acting view, action and final score.
The [CPU and Vulkan measurements](../crates/skat-analysis/experiments/strong-gpu/README.md#early-search-endgames)
include the procedure for comparing complete, byte-identical training shards.

On 2026-11-03 the i9-10900K ran the release generators sequentially on
the same 1,000 deals (seed 0, ten deals/shard, 20 logical CPUs). Each cell
is the median of three wall-clock runs including startup and shard writes.
The reference was `4eb0d75`. All 18 runs produced byte-identical shards
with 42,486 records and the same eight incomplete analyses. On deal 239
the opening lead retained all ten options, each with 23 completed worlds.

| Game workers × analysis threads | Previous Strong (deals/h) | Optimised Strong (deals/h) |
|---|---:|---:|
| 4 × 5 | 1,917 | 7,280 |
| 10 × 2 | 2,033 | 7,904 |
| 20 × 1 | 2,028 | 7,946 |

Best median against best median was **3.91× throughput**, or 74.4% less
elapsed generation time for a fixed dataset. In a separate serial run
with one interactive decision at a time, 30 positions per kind gave:

| Decision | Previous median / p90 (s) | Optimised median / p90 (s) |
|---|---:|---:|
| Kontra/Re | 0.306 / 2.704 | 0.088 / 0.696 |
| Trick 1 | 0.771 / 2.244 | 0.216 / 0.681 |
| Trick 2 | 0.081 / 0.649 | 0.021 / 0.194 |
| Trick 3 | 0.033 / 0.088 | 0.011 / 0.021 |

Actions, node counts, completed worlds and statuses matched on all 120
positions. Experimental additionally uses adjacent-card bounds. On fresh
validation deals 14,000–14,199 (six seatings, 1,200 games), Experimental
minus Strong was +0.40 Seeger-Fabian points per seat-game (95% interval
[−3.28, +4.09]). Relative playing strength was inconclusive.

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
  Mini-batch Adam on the CPU or with `--device vulkan` (the default
  adapter); `vulkan:discrete[:N]` and `vulkan:integrated[:N]` pick the
  N-th adapter of that kind, so parallel runs can use different GPUs.
  `--queue LxH,...` trains several shapes in one process on data loaded
  once (one copy in RAM instead of one per run): one thread per
  `--workers` device takes the next shape when it is free (`last:DEVICE`
  from the end of the list, `once:DEVICE` only one), each into
  `OUT_DIR/<L>x<H>` with its own recipe, checkpoint and `train.log`.
  CPU training uses about one core per worker, so several `cpu` workers
  run in parallel; they slow each other down (2x384 on all data: 45 min
  per epoch with eight CPU workers, 4x896 on the RX 6900 XT 10 min).
- **Training data in memory** (`tools/nn-train/src/{features,cache,resident}.rs`):
  examples keep one byte per feature, a code into a per-feature table of
  the exact `f32` values (lossless, bit-identical on decode; more than 256
  distinct values of a feature stop the load instead of rounding), about
  a quarter of the earlier `f32` rows. `--cache DIR` writes the loaded
  data once to `DIR/<key>.gcdata` and reads it back on later runs; the
  key covers the cache format, encoder version, every source run
  (manifest and the shard hashes of its `.done` files, in order) and the
  target hyperparameters (`soft_weight`, `soft_temperature`, `value_mix`,
  belief targets), so changed data or targets never reuse a stale file;
  the run directories are only read. `--data-on-device GB` keeps the
  training split's feature codes on the training device (four per `i32`,
  1 GiB buffers) if they fit in GB; a step then uploads row indices and
  the sparse per-action targets, and the device decodes features and
  scatters targets into the dense tables. Batches have the same values as
  the RAM path (a test trains both bit-identically on the CPU); without
  enough budget the run falls back to RAM. Neither option enters the
  recipe, so a run resumes with or without them. Measured on 2026-11-07
  (100,000 easy-mix games, 2.45 million training examples, 2×512, RX 6900
  XT via Vulkan): loading from the runs 69 s (before: 83–164 s), from the
  cache 2 s (2.3 GB file); peak RAM 2.5 GB; one epoch 61 s at batch 256
  and 32 s at batch 1024 from RAM, 58 s and 30 s with the rows on the
  device (1.6 GB). The losses from RAM equal those of the earlier `f32`
  storage. Device residency mainly helps when the CPU is busy elsewhere.
- **Reproducibility**: initial weights and example order come from the
  recipe's seed (SplitMix64), never from a device's random state. After
  every epoch a checkpoint (weights, Adam state, best weights, each with
  its SHA-256) is written and `state.json` points at it last; a restart
  with the same recipe resumes bit-identically, a different recipe or a
  corrupt checkpoint is refused. No epoch starts after the wall-time cap
  (`--minutes`). With `--patience N`, N epochs without a lower validation
  loss continue from the best weights (Adam state kept) with the learning
  rate times `--lr-decay` (default 0.1), at most `--lr-decays` times
  (default 1); the next plateau stops the run (`plateau` in
  `train.json`). The decays are part of the checkpoint, so a resumed run
  stays bit-identical. On 2026-11-04 one decay to lr 0.0001 after the best
  epoch of a 2×512 run on 66,700 Strong games lowered the validation loss
  from 1.778 to 1.727 and gained +0.73 points per seat-game (95 %
  interval [−0.27, +1.72], 4,000 deals) against the undecayed model.
  The output is the epoch with the lowest validation loss
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

## Decision dependencies

The decisions of one game depend on each other in two ways. Their value
targets are shared: every card-play row of a seat carries the same final
points. Their information is causal: each earlier action of another seat
follows from its hidden hand. `nn-train probe` measures both on whole
validation deals (`tools/nn-train/src/probe.rs`):

- the intra-class correlation (ICC) of the value residuals per (deal,
  seat) and per deal, with the effective number of independent rows;
- the change of policy loss and agreement when each encoder block is
  zeroed, overall and by completed tricks;
- bits per hidden card of its true place (replayed deal) under the hard
  constraints alone (uniform samples, as Classic), with Classic's bidding
  evidence, and under the learned belief head.

`train --belief-weight X` adds that head: a training-only linear layer on
the last hidden layer. It predicts the place of every card the seat cannot
see (next seat, seat after, Skat), so the shared layers have to infer
hidden hands from the history. The model file stays a plain network; the
head is written to `belief.bpk` for the probe. Validation loss for model
selection excludes it. It adds about 5 % per epoch (64.4 s → 67.9 s) and
nothing at inference.

Runs on 2026-11-03 (i9-10900K, CPU, the #73 data and recipe: 2×512,
lr 0.001, batch 256, seed 0, at most 12 epochs, stopped after two epochs
without a better validation loss). Pilot: the earlier 439-feature v2. M1:
the extended 634-feature v2. M2: M1 with belief weight 0.5. Probes use the
same 10,026 rows of 320 validation deals, 256 samples per row.

| Model | Best epoch | Test agreement | Value RMSE train / validation | ICC per (deal, seat) |
|---|---|---|---|---|
| v1 #73 supervised | 10 | 75.6 % | – / 45.3 | 0.780 |
| v2 pilot | 7 of 9 | 75.65 % | 24.3 / 43.4 | – |
| M1 | 7 of 9 | 75.09 % | 22.9 / 45.5 | 0.821 |
| M2 | 12 of 12 | 74.96 % | – / 44.8 | 0.787 |

An ICC near 0.8 means 10,026 rows carry about as much value information
as 1,150–1,200 independent ones. The residual against the teacher's
chosen-option value has an ICC of 0.45–0.49. The value overfits within a
few epochs (train RMSE half of validation), so the shared target, not the
encoder, limits the value head.

Hidden-card bits (lower is better):

| Completed tricks | Constraints | + bidding evidence | M2 belief head |
|---|---|---|---|
| 0 | 1.228 | 1.204 | 1.176 |
| 2 | 1.229 | 1.202 | 1.177 |
| 4 | 1.183 | 1.152 | 1.166 |
| 6 | 1.133 | 1.093 | 1.178 |
| 8 | 1.025 | 0.960 | 1.153 |
| all | 1.184 | 1.153 | 1.170 |

The head beats Classic's beliefs only in the first tricks (about 0.03
bits per card). Later it loses, because it does not enforce voids and hand
sizes exactly. In Strong self-play the card play reveals little about
hidden hands beyond the constraints. Zeroing the new blocks (play context,
party of the trick, pass values) changes M1's policy loss by at most
0.012. Zeroing the unseen-card mask changes it by 0.33, the Trumpf mask by
0.13.

Arena (`calibrate`, tuning suite, A − B in points per seat-game, 95 %
interval):

| A | B | Deals | A − B | 95 % interval |
|---|---|---|---|---|
| v2 pilot | v1 #73 supervised | 2,000 | −1.95 | [−3.55, −0.34] |
| M1 | v1 #73 supervised | 2,000 | −0.64 | [−2.15, +0.88] |
| M2 | v1 #73 supervised | 2,000 | −2.46 | [−3.89, −1.04] |
| v2 pilot | v1 RL arm A (iter 170) | 2,000 | −7.25 | [−8.99, −5.51] |
| M1 | v1 RL arm A (iter 170) | 2,000 | −8.20 | [−9.91, −6.50] |
| M1 | Easy | 400 | +5.61 | [+2.39, +8.83] |
| v2 pilot | Medium | 400 | −13.88 | [−17.75, −10.01] |
| M1 | Medium | 400 | −13.03 | [−16.40, −9.66] |
| M1 | Strong | 200 | −14.60 | [−19.06, −10.14] |

M1 ties v1 and the belief head costs strength at weight 0.5. These are
measurements, not gates (D-07). The extended v2 encoder stays; the belief
head stays a diagnostic; the value target shared per game is the next
lever.

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

**Pilot** on 2026-11-02 (reference hardware of
[analysis.md](analysis.md#latency-target-and-benchmark), release build,
commit `e84ec58`, default recipe, seed 0, `--minutes 30 --cpus 16`, start
`card-play.gcnn` of #73; builds and tests ran alongside at first): 11
iterations of 600 games (6,600 games, about 13,000 games and 200,000
learner decisions per hour) in 30 minutes; an iteration took 119–189 s
playing and 2 s updating. Per iteration the learner made 9,283–9,397
decisions; the KL divergence from the reference grew to 0.050 nats and
the policy entropy from 0.516 to 0.574 (the same with entropy weight 0.01,
0.001 or 0 in a 12-iteration check against a cheap pool, so the bonus does
not drive it); 1.1–3.0 % of the ratios were clipped. Results per pool
kind in single iterations vary by about ±15 points, so the learning curve
is judged by `calibrate`. Screening on the tuning suite (deals 0–399,
6 games each):

| A | B | Deals | A − B | 95 % interval |
|---|---|---|---|---|
| snapshot after 10 iterations | #73 model | 0–399 | +3.37 | [+0.46, +6.27] |
| learner after 11 iterations | #73 model | 0–399 | +2.17 | [−0.98, +5.33] |
| snapshot after 10 iterations | Medium | 0–99 | −8.23 | [−14.85, −1.60] |
| #73 model | Medium | 0–99 | −8.05 | [−15.50, −0.60] |
