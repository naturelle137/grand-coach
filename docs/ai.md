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

## Neural table mode

Both opponents use the [bundled card-play model](learned-models.md) with
Classic Strong fallback. Bidding, pickup, discards, announcement and the
default Kontra rule come from Classic. The model can choose cards and Re.

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
interfaces and freezes the conditional budgets. Fresh measurements remain
pending and unauthorised; no default change or #104 closure is justified.

Time Kontra/Re and the first three tricks separately. Solver-only timings
miss the cost of sampling, all legal options and output. Use the
[reproduction procedure](../crates/skat-analysis/experiments/strong-gpu/README.md#early-search-endgames).
