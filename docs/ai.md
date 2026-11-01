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
in issue #69. No trained model ships yet; Classic stays the default.

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
  width, a policy head (one logit per action) and a value head (expected
  Seeger-Fabian points of the acting seat). The same definition trains
  and infers.
- **Model file**: a 24-byte header (magic `GCNN`, format and encoder
  version, layer sizes, covered phases) and the weights in Burn's
  burnpack format. The caller passes the bytes and their SHA-256 hash;
  `skat-nn` reads no files.
- **Safety**: illegal actions are masked out before the softmax, and the
  chosen action is checked against the view again before acting. A
  missing model, a wrong hash, an unknown format or encoder version, a
  shape that does not fit, corrupt weights, an uncovered phase or a
  non-finite output give a `Fallback` reason; `NeuralOpponent` then lets
  its Classic `Opponent` decide (deterministic, same seed) and reports
  the reason (`NeuralDecision::fallback`).
- **Estimates**: policy probabilities and the value are model estimates.
  They are not `Estimate`s of the analysis and never claim `exact`,
  Monte-Carlo samples or a reliable comparison.
- **Arena**: `Player` is implemented by `Opponent` and `NeuralOpponent`;
  `arena::play_with` and `arena::compare_with` seat any players, and
  `calibrate` accepts the profile `nn:<file>[@<level>]`. Tests use a
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
