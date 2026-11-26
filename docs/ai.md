# Opponents and calibration

The app offers four Classic levels per opponent and five special table
modes. The frontend advances computer turns through `skat-app`. Each
ordinary opponent decides from its own legal actions and permitted history.
Deal, opponent and coach seeds use separate streams.

## Levels

| Level | Before card play | Kontra | Card play, Re | Budget |
|-------|------------------|--------|---------------|--------|
| Rookie | rules of thumb with mistakes | rules of thumb with mistakes | rules of thumb with mistakes | no search |
| Easy | rules of thumb | rules of thumb | rules of thumb | no search |
| Medium | rules of thumb | rules of thumb | `analyze` | 6 samples, 10⁸ nodes per task |
| Strong | rules of thumb | rules of thumb | `analyze` | `Budget::INTERACTIVE` (24 samples) |

- The rules of thumb are `BaselineOpponent`: the shared continuation
  policy of `skat-analysis` (`policy`) plus simple card rules.
- For equally sampled options, Medium and Strong maximise expected seat
  points. Defenders subtract half the declarer's expected game points.
  The rules-of-thumb action gets a 0.5-point selection bonus.
- The default Kontra rule requires at least five trumps and declines Kontra
  in the Ouvert round. In a paired role tournament the double-dummy policy
  called Kontra in 69% of defended games even though the declarer usually
  won. Switching Kontra to the rules gained defending Medium and Strong
  pairs about 23 and 29 points per game. Re remains searchable.
- Analysed bidding also passed too often against perfect double-dummy
  defenders. Classic levels therefore use the rules before card play.
  Experimental profiles can enable analysed bidding or selection noise.

### Experimental opponents

Both seats use the Strong profile with an additional safe lower bound on
declarer Augen in Farbspiel and Grand. The coach can show provisional
progress during card play. See the [search experiments](../crates/skat-analysis/experiments/strong-gpu/README.md).

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

## Neural engine selection

Ordinary tables select Classic (default) or experimental Neural independently
of each seat's level. Neural admits Easy, Medium and Strong; Rookie remains
Classic-only (switching a Rookie draft to Neural selects Easy). All choices
remain experimental: adjacent-level calibration did not establish an ordering;
Classic equivalence has not been measured. The legacy `Opponents::Neural` table stays greedy/Strong.

`skat-nn::PolicyLevel` freezes `temperature-policy-1`: Easy 2, Medium 1,
Strong 0. Positive temperatures sample the stable legal-logit softmax using
ChaCha8 and an explicit decision seed; zero preserves historical greedy policy
and first legal tie. `NeuralOpponent` derives that seed from its opponent seed
and permitted-view facts, using the existing decision-seed helper. Neither
model inputs nor dealing/coach RNG change. Fixed bytes, revision, settings,
seeds and human commands reproduce actions on the same build/CPU backend.
Floating-point inference is not promised bit-identical across platforms;
cancellation timing of Classic fallback search is excluded. Healthy models choose cards/Re;
pre-play, Kontra and unsupported/unusable output use the selected Classic level.

Sessions freeze bytes/load failure, actual hash, encoder and policy revision.
Format-2 recordings retain these and per-seat levels in SessionConfig; replay
needs no weights. Practice preserves the recorded supported revision, including
`greedy-policy-1`, or visibly uses Classic for unknown/different/unavailable
weights. Legacy unknown identity uses Strong. The provenance `fallback` field
retains the historical Strong default; temperature profiles use SessionConfig's
per-seat levels. Settings apply after Start; Cancel retains the series/workers.
Loading success/failure is process-cached; retry requires restart.
`OpponentJob::run_explained` exposes structured fallback reasons.

`calibrate` accepts `nn-easy:PATH`, `nn-medium:PATH`, `nn-strong:PATH` using
these exact profiles and matching Classic fallbacks. It prints full hash,
encoder, revision, temperature and fallback along with the existing suite/range.
Legacy `nn:PATH@LEVEL` and `nnq:` keep their semantics. Parser tests exercise
plumbing without playing an arena. The following validation used the bundled
PPO v2 model (SHA-256 `6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75`,
encoder 2, 4 × 384), `temperature-policy-1` and each level's corresponding
Classic fallback. On 2026-11-18, job 2089 ran the protocol with six parallel
deals, validation indices 0–199, 200 deals and six seatings per pair, with
Kontra/Re enabled. No tuning or training was run.

| Comparison (A − B) | Points per seat-game | Standard error | Paired 95% interval | Result |
|---|---:|---:|---:|---|
| Neural Medium − Easy | +3.01 | 1.77 | [−0.47, +6.49] | Inconclusive |
| Neural Strong − Medium | +0.48 | 1.34 | [−2.15, +3.10] | Inconclusive |

Each comparison included 24 thrown-in games out of 1,200. These intervals do
not establish an ordering or equivalence to Classic; the controls remain
experimental. Any further tuning and fresh validation follow the
[protocol](#calibration-protocol) and applicable compute holds. Historical
implementation scope and holds are in [the selection record](learned-engine-selection.md).

## Hybrid backend (experimental, #76)

The independent engine selector offers Hybrid with per-seat levels. Easy verifies
proposals with one sample; Medium and Strong use their respective Classic search
budgets. Pre-play and unavailable models use the selected Classic level.
Recordings freeze the model identity and hybrid revision; mismatches fall back
to Classic. `HybridOpponent` uses the shared
[learned-proposal verification](analysis.md#hybrid-proposals-experimental-76).

### Hybrid, Neural and Strong comparison

On 2026-11-18, the release `calibrate` runner compared all three pairs on
validation deals 0–199, six seatings per deal, Kontra/Re on, six parallel
deals, on an Intel i9-10900K in Fedora 44 Distrobox. Hybrid uses Strong's
budget and fallback; Neural uses `nn-strong` (greedy, Strong fallback); Strong
uses 24 samples and 100,000,000 nodes per task. Both learned players used
`assets/models/card-play.gcnn`, encoder 2, SHA-256
`6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75`.
All comparisons had 24 thrown-in games out of 1,200.

| Pair (A − B) | Points per seat-game | Standard error | 95% interval |
|---|---:|---:|---:|
| Hybrid − Strong | +0.26 | 1.41 | [−2.51, +3.03] |
| Hybrid − Neural | +1.65 | 1.12 | [−0.54, +3.83] |
| Neural − Strong | +0.10 | 1.35 | [−2.55, +2.74] |

All intervals include zero: none establishes an ordering or equivalence. These
results apply to the measured Strong profiles, not the other learned levels.
The per-pair games differ, so their point estimates need not add up. The
paired-deal interval and scoring are defined by the protocol below.

```sh
cargo run --locked --release -p skat-ai --example calibrate -- validation hybrid:assets/models/card-play.gcnn strong 200 6 0
cargo run --locked --release -p skat-ai --example calibrate -- validation hybrid:assets/models/card-play.gcnn nn-strong:assets/models/card-play.gcnn 200 6 0
cargo run --locked --release -p skat-ai --example calibrate -- validation nn-strong:assets/models/card-play.gcnn strong 200 6 0
```

The same host's [latency comparison](analysis.md#hybrid-latency-comparison)
measures decision time; it does not establish playing strength. This job's
specific evaluation was admitted; the existing hold on other arenas and
training remains in force.

## Calibration protocol

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
  points per role are reported as supporting information. Behavioural tables
  report Bid, Hold and Pass counts and rates over all Reizen actions (Bid
  includes Vorhand accepting 18). Per-profile Einpassen counts participation
  over seat-games, not causation by that profile on mixed tables. Contract
  mix (Kreuz, Pik, Herz, Karo, Grand, Null) uses only that profile's declarer
  games; Hand, Schneider/Schwarz announcements and Ouvert overlap those bins.
  Achieved Schneider/Schwarz are not counted as announcements. A Kontra/Re
  table counts Kontra said over played defender seat-games and Re over
  declarer games that faced Kontra; declined or unoffered chances are not
  distinguished, so it exposes a Kontra-always policy but is no quality
  measure. Zero
  denominators print `–`; these descriptive rates have no independent-row
  confidence intervals.
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
games already written. A player spec ending in `+ak` lets the analysis or
the model decide Kontra (`Profile::search_kontra`) instead of the rules of
thumb.

### Experimental versus Strong

An equal-budget check (24 samples, 200 validation deals, six seatings)
gave +1.83 Seeger-Fabian points per seat-game for Experimental, with a
95% interval of −1.94 to +5.59. This comparison did not establish a
strength advantage. Recalibrate when a profile or decision rule changes.

## Experimental teacher throughput

Measure completed deals per hour with `nn-train generate`, including
shard writes. Run `nn-train verify` and compare shard bytes for identical
seeds before adopting a faster backend for the same teacher.

## Faster Strong search

Two-stage card play (#104) remains an opt-in prototype. Strong, Medium,
the coach and the generator stay single-stage. The
[actual HQ520/HQ521 pilot table, source identity and hashes](analysis.md#actual-bounded-tuning-diagnostic-hq520-audited-by-hq521)
record 39/40 identical actions overall, plain card play 34/35 and screened
24/25; differing-choice regret 9.544. Per repeat, nodes were
343242621 → 252010059 overall, 261865780 → 170633218 plain and
246908764 → 155676202 screened. Stage outcomes were 25 screened,
5 all-survived, 5 few-option skips, 3 Kontra/Re skips and 2 absent reports.
Median summed wall ratio was 0.841197 (p90/max 0.923247).

Those are three timing repeats of 40 positions from five tuning deals,
with the reference always first, not fresh strength or label evidence.
Measured source `6246cb0` predates fix `15f098c`; it is not final-head
validation. No strength interval, certified speedup, label agreement or
end-to-end generator throughput follows from the diagnostic. The
[observation/tool no-go](issue104-observation-no-go.md) records the missing
interfaces and freezes the conditional budgets. Later paired tuning (93.5 %) and holdout (90.25 %) runs also missed the
97 % target: the [#104 conclusion](analysis.md#conclusion-for-104-no-go-for-the-proposal)
is a no-go and the default stays single-stage.

Time Kontra/Re and the first three tricks separately. Solver-only timings
miss the cost of sampling, all legal options and output. Use the
[reproduction procedure](../crates/skat-analysis/experiments/strong-gpu/README.md#early-search-endgames).

## Learned pre-play preparation (#74)

The [all-phase imitation tooling](learned-models.md#all-phase-imitation-preparation-74)
now preserves the #70 records' pre-play and joint discard labels through the
existing shared encoder/head, with explicit recipe, cache and model phase
coverage. Verification is limited to tiny fixture training/reload/legal
inference and scripted visibility/Re schema tests. There are no production
all-phase imitation weights, behavioural rates, contract/role statistics or
playing-strength results yet. Independent label validation exists (`nn-train validate-labels`; first
result in the [pre-play model card](pre-play-model-card.md): 405/405 labels
of a 40-deal Strong run equal the Classic rules, no leaks); the outcome-label pilot is postponed until it and imitation calibration are
available. Classic and the shipped Neural model and Kontra behaviour are
unchanged.

### First all-phase imitation run (#74)

Measured 2026-11-19 on this PC (CPU, 8 cores, nice 10; compute pre-admitted,
the earlier Phase 2 hold is superseded). `nn-train train M T-2-10k
--all-phases --hidden 384 --layers 4 --epochs 4` on the retained Strong
`T-2-10k` set (10,000 deals, seed 73; labels validated in the
[model card](pre-play-model-card.md)): 338,484 train / 43,441 validation /
42,322 test rows, 250 s. Epoch 3 was kept (lowest validation loss; epoch 4
overfit). Model SHA-256
`6ed45d3400b71b42adf4dc7333c14a59a613056623914c3c49b628fae5ad9aa8`.
Imitation agreement with the recorded teacher choice over all five phases
pooled: 71.86 % validation (43,441 rows), 71.59 % test (42,322 rows). This
pooled figure is dominated by card-play rows; it is no per-phase accuracy.

Complete games (`calibrate validation nn:M/model.gcnn@easy B 200 4 10000`;
200 deals × 6 seatings = 1,200 games, Kontra/Re on, Kontra by the existing
rules; the model decides Reizen, pickup, Drücken, Ansage and cards):

| B | A − B per seat-game | 95 % interval |
|---|---:|---|
| Classic Easy | -34.60 | [-40.27, -28.94] |
| Classic Strong | -43.39 | [-49.13, -37.65] |
| shipped card-play model (Classic pre-play) | -44.98 | [-50.73, -39.23] |

The all-phase imitation model is clearly **weaker** than all three. Behaviour
(A vs Strong table; 1,200 games): A bid 2,755 (40.0 %), held 3,102 (45.1 %)
and passed 1,026 (14.9 %) of 6,883 Reizen actions, against Strong's 48.0 / 28.5 /
23.5 % of 5,846; Einpassen 3/1,800 seat-games for both. A declared 774 games
(Strong 424), won 42.9 % (Strong 90.8 %) at -41.5 points per declarer game
(Strong +76.8); as defender it won 30.6 % of 1,023 games (Strong 47.3 % of
1,373). A's contracts: Kreuz 194, Pik 235, Herz 102, Karo 185, Grand 58,
Null/Hand/Schneider/Schwarz/Ouvert 0 of 774. The model therefore over-reaches
in Reizen (few passes, many weak declarer games) and never chooses Null, Hand
or announcements; the rare-label classes are not learned by this small run.
Classic Easy/Strong tables use their own pre-play. No claim of a playable
learned opponent is made; the model is not shipped, and Classic stays the
default. One seed, one dataset, one shape and a 200-deal sample: this is a
first measurement, not a tuned result. Raw outputs:
`grand-coach-data/studies/2026-11-19-preplay-74/`.

#### Improved imitation recipe (#74)

Measured 2026-11-19 on this PC (CPU, nice 10; commit of this change). New
opt-in `train --all-phases --preplay-weight W` scales the policy loss of every
row that is not plain card play (Reizen, Hand/pickup, Drücken, Ansage,
Kontra/Re); `W` 1 is the previous recipe. The data are three retained Strong
sets, `T-2-10k`, `T-3-8k` and `T-8-11k` (≈ 28,500 deals, 967,137 train /
120,049 validation / 120,273 test rows, three times the first run), 4×384,
`--epochs 6 --patience 1` (best epoch kept, 1,041–1,070 s per run), seed 0.
`evaluate` now prints agreement with the teacher per phase. Test agreement
(the validation rows behave alike; correct/rows):

| Phase | W = 1 | W = 4 | W = 8 |
|---|---:|---:|---:|
| Reizen (23,034) | 93.73 % | 96.41 % | 95.11 % |
| Hand/pickup (2,827) | 99.58 % | 99.68 % | 99.61 % |
| Drücken (2,815) | 72.75 % | 79.29 % | 78.44 % |
| Ansage (2,827) | 87.44 % | 91.44 % | 91.40 % |
| Kontra/Re (3,967) | 95.16 % | 95.03 % | 95.36 % |
| Card play (84,803) | 73.76 % | 73.28 % | 72.68 % |
| All (120,273) | 79.19 % | 79.61 % | – |

Model SHA-256 (not shipped): W = 1 `9e7e3ec8…948ade`, W = 4
`c7927678a16dd6e9151b4bb1c895617405bb39305fb76bc5c45493f596b58e1a`, W = 8
`3efc70aef425aae765be1666e0926135dd8bdd31822ec9089df44a82240ce6c7`.
The teacher agreement is high on the rule-based phases by construction
(Strong's pre-play labels are the Classic rules, see the
[model card](pre-play-model-card.md)); Kontra/Re labels are the extreme D-40
teacher choice and its agreement is no evidence of good Kontra play.

Complete games as above (200 deals × 6 seatings = 1,200 games, validation
deals 10000–10199, Kontra/Re on with the existing Kontra rules; the model
decides Reizen, pickup, Drücken, Ansage and cards). A − B per seat-game:

| Model | vs Classic Easy | vs Classic Strong |
|---|---|---|
| first run (a third of the data) | -34.60 [-40.27, -28.94] | -43.39 [-49.13, -37.65] |
| W = 1 | not run | -18.13 [-22.43, -13.83] |
| W = 4 | -2.10 [-6.40, +2.21] (inconclusive) | -14.99 [-18.92, -11.07] |
| W = 8 | -2.76 [-7.43, +1.91] (inconclusive) | -18.23 [-22.87, -13.58] |

Behaviour of W = 4 against Strong (1,200 games): Reizen actions 5,099, Bid
43.8 %, Hold 32.7 %, Pass 23.6 % (Strong's table 5,033 actions: 43.0 / 33.1 /
23.9 %); Einpassen 5/1,800 (Strong 4/1,800). It declared 599 games, won 70.3 %
at +27.9 points per declarer game (Strong 598 games, 86.6 %, +66.3), and won
17.3 % of 1,196 defender games (Strong 25.8 % of 1,198). Contracts of 599
declarer games: Kreuz 192, Pik 147, Herz 69, Karo 79, Grand 109, Null 3, Hand
0, announcements 0 (Strong 6 Null, 4 Hand of 598). Pass rates and the
contract mix are now close to Strong's; declarer play is still clearly
weaker, and Hand and announcements are never chosen (W = 8 likewise: Null 3,
Hand 0 of 624). Raw logs: `grand-coach-data/studies/2026-11-19-preplay-74b/`.

Caveats: one seed, one dataset, one shape and a 200-deal sample; W = 1, 4 and
8 differ in per-phase accuracy but their game-score intervals overlap, so no
ordering between the weights is claimed, and W = 4 is only the best measured
value. The models stay behind the opt-in tooling: they are weaker than
Classic Strong, only statistically indistinguishable from Classic Easy
(not shown better), unshipped and unchanged as default. Kontra/Re stays with
the existing rules in these games; the learned Kontra/Re head is not used in
play. Outcome-label pilot: still postponed (no model is competitive with
Strong).

#### Rare pre-play classes and the learned Kontra/Re head (#74)

Measured 2026-11-19 on this PC (same data, shape, seed, validation deals
and 1,200-game protocol as above; all models vs Classic Strong; W = 4).
New opt-in `train --all-phases --rare-weight R` multiplies the policy loss of
rows whose teacher choice is rare (Hand instead of the pickup, Null, or a
Schneider/Schwarz/Ouvert announcement) by `R` on top of `--preplay-weight`.
Test agreement per phase (correct/rows) and games:

| Model | Reizen | Hand/pickup | Drücken | Ansage | Kontra/Re | A − B per seat-game | Null / Hand of declarer games |
|---|---:|---:|---:|---:|---:|---|---|
| W = 4 (above) | 96.41 % | 99.68 % | 79.29 % | 91.44 % | 95.03 % | -14.99 [-18.92, -11.07] | 3 / 0 of 599 |
| W = 4, R = 10 | 93.74 % | 98.23 % | 70.27 % | 89.28 % | 94.76 % | -19.14 [-23.41, -14.88] | 5 / 3 of 641 |
| W = 4, R = 40 | 95.67 % | 99.65 % | 76.45 % | 88.57 % | 94.73 % | -20.93 [-25.14, -16.72] | 0 / 0 of 583 |

Strong's tables chose Null 2–6 and Hand 4 times per ~600 declarer games;
nobody announced Schneider, Schwarz or Ouvert (Strong never did in these
games, so that class has no teacher behaviour to imitate here). `R` = 10
makes the learned pre-play choose Hand (3 games) and Null (5) at Strong's
rate, but costs accuracy and strength; `R` = 40 does not help. The
intervals overlap, so no ordering is claimed, and neither model is
better than the W = 4 model without `R`. Not shipped.

Learned Kontra/Re head: `calibrate … nn:W4/model.gcnn@easy+ak strong` lets the
model decide Kontra/Re instead of the rules of thumb (it already worked via
`Profile::search_kontra`; no code change). A − B −19.87 [-24.37, -15.37] against
−14.99 [-18.92, -11.07] with the rules: no gain, the head imitates the
extreme D-40 teacher Kontra, so the shipped rule stays. Raw logs:
`grand-coach-data/studies/2026-11-19-preplay-74c/`. The outcome-label pilot
remains postponed: no imitation model is competitive with Strong.

Outcome-label tooling for Drücken (step 2, not yet run at scale):
`skat_ai::outcome::discard_outcomes` plays each of the 66 legal pairs of a
declarer's Drücken view out in `N` hypothetical games. Opponent cards are
sampled from the view's constraints (`HiddenCards`), the public history is
replayed, and frozen `Player`s finish the game; the result is the mean
Seeger-Fabian score of the acting seat per pair, shared samples across pairs.
These are game outcomes against those opponents, not double-dummy values, so
the D-40 declarer bias does not enter. Tests cover 66 scored pairs,
reproducibility and rejection of other phases and seats. No dataset was
generated and no model was trained or compared with these targets; no
strength claim follows.

`skat_ai::outcome::discard_dataset` labels the declarer's Drücken decisions
of Easy table games and `compare_discards` scores any `Player` against them
(mean label of its pair, mean best label, share within a tie tolerance).
`nn-train outcome-druecken OUT.jsonl [--deals N] [--samples N] [--seed N]
[--model FILE]` writes the rows (permitted view, 66 labels, deal number for
reproduction only) and prints that comparison for Easy, Strong and the model.
Pilot run (`--deals 100 --samples 4 --seed 1`, frozen Easy opponents, 99
rows): Easy and Strong choose the same pairs (mean label 2.73, within 0.5 of
the best label in 44.4 % of rows); the shipped card-play model does not cover
Drücken and falls back to Easy, so it scores identically. The "best" mean of
30.87 is the maximum of 66 four-game means and therefore strongly optimistic
(winner's curse): the 28-point regret is not a measure of lost strength. Use
more samples or re-label the best candidates with fresh seeds before trusting
absolute values. `--holdout-seed N` repeats the sampling with a different seed
and reports the fresh mean for the pair selected as best on the original labels;
it also reports each policy’s chosen-pair mean on the fresh samples. This is
an evaluation against frozen opponents, not a strength measurement. Zero samples
are rejected. `nn-train verify-outcome-druecken FILE` rejects empty files,
duplicate deals, invalid views, missing or duplicate pairs and non-finite labels.
It checks structure only, not provenance, sampling independence or label quality.
No Drücken model was trained against these labels and no strength claim follows.

A baseline on validation deals 10000–10199 (six seatings each, Kontra/Re
on) compared the shipped card-play model, greedy policy with Easy fallback,
against Classic Easy. A − B was +19.04 SF points per seat-game, standard
error 2.04, 95 % interval [+15.04, +23.05]. The model's declarers won
84.8 % of 600 games and defenders won 31.2 % of 1,200 games; Easy's
respective rates were 63.2 % and 20.8 %. Both sides bid in 43.3 %, held in
32.7 % and passed in 24.0 % of 5,004 Reizen actions; neither participated in
an Einpassen. The identical contract mix per 600 declarer games was Kreuz
177, Pik 153, Herz 102, Karo 72, Grand 90 and Null 6, including six Hand
and no Schneider, Schwarz or Ouvert announcements. This is a baseline for
card play only: both sides use Classic for pre-play, and it does not evaluate
an all-phase imitation model. Command: `calibrate validation
nn:assets/models/card-play.gcnn@easy easy 200 4 10000`. Model SHA-256:
`6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75`.

`calibrate` now records the required behavioural counts alongside the existing
role-separated scores. Deterministic rule-engine tests exercise a fully
passive table (six Einpassen games, nine Pass actions and nine thrown-in
seat-games per profile) and twelve scripted completed games covering all
six contract bins, pickup, Hand and Ouvert announcements. They check
seat attribution and merged tallies against the actual calls and results.
These are instrumentation fixtures, not production learned-opponent
calibration or an independent strategic validation of Strong's labels.
No training or arena campaign was run for this reporting increment.

A one-epoch CPU CLI smoke on the two checked-in self-play fixture deals
used two hidden layers of width 16 and all 101 training rows: Reizen 31,
Hand/pickup 2, Drücken 2, Ansage 2, Kontra/Re 4 and plain card play 60.
Reloaded `evaluate` also selected 101 rows. Validation/test had zero rows;
this run establishes plumbing only and supplies no held-out accuracy or
behavioural/strength result. The temporary model was 171,212 bytes,
SHA-256 `536095bd8af78f57f8faffeb94637679993b5ef47c9fb53edf1a4e562fd30140`;
it is not shipped. Separate scripted tests cover the rare Hand/Ouvert/Re
transitions absent from that training fixture.

### Phase-masked learned pre-play (#74)

`NeuralOpponent::with_learned_phases` lets the model decide only a subset of
phases; Classic decides the others and the decision reports
`Fallback::UnsupportedPhase`. `calibrate` takes it as `^<bits>` after the
fallback (bit 0 Reizen, 1 Hand/pickup, 2 Drücken, 3 Ansage, 4 Spiel). Default
is every phase, so existing specs and protocol descriptions are unchanged.

Measured 2026-11-20 with the W = 4 imitation model (SHA-256 `c7927678…8e1a`,
not shipped), `calibrate validation nn:W4/model.gcnn@strong^21 strong 200 4
10000` (1,200 games; Classic Strong decides Hand/pickup and Ansage): A − B
-15.10 [-18.97, -11.23] per seat-game, against -14.99 [-18.92, -11.07] with
the model deciding every phase. Declarers won 70.6 % of 599 games (Strong
86.6 % of 598); Pass 23.6 % of 5,099 Reizen actions (Strong 23.9 %); Hand 5 and
Ouvert 6 of 599 declarer games (Strong 4 and 0). Leaving the rare classes
to Classic therefore does not close the gap: the shortfall lies in Drücken and
card play, not in the missing Hand/announcements. The model is still weaker
than Classic Strong; no strength is claimed, nothing is shipped and #74 stays
open.

For controlled comparisons, `calibrate` also accepts
`'split:nn:PRE.gcnn@strong|nn:SPIEL.gcnn@strong'`: the first profile decides
before Spiel, the second decides Spiel including Kontra/Re. Each retains its
own Classic fallback and the report identifies both models. This enables
separate assessment of existing models; it is not a strength result or a
change to the shipped opponent. No training is authorised by this option.

#### Lower rare-class weight experiment (#74)

On 2026-11-22, the retained PR #225 trainer used the same three datasets,
4×384 shape, seed 0 and six-epoch recipe as above with `--preplay-weight 4
--rare-weight 2`. Epoch 6 had the lowest validation loss. The resulting
unshipped model has SHA-256
`acaf10a4c3e117ea09be03282339c5319aa1ffb9cefdea8a5c93e18dc8bf1d88`.
The current tools at `ba00742` evaluated the model on the existing held-out
splits. Agreement with the teacher was Reizen 22,193/23,034 (96.35 %),
Hand/pickup 2,817/2,827 (99.65 %), Drücken 2,281/2,815 (81.03 %), Ansage
2,618/2,827 (92.61 %), Kontra 1,980/1,987 (99.65 %), Re 1,801/1,980
(90.96 %) and card play 62,881/84,803 (74.15 %). W = 4 without the rare
weight had 79.29 % Drücken agreement on the same test positions. These are
teacher-generated views, not a strength test. Of 2,827 Hand opportunities,
the teacher chose Hand 12 times, the model 10 and both six. Of 2,805 Null
opportunities, the respective counts were two, four and one. There were no
teacher Schneider/Schwarz/Ouvert announcements in 2,805 opportunities; the
model chose three. On 1,987 Kontra views, the teacher chose Kontra 1,980
times, the model 1,984 and both 1,979; on 1,980 Re views the respective
counts were 754, 735 and 655. Rare-class and biased Kontra data limit the
interpretation.

Complete-game calibration used validation deals 10000–10199, six seatings,
Strong and Kontra/Re on (`calibrate validation nn:MODEL@easy strong 200 4
10000`; three of 1,200 games thrown in). With the existing Kontra rule,
A − B was -16.99 points per seat-game, 95 % interval [-20.83, -13.15]. A bid,
held and passed 42.6 %, 33.8 % and 23.6 % of 5,100 Reizen actions; Einpassen
was 5/1,800 seat-games. Of 595 declarer games it chose 184 Kreuz, 154 Pik,
93 Herz, 74 Karo, 87 Grand and three Null, with no Hand or announcements.
It won 70.3 % of declarer and 15.6 % of 1,200 defender games; Strong won
89.2 % of 602 declarer and 24.9 % of 1,194 defender games.

With `@easy+ak`, the learned model also chose Kontra/Re: A − B was -22.09
[-26.72, -17.46], with Kontra 521/1,200 played defender seat-games (43.4 %)
and Re 83/212 declarer games facing Kontra (39.2 %). Strong said Kontra
33/1,194 and Re 128/342. Unoffered and declined Kontra chances are not
separated. These intervals are not a paired difference test between the two
configurations. Both runs use the same deals as the earlier W = 4 study and
are not independent confirmation; no ordering among models is claimed. The
model remains unshipped, the existing rule and Classic fallback stay in place,
and the outcome-label pilot remains postponed. Commands, model and logs are
retained locally in `grand-coach-data/studies/2026-11-22-preplay-74/`.

#### Drücken-specific loss weight (#74)

New opt-in `train --all-phases --discard-weight X` (default 1, omitted from
the recipe then) multiplies the policy loss of Drücken rows only, on top of
`--preplay-weight`. Measured 2026-11-26 on this PC (CPU, nice 10, 1,412 s):
the W = 4, R = 2 recipe above with `--discard-weight 4`, same three datasets,
4×384, six epochs, seed 0 (`best epoch 6`). Unshipped model SHA-256
`b1a2c6365e37c5e5f9de5198e54d7ff970554e3d0bc8c10dae540ed33b33af54`.
Test agreement with the teacher (correct/rows), against W = 4 / R = 2:

| Phase | W = 4, R = 2 | + `--discard-weight 4` |
|---|---:|---:|
| Reizen (23,034) | 96.35 % | 96.10 % |
| Hand/pickup (2,827) | 99.65 % | 99.61 % |
| Drücken (2,815) | 81.03 % | **86.68 %** (2,440) |
| Ansage (2,827) | 92.61 % | 91.48 % |
| Card play (84,803) | 74.15 % | 73.47 % |
| All (120,273) | – | 79.88 % |

Validation Drücken agreement is 87.01 % (2,445/2,810). The joint-pair
agreement thus rises by 5.7 points (+7.1 points over plain W = 4) at the cost
of about 0.3–0.7 points on other phases. Complete games
(`calibrate validation nn:D4/model.gcnn@easy strong 200 4 10000`, 1,200 games,
existing Kontra rule): A − B -18.02 per seat-game, 95 % interval
[-22.29, -13.75], against -16.99 [-20.83, -13.15] for W = 4, R = 2 on the
same deals. The intervals overlap, so no ordering is claimed; the model is
still clearly weaker than Strong. A won 69.1 % of 601 declarer games (Strong
88.6 % of 599) at 24.5 versus 70.1 points per declarer game; Reizen
rates match Strong (Pass 23.7 % vs 23.9 %). Teacher agreement on Drücken
does not translate into strength here, so the remaining gap lies in card
play and in how declared games are played. One seed and one 200-deal sample.
Nothing is shipped; Classic stays the default and #74 stays open. Raw output:
`grand-coach-data/studies/2026-11-26-discard-74/`.

#### Longer training and wider network (#74)

Measured 2026-11-26 on this PC (CPU, nice 10), same three datasets, seed 0
and weights as the Drücken run (W = 4, R = 2, D = 4), checking whether the
remaining gap is under-training or capacity. (1) 4×384 with `--epochs 12`
stops by plateau after epoch 7 (best epoch 6) and exports the identical model
`b1a2c636…3e54`, so the recipe's plateau rule, not the epoch budget, limits
it. (2) 4×512, `--epochs 12 --patience 2 --lr-decays 2` (best epoch 11,
SHA-256 `8f0dc3872f7534ccdc01c23c00b2020846bec7ebe403e5e92bb53b94f65705a5`):
test agreement Reizen 96.49 %, Hand/pickup 99.65 %, Drücken 87.53 %, Ansage
92.50 %, Kontra 99.60 %, Re 91.16 %, card play 74.40 % (63,093/84,803; 73.47 %
at 4×384), all 80.65 %. Complete games (`calibrate validation nn:…@easy strong
200 4 10000`, 1,200 games): A − B -17.09 per seat-game, 95 % interval
[-21.28, -12.90], against -18.02 [-22.29, -13.75] at 4×384; A won 67.9 % of
602 declarer games (Strong 86.1 % of 595) at 20.1 versus 64.5 points per
declarer game, and 18.4 % of defender games (Strong 27.6 %). Kontra/Re rates
match Strong (2.7 % vs 3.0 %). Card-play agreement rises by under one point
and the intervals overlap, so more capacity and training do not close the gap;
it lies in card play and declarer play, which imitation of Strong at this
data size does not reproduce. One seed, one 200-deal sample. Nothing is
shipped; #74 stays open. Raw output: `grand-coach-data/studies/2026-11-26-long-74/`.

#### Higher Drücken weight (#74)

On 2026-11-26, the retained PR #412 trainer ran on this PC (CPU, nice 10,
1,275 s) with the same three Strong datasets, 4×384 shape, seed 0, six epochs,
W = 4 and R = 2 as the D = 4 experiment, but `--discard-weight 8` (best
epoch 6). The unshipped model SHA-256 is
`1bc4f58c9ce1bd9abb5b6a2c82b7634b966392eec47cfe38e64380016ae979f5`.
On the same deal-separated test set, agreement with the teacher was Reizen
21,482/23,034 (93.26 %), Hand/pickup 2,818/2,827 (99.68 %), Drücken
2,362/2,815 (83.91 %), Ansage 2,507/2,827 (88.68 %), Kontra 1,980/1,987
(99.65 %), Re 1,799/1,980 (90.86 %) and card play 61,768/84,803 (72.84 %).
Drücken agreement is lower than D = 4 (86.68 %), as are Reizen and Ansage.
Validation Drücken agreement was 2,338/2,810 (83.20 %). This is one seed,
not evidence that every larger weight is worse. On test views the teacher
chose Kontra 1,980 times and this model all 1,987 times; neither the high
agreement nor the model establishes suitable Kontra behaviour.

In `calibrate validation nn:MODEL@easy strong 200 4 10000` (1,200 games,
existing Kontra rule), A − B was -17.02 points per seat-game, 95 % interval
[-21.50, -12.54], against -18.02 [-22.29, -13.75] for D = 4 on the same
deals. The intervals overlap, so no ordering follows. A won 68.7 % of 594
declarer games versus Strong's 86.7 % of 603, and 18.3 % of 1,201 defender
games versus 26.2 % of 1,193. A passed in 25.0 % of 4,832 Reizen actions
versus Strong's 23.9 % of 5,018; A participated in Einpassen in 5/1,800
seat-games versus 4/1,800. A declared Kreuz 208, Pik 143, Herz 83, Karo 73,
Grand 87 and Null 0, with no Hand or extended announcements in 594 games.
Strong declared the same contracts 180, 150, 102, 72, 93 and 6 times,
including four Hand games. A said Kontra in 27/1,201 defender seat-games
and Re in 7/33 declarer games facing Kontra, using the existing rule.
One seed and one deal sample do not establish general playing strength. The
model remains unshipped; Classic remains the default. Training, evaluation
and calibration logs are in `agent-hq/var/jobs/5069/tmp/discard8/`.

#### Kontra/Re behavioural evidence for W = 4 (#74)

On 2026-11-22, the current `evaluate` and `calibrate` tools at commit
`ba00742` re-evaluated the retained W = 4 model (`c7927678…8e1a`) on the
same three Strong datasets and validation deals 10000–10199. On held-out
teacher views, Kontra agreement was 1,980/1,987 (99.65 %) and Re agreement
1,790/1,980 (90.40 %). The teacher chose Kontra on 1,980 views and the model
on 1,984; both chose it on 1,979 of the 1,987 views where it was legal. The
teacher chose Re on 754 views, the model on 756, and both on 660 of 1,980
available views. These are conditional on teacher-generated positions and
reflect the known extreme Kontra labels, not playing quality.

In `calibrate validation nn:W4/model.gcnn@easy+ak strong 200 4 10000`
(1,200 games, three thrown in), the learned player said Kontra in 525/1,196
played defender seat-games (43.9 %) versus Strong's 35/1,198 (2.9 %). It said
Re in 92/217 declarer games facing Kontra (42.4 %) versus Strong's 135/343
(39.4 %). Declined and unoffered Kontra opportunities are not distinguished.
A − B was -19.87 points per seat-game, 95 % interval [-24.37, -15.37],
reproducing the earlier result on the same deals rather than an independent
strength test. A passed 1,201/5,099 Reizen actions (23.6 %); its 599
declarations comprised 192 Kreuz, 147 Pik, 69 Herz, 79 Karo, 109 Grand and
3 Null, with no Hand or announcements. It won 69.9 % of declarer games and
18.3 % of 1,196 defender games; Strong won 85.6 % of 598 declarer and 26.1 %
of 1,198 defender games. The model remains unshipped and the existing
Kontra rule remains in use. Raw outputs are retained with the study.

All-phase `evaluate` reports teacher agreement for Kontra and Re separately,
including each row count. This distinguishes the two decisions; agreement
alone still does not establish their playing strength. Existing combined
Kontra/Re results above remain the measurements actually performed.

All-phase `evaluate` also reports teacher and greedy-model counts for Pass,
Hand, Null, Schneider/Schwarz/Ouvert announcements, Kontra and Re on the same held-out
views. The report also counts positions where both choices fall in each category;
this need not mean the exact actions match. The available count gives positions
where at least one legal action belongs to the category; exact counts additionally
require the same action on teacher-category positions. It is a per-position
denominator, even when several legal actions match. Categories may overlap.
These counts are conditional on teacher-generated views, not complete-game rates, and do not
establish playing strength.

#### Current Strong teacher pilot (#74)

On 2026-11-22, the generator at `814cee30e1e9` produced 100 Strong
self-play deals (seed 393100, two CPUs): 4,161 records in 602 s, with one
incomplete analysis. Replay and label validation passed without view leaks or
illegal pairs. Of 112 Kontra opportunities, the teacher and the current
Classic rule each chose Kontra six times, always on the same views. This
small sample checks the change in teacher behaviour, not its quality. The
category audit counts one Hand choice in 99 opportunities, no Null or extended
announcements in 98 opportunities each, and one Re in six opportunities. The
sole Hand and Re choices are in the training split; validation and test have
no such choices. The test split has no Kontra choices in 14 opportunities.
These split counts are from replay-verified stored records, not new games.

A six-epoch 4×384 all-phase model trained only on these deals (`W=4`, `R=2`,
seed 0; SHA-256 `0a2c5844069b8900213887951c0b9f98b571d79abb87564bdfc427d22a026a32`)
had 3,335 training and 447 test decisions. On the deal-separated test set,
Drücken agreement was 0/11, Ansage 0/11 and Kontra 4/14; no Re opportunity
was present. There were no teacher Hand, Null or announced Schneider/Schwarz/
Ouvert examples in the 11 available test positions. These tiny denominators
do not establish class performance; the candidate is unsuitable for
integration and no full-game strength is claimed. Classic fallback and the
outcome-label postponement remain. Job 3931 retains the data and model in its
scratch directory.
