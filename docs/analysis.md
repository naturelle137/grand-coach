# Analysis: `skat-analysis`

The AI and coach use the same option evaluations from `skat-analysis` (D-37, D-38).

## Information boundary

- `analyze(view, budget, seed, cancel)` takes one seat's `PlayerView` and
  nothing else (D-23). It cannot reach the real hidden cards.
- Sampled deals (`World`) and fully specified card-play positions
  (`double_dummy::Position`) are hypothetical simulation inputs.
- The only bridges from a real game to full knowledge are
  `Position::from_omniscient(&OmniscientView)` and
  `World::from_omniscient(&OmniscientView)`, for privileged analysis,
  including Mephisto (D-41) and God Mode (D-62). Their
  names and argument type make every use visible.
- `analyze_known(view, world, budget, cancel)` evaluates every option in
  one given distribution with the specified budget and continuation policy. Its estimates are marked
  `Effort::known` and its assumptions name `KnownDeal` instead of uniform
  hidden cards and bidding evidence; its inferences stay those of the
  seat's own information set (`budget.samples` distributions, seed 0), so
  they say what the seat could deduce without seeing the cards. `analyze_forgetful(view, forgotten,
  budget, seed, cancel)` is `analyze` for a seat that does not remember
  where some played cards went (Goldfish, [ai.md](ai.md#goldfish)).
- Results are plain data (serde behind the feature `serde`): estimates,
  structured observations, inferences and assumptions, no texts (D-13).
- `inferences`: per hidden place (other seat, unknown Skat) the weighted
  mean, minimum and maximum number of trumps (Jacks before the
  announcement; none in Null) over the finished distributions; `certain`
  only when the hard constraints prove the count. The coach shows them as
  estimates ([coach.md](coach.md)).
- `utility(view, estimate)` is the value opponents choose by and the coach
  ranks by: own expected Seeger-Fabian points, for a defender minus half
  the declarer's expected game points.

## Shared deterministic rule path (#131)

`baseline::play` is the original Easy card-play policy, moved from
`skat-ai` without changing selections, Kontra/Re precedence, legal ordering,
min-first/max-last ties or fallback. `baseline::rule` returns its actual card,
branch, predicates, winning/losing groups, candidates and selection key from
one permitted view. Easy and `skat-coach::guided` consume this result; the coach
has no dependency on `skat-ai` and runs no second strategy or search.
`Knowledge::is_master` means master of the follow class only: a side master
can still be trumped. Unsupported branches are not explained as safe tactics. The Null branch and
`HighestPowerLast` are broader than the #165/#169 teaching guard: only a non-Ouvert
human declarer second or last to the current trick, with at least two pure legal cards
and a current loser, gets the dialogue. It uses the existing current loser
set, maximum own printed-suit power and last exact legal-order tie unchanged;
off-suit power never means that the card can win the led suit. The separate
issue #176 opening-lead guard below admits only physical Vorhand and uses
`LowestCostFirst`, never highest-loser shedding.

The guided exception to D-49 exposes action-bearing rule data only after an
attempt or deliberate reveal inside that explicitly entered dialogue. Ordinary
lesson `Evidence` below remains action-free. The bounded guided families
and their core-legal fixtures are specified in [the walkthrough](guided-dialogue.md).
Exact facts come from own hand, known own Skat, public trick cards, seat order
and rules; unknown holders, including unknown Skat, stay unknown. Observations
cite `Source::Trick`; an empty Grand or admitted Null lead uses `Source::Lead` without
inventing a winning card. The bounded `LeadMaster` consumer preserves the
own/played-only master predicate (known discards still block), maximum Augen
and last exact legal-order tie. Both real replies and the individual winner
are observed. Observations use the announced game's follow class (Buben are
Trumpf in Farbspiel). Search estimates remain a separate optional source,
attached to the original decision with their budget and uncertainty.

## Evidence for lessons

`lesson` (D-53) is the contract between the analysis and coach lessons.
It reuses one `Analysis`; it runs no search of its own.

- `gather(view, analysis)` returns `Evidence`: a decision `Snapshot`
  (seat, phase, role, game, bidding calls, tricks, own cards, own Skat,
  an Ouvert declarer's cards seen by a defender) and items, each with its
  class and sources. Observations are `Exact`; inferences are `Sample`,
  whose minimum and maximum are extremes of the evaluated distributions,
  not bounds, and whose positive mean proves no card. Sources point into
  the view: own hand, own Skat, Ouvert cards, the played cards, the trick
  card that proved a void, the bidding call that showed a value, the
  bidding history and rules, or the model for samples. `Evidence` names
  no action and depends only on view, seed and budget; exact items stay
  valid when the search stopped early.
- `compare(view, analysis, chosen)` returns a `Comparison` of the chosen
  option with the best one by `utility` (evaluated on the most
  distributions, ties: first legal). It is `Reliable` only for a complete
  analysis with the chosen option evaluated on every distribution;
  otherwise `Incomplete`, `Screened` or `NotEvaluated`. It also carries
  the exact `mechanisms` of the trick after the chosen card (below); they
  do not depend on the estimates.
- A `Lesson` has a stable key and version, applicability (phases, roles,
  game kinds), permitted evidence classes, required and excluding
  conditions and a support status. `Lesson::check` cites the matching
  items or says what is unmet. Conditions on facts match only `Exact`
  items. A `Fact` lesson may not cite comparisons and holds with an
  incomplete search; a `Mechanism` needs a comparison (after the move)
  that is reliable. Unsupported lessons never apply.
- `catalogue()` holds the two mechanisms of D-56,
  `trumpf-vulnerability` and `partner-overtake`
  ([coach.md](coach.md#card-play-mechanisms-d-56)), then the two exact
  lessons of D-55, `proven-void` and `remaining-trumpf`
  ([coach.md](coach.md#exact-lessons-d-55)). The condition `Recent(kind)`
  holds for facts resting on a card of the trick in progress or the last
  completed trick, so a lesson is not repeated at every card;
  `Mechanism(kind)` holds when the comparison carries a mechanism of that
  kind. Then come the pre-play lessons of D-57, `game-value` and
  `hand-structure` ([coach.md](coach.md#pre-play-lessons-d-57)); the
  condition `OpeningCall` holds at a bidding decision before the seat's
  own first call (`Snapshot::own_calls`).
- `mechanism::mechanisms(view, action)` (D-56) decides two exact
  questions about the trick after the chosen card, from the view only:
  can an opponent playing later trump the seat's winning side Ass or Zehn
  (`Abstich`), and can the partner, playing last, beat the declarer's led
  card (`Overtake`). Each is `Possible` (some hidden distribution within
  the hard constraints of `sampling` meets it) or `Certain` (all do).
  Feasibility is exact without enumeration: hidden cards go to at most
  three places of fixed size, so a distribution exists if and only if,
  for every set of places, the cards allowed only there fit (Hall's
  condition). Conditions are checked jointly; a Trumpf count or a void
  alone proves nothing for a seat.
- `preplay` (D-57) gives the bidder or declarer before card play exact
  observations, from the view and the rules only: per Farbspiel and
  Grand the own Buben, Trumpf and Spitzen (`Structure`), per game type
  the value range of each contract still possible (`GameValues`: with
  pickup, Hand, Null Ouvert, Hand Ouvert) and the value the game must
  reach (`Commitment`: the bid, or the lowest value an own Bid or Hold
  commits to). This is the pre-decision minimum, not the chosen jump bid;
  post-move lessons retain that snapshot and label its temporal context.
  An unknown Skat counts for the Spitzen, so ranges run over
  every pair of unseen cards (at most 231); after pickup the twelve cards,
  and after Drücken the own Skat, fix them. Values leave out Schneider and
  Schwarz made in play; `Reach` says whether a contract reaches the
  value, only with a suitable Skat, only with such levels, or never.
  Nothing here estimates winning or prefers a game.

## Knowledge (card counter)

Exact facts from the view (`Knowledge`): own cards, played cards, Augen
won per seat, trumps out, the own discard (declarer only, after Drücken),
the Ouvert declarer's cards (after the reveal) and voids proven by not
following suit (the whole follow class: Trumpf or one side suit).

Count semantics (D-55): `trumps_out` (observation `TrumpsOut`) counts the
Trumpf neither held nor played, so it includes a Trumpf in the own
discarded Skat and in a revealed Ouvert hand; its meaning is unchanged.
`trumpf_sets` (observation `TrumpfSets`, Farbspiel and Grand only) splits
the Trumpf into disjoint sets: own hand, played, own discarded Skat
(known, in nobody's hand), revealed Ouvert cards and unseen. Unseen
Trumpf lie in a concealed hand or, when the seat does not know the Skat
(defender, Hand declarer), in the Skat; which seat holds them is not
part of the fact.

## Hidden cards (opponent model)

- **Hard constraints** (never violated): every unseen card in exactly one
  place, hand and Skat sizes, voids. The Skat is a place of its own; it
  is known only to a declarer who discarded. While the declarer holds
  twelve cards there is no Skat place.
- **Uniform sampling**: `HiddenCards` counts all consistent distributions
  exactly (dynamic programming over groups of cards with the same allowed
  places) and draws each with probability 1 / count. If there are no more
  distributions than the sample budget, all are enumerated (exact mode).
- **Bidding evidence** (heuristic, fallible): each sample is weighted by
  how well the other seats' bids fit their hypothetical cards under the
  continuation policy: factor 0.25 for a bid or hold above the policy
  limit, 0.5 for a pass below it. Weights are never zero, so evidence
  never excludes a distribution the hard constraints allow; conflicting
  evidence only multiplies down. Every estimate reports the effective
  sample size. Discards are not used as evidence yet.

## Evaluation

For every distribution and every legal option the game is played to the
end:

1. the option is the only enumerated decision,
2. later decisions before card play follow the **continuation policy**
   (`policy`, the baseline opponent's rules, shared with `skat-ai`): bid
   limit from the best game, Hand only with very strong cards, discard,
   announcement, Kontra with five trumps, Re with a strong hand. Each rule
   sees only the deciding seat's own cards in that distribution,
3. card play is **double-dummy**,
4. the result is settled by `skat-core` (`settle`, `seeger_fabian`),
   including Überreizen and Kontra/Re.

Option semantics: `Bid(v)` means "say `v` now and go on up to `v`";
`Hold` means "hold and go on up to the policy limit or the current bid";
Kontra and Re in play use the position before the next card. Later Kontra
by the other defender follows the policy.

**Double-dummy**: the declarer maximises, the defenders minimise the
declarer's game points. For a fixed contract these depend only on the
outcome class (no trick, ≤ 30, 31–60, 61–89, ≥ 90 Augen, every trick) and
never decrease from class to class, so the solver finds the class with
null-window alpha-beta searches (61, then 90 or 31), a forced-all-tricks
search and a forced-no-trick search, independent of the contract. Null:
can the declarer avoid every trick. A transposition table, equivalent
cards and a sure-tricks bound keep it fast. Tests compare it with a plain
exhaustive minimax over `settle` on small positions.

The Augen kernel encodes each hand as a 32-bit mask with each follow class
in descending strength order. It carries the current trick winner, Augen on
the table and remaining Augen into child positions. Classic visits the same
moves in the same order and charges the same node budget. Experimental can
also reuse an adjacent card's proven bound when no outside or table card
separates the two ranks. Swapping those cards preserves every trick winner
and changes the declarer's total by at most their point difference.

## Results

Every option of `PlayerView::legal_actions` gets an `Estimate`, from the
deciding seat's point of view:

| Field | Meaning |
|-------|---------|
| `win_chance` | probability that the seat's party wins the game (thrown in: not won) |
| `expected_score` | expected Seeger-Fabian points of the seat for this game (0 if thrown in) |
| `expected_game_points` | expected declarer list score (`GameResult::score`, negative if lost, with Kontra/Re) |
| `thrown_in` | probability that all pass |
| `score_std_error`, `samples`, `exact` | uncertainty; `exact` means every distribution was enumerated |

Augen are not estimated; the solver only resolves the scoring classes.
Kontra changes only the declarer's points: a defender's Seeger-Fabian
points (40 if the declarer loses) do not depend on it, so compare
`expected_game_points` for Kontra.

`Status` is `Complete`, `BudgetExhausted` or `Cancelled` (estimates only
over finished distributions; a distribution with an interrupted task is
dropped for every option), or `NoDecision`.

**Drücken** has 66 options, each its own deal to solve: every option is
screened on 3 distributions, the best 6 are evaluated on all. `samples`
shows which options had fewer.

## Hybrid proposals (experimental, #76)

`hybrid::analyze_hybrid` is an opt-in experimental backend and engine choice. From the original permitted `PlayerView`, the loaded model
proposes its two highest policy probabilities (stable legal-order ties).
The existing shared rule card is included, giving two or three candidates.
The ordinary `analyze` evaluates these in original legal order using the
unchanged world/node budget and seeded sampling. The learned value is
reported separately and never enters solver bounds or settlement.

`HybridAnalysis::LearnedEstimate` carries the actual model SHA-256, header
encoder version, learned seat value and every original legal action. Each
action has a prior, explicit proposal membership and optional ordinary
Monte-Carlo verification. An unsearched action has no verification, never
zero utility or an inferior rank. The search completion status and effort
refer only to proposals. This distinct type cannot enter `lesson::compare`
or any existing `Analysis` reliability predicate; there is deliberately no
conversion to a complete analysis. The recommendation compares only finished
verified proposals at the greatest sample count, by existing `utility`,
keeping the first original legal-order tie. No finished verification means
no recommendation, even if the model supplied a value or prior.

All enumerated distributions remain ordinary all-option Classic, including
cheap exact late games. At most three legal cards, decisions outside pure
card play (including Kontra/Re), and missing/broken/unsupported/non-finite
models also keep Classic. Cancellation before inference skips the model;
interrupted proposal verification reports its actual status without invented
estimates. No filesystem/network IO occurs here. `skat-ai::HybridOpponent`
provides a thin `Player` adapter with the selected Classic level for model
fallback and legal rule fallback when verification cannot finish. No coach strategy is duplicated.

The [paired quality comparison](ai.md#hybrid-neural-and-strong-comparison)
did not establish a strength ordering. Classic remains the default; no
training, weights or model provenance were added by that measurement.

## Two-stage card play (experimental, #104)

`analyze_two_stage` is an opt-in prototype. No shipped level, coach or
generator calls it; `analyze` and `analyze_experimental` stay the
reference (`Kernel` picks which one to match). `Opponent::decide_two_stage`
in `skat-ai` exposes it for paired measurements only.

1. **Stage 1:** every legal card on the first `screen_worlds` distributions
   of the single-stage analysis, the same worlds in the same order.
2. **Screening:** best = highest weighted mean `utility`; ties go to the
   lowest option index. Another option survives if its gap to the best is
   at most `se_multiple` × the standard error of the paired
   per-distribution difference. That standard error uses the evidence
   weights' effective sample size and is raised to `se_floor`. Survivors
   are filled up to `min_keep` and cut to `max_keep` in rank order (mean
   descending, index ascending).
3. **Stage 2:** survivors on the remaining distributions. A single survivor
   (only with `min_keep` 1) skips stage 2.

Survivors get bitwise the single-stage estimates. Screened-out options keep
their stage-1 estimate with fewer `samples`. `StageReport` gives the
outcome, survivors, stage-1 nodes, and per option `kept` and the effective
sample size. `Opponent::decide_two_stage` chooses only among the survivors
(with a single survivor, that option). Same view, seed, budget, kernel and parameters give the same
result for any thread count.

No screening, result identical to the reference, with `SkipReason`: not
this seat's turn, not card play, Kontra/Re offered, enumerated (exact),
at most `max(2, min_keep)` options, at most `screen_worlds` distributions
(so Medium's 6 samples always skip under the proposal), an interrupted
stage-1 task (all options continue), or a stage-1 effective sample size
below 2.

The frozen proposal is 24 worlds with screening `(8, 2.5, 2, 2, 4)`.
Medium's 6 worlds are not enabled. No parameter tuning or default change
is authorised by the diagnostic below.

### Actual bounded tuning diagnostic (HQ520, audited by HQ521)

The [source audit and actual result](https://github.com/naturelle137/grand-coach/issues/104#issuecomment-6826724749)
cover three timing repeats of the same 40 role/trick positions from only
five underlying tuning deals (10001–10005), five positions per bucket.
Repeats are not independent observations. The measured source is
`6246cb07f7a728e5f902697dd3a051221540255a`, before the single OneSurvivor
fix `15f098c3a4733162b20e0bfffc1e69dc24f99dac`; these measurements do not
validate that final head or merged PR #120.

| Subset (each repeat) | Identical actions | Reference nodes | Candidate nodes |
|---|---:|---:|---:|
| All positions | 39/40 (97.5%) | 343242621 | 252010059 |
| Plain card play (report present, excluding Kontra/Re) | 34/35 (97.14%) | 261865780 | 170633218 |
| Actually screened | 24/25 (96%) | 246908764 | 155676202 |

Per repeat: Screened 25, AllSurvived 5, Skipped(FewOptions) 5,
Skipped(KontraRe) 3, absent report 2. No OneSurvivor, Exact, SmallBudget,
Interrupted, LowEffectiveSamples, NoDecision or NotCardPlay reports were
observed; absence is not test coverage. The one differing action is seed
10004, trick 1, defender; mean and maximum reference-utility regret are
9.544 points in every repeat. Equal actions do not establish equal estimates
or teacher labels.

| Run | Summed wall ms (reference → candidate) | Median ms | p90 ms | Maximum ms | Summed ratio |
|---|---:|---:|---:|---:|---:|
| 1 | 4214.5 → 3427.7 | 22.90 → 17.40 | 326.7 → 215.2 | 1120.1 → 1111.7 | 0.813311 |
| 2 | 3711.9 → 3427.0 | 23.10 → 15.60 | 237.0 → 179.8 | 1035.0 → 1285.3 | 0.923247 |
| 3 | 3637.2 → 3059.6 | 20.85 → 15.45 | 250.0 → 185.6 | 1032.7 → 999.3 | 0.841197 |

Median per decision uses the middle pair; p90 uses round((n−1) × 0.9).
Across summed ratios, median = 0.841197 and p90/max = 0.923247. The
predeclared overall diagnostic threshold passed (identity ≥97%, median
ratio ≤0.90, p90 ≤1.05, three deterministic runs). The screened subset
remains below 97%. Reference always ran first and starting load was 6.09:
this is not a certified speedup. Status, fallback, completion, individual
actions and labels were absent from the TSV. No fresh holdout, paired-tool
throughput, label agreement or strength interval was measured.

Source archive SHA-256:
`bfde1fae677d3730d36cf5a6b657a468ac311974b8736623adc885949a1dfe60`;
binary SHA-256:
`637bfe013b2d2bd688ca376127125299679582c7ec484cd1f4f4ae02545e47b3`.
The audit checked 509 source files. Raw TSV SHA-256, runs 1–3:

- `e9d624215c045420f281f3c842f3b2ab693ff55f0a0ccd5fe6338d8ba41481e9`
- `b76ee65dbfaaf3e199c9aff85d2bfd3b89cdae18ae49bec7d803aeb0ae33d2d8`
- `cf190eda966f27222dbd70d63190d73f912eeaca8011759ae48b46476b8b0801`

The earlier observation/tool no-go and frozen budgets are recorded in
[issue104-observation-no-go.md](issue104-observation-no-go.md).

### Paired holdout measurement (job 2124)

The `two_stage` example now alternates the order deterministically
(reference first iff `(position + repeat)` is even), takes
`--repeats=N`, and prints seat, order, analysis status, fallback flag,
chosen actions and every option's utility and sample count for both paths;
decisions must be identical in every repeat. Rules-only decisions show
`unavailable`, never `Complete`. The optional `two-stage` argument of `skat-ai --example latency` and
`--two-stage --repeat=N` of `skat-analysis --example bench` now emit paired
same-input analyses and alternate order by position and repeat. Their raw
rows include full estimates, effort and screening reports. The paired
`two_stage` example also emits complete estimates and effort. `nn-train --example paired_labels` compares the real reference teacher
record and candidate action, teacher options and analysis on the same
reference-generated view, as diagnostic JSONL. It does not create training
shards or measure a candidate trajectory. The `two-stage` calibration profile dispatches the same experimental opponent.
Generator throughput remains unpaired (see the no-go document).

Fresh holdout, once, no tuning: `two_stage holdout 50 24 --repeats=3`,
proposal (8, 2.5, 2, 2, 4), 24 samples, seeds from 2,000,001, 400
distinct role/trick positions (tricks 1–4), pinned to four CPUs. The
machine was not idle (load average about 31 at the end), so times are
indicative only.

- Identical action: **361/400 = 90.25 %** (target ≥97 % **missed**).
  The retained summary reports 39 comparable differences, mean regret
  2.400 and maximum 25.375 (mean over all 400: 0.234). The raw TSV was
  not retained, so the individual regrets and their breakdown cannot be
  independently audited.
  Identity by bucket (trick, role): 44–48 of 50, no bucket above 96 %.
- Outcomes: Screened 246, Skipped(FewOptions) 105, AllSurvived 27,
  Skipped(KontraRe) 19. All analyses `Complete`, no fallback.
- Nodes (three repeats): reference 12,407,924,919, two-stage
  8,809,132,689 (0.71).
- Summed wall-time ratio two-stage/reference per repeat: 0.854, 0.896,
  0.924. Per-decision median 33.6 → 29.0, 30.3 → 28.2, 28.0 → 26.3 ms;
  p90 535.9 → 462.3, 504.7 → 435.1, 361.2 → 374.6 ms.

Result: the frozen proposal saves roughly 8–15 % wall time (29 % nodes)
but misses the 97 % decision-equivalence target on fresh positions: a
**measured no-go for the proposal as a default**; the tuning-suite result
(39/40) did not carry over. Defaults stay single-stage. Strength,
generator throughput and label agreement were not measured. Retuning
needs the tuning suite only, with a new deliberate decision.

### Bounded tuning and tool diagnostic (job 2386)

On the opt-in proposal, 25 tuning deals (seeds 10001–10025) supplied
200 distinct role/trick positions, repeated three times on four CPUs. All
repeats agreed on actions: 187/200 (93.5%) matched the reference; the 13
differences had mean reference regret 3.828 and maximum 11.846 points.
The proposal missed the 97% target on this tuning sample. Across repeats,
reference/candidate nodes were 7,437,041,394 / 5,332,202,643. Outcomes per
position: 122 screened, 18 all survived, 46 few options, 12 Kontra/Re and
2 without a report. The reference trajectory's one-deal teacher diagnostic
compared 37 records: 37 actions and 31 complete teacher records matched.
This small sample does not establish dataset-wide label agreement.

Three paired tool runs used three positions per kind. Pooling only card-play
positions, reference → candidate median / p90 / maximum in milliseconds:

| Run | Latency (15 positions) | Analysis bench (9 positions) |
|---|---|---|
| 1 | 13.2 / 1033.8 / 2474.2 → 7.0 / 848.3 / 1855.0 | 6.1 / 203.2 / 1781.4 → 4.7 / 120.5 / 1860.1 |
| 2 | 6.1 / 635.4 / 1481.5 → 6.5 / 507.0 / 1148.9 | 2.9 / 191.3 / 1177.7 → 2.9 / 112.3 / 1240.9 |
| 3 | 7.5 / 2455.6 / 2494.6 → 7.1 / 2369.2 / 3071.2 | 3.4 / 197.1 / 1324.1 → 2.9 / 123.9 / 1470.5 |

Median averages the middle pair; p90 selects round((n−1) × 0.9). Host
activity was uncontrolled (load around 10–25); these tiny samples establish
no general speedup. The tuning TSV SHA-256 is
`9c1fb7c5e255eb43267c8e8ee3df6f741ba23b7f36879647caaefcb3056332ce`;
the teacher JSONL SHA-256 is
`0bb6cb44803fe71a7444a6d145c5a1852a03551ff1dc86b3d3df490d3c1a27a9`.
No paired generator throughput or strength interval was measured. Defaults
and existing datasets remain unchanged.

### Conclusion for #104: no-go for the proposal

Both position sets miss the 97 % decision-equivalence target: 93.5 % on
the 200 tuning positions and 90.25 % on the fresh 400-position holdout.
The saving is real but small (29 % fewer nodes, 8–15 % wall time on the
holdout; tiny, noisy tool samples agree in direction). Screening with
8 of 24 worlds therefore changes too many decisions for Strong, the coach
or the teacher. Two-stage sampling stays an opt-in measurement tool; the
single-stage path remains the only shipped path, so golden decisions,
existing datasets and Strong's strength are unchanged. Paired generator
throughput, label agreement over a full dataset and a `calibrate` interval
were not measured, because a candidate that misses the equivalence gate is
not adopted. A later attempt needs a different mechanism (for example
stage 1 on more worlds or root grouping, which is identical by
construction) tuned on the tuning suite only, with a new decision.

## Budgets and determinism

- `Budget { samples, nodes_per_task }`: a task evaluates every option on
  one distribution (Drücken: one option). Work is counted in search nodes,
  never time. Each task gets a fresh solver, so results and node counts do
  not depend on thread scheduling: same view, seed and budget give the
  same `Analysis`.
- Tasks run on all cores (`std::thread::scope`); results are combined in
  task order. `with_max_threads` limits the threads of analyses started
  on the calling thread (tools running games side by side, D-11); the
  result does not depend on the thread count.
- `Cancel` stops an analysis from outside; `Cancel::with_deadline` adds a
  time limit for interactive use (then only the status is reproducible).
- `Budget::INTERACTIVE` = 24 samples, 100,000,000 nodes per task.
- Historical replay comments (D-67) seed `analyze` from the permitted view
  and a fixed constant only, never from recorded or concealed data.

## Latency target and benchmark

Target with `Budget::INTERACTIVE`, release build, on the reference
hardware (Intel Core i9-10900K, 10 cores / 20 threads, 32 GB, Fedora 44
Distrobox): every benchmark analysis complete, and

| Decision | Median | p90 |
|----------|--------|-----|
| card play, Kontra/Re | ≤ 1 s | ≤ 3 s |
| Reizen, Hand/pickup, announcement | ≤ 3 s | ≤ 5 s |
| Drücken | ≤ 8 s | ≤ 10 s |

Benchmark positions: seeded games 1, 2, … (Kontra/Re on) in which the
policy decides before card play and the first legal card is played; the
first 10 positions of each kind: an opening bidding decision (first or
second call), Hand or pickup, Drücken, announcement, Kontra decision (the
benchmark games never say Kontra, so no Re decision), card play at tricks
1, 4 and 8.

```sh
cargo run --locked --release -p skat-analysis --example bench
```

At the interactive budget, pre-play estimates can have large standard
errors. Inspect sample counts, screening and uncertainty before comparing
close options. The analysis tests compare Monte Carlo with exact
enumeration on known positions.

### Hybrid latency comparison

On 2026-11-18 the release `skat-ai` latency example ran ten seeded positions
per kind on the same i9-10900K/Fedora 44 Distrobox as the
[quality comparison](ai.md#hybrid-neural-and-strong-comparison), using the
same frozen model and Strong fallback. It measures decisions sequentially;
other host activity was not controlled. Median / p90 are milliseconds:

| Decision | Strong | Hybrid | Neural |
|---|---:|---:|---:|
| Kontra | 89 / 620 | 90 / 613 | 0.138 / 0.187 |
| Card play, trick 1 | 258 / 491 | 150 / 194 | 0.135 / 0.142 |
| Card play, trick 2 | 30 / 180 | 20 / 95 | 0.134 / 0.139 |
| Card play, trick 3 | 7.267 / 11 | 5.825 / 9.089 | 0.134 / 0.140 |
| Card play, trick 4 | 3.104 / 6.287 | 5.063 / 6.379 | 0.142 / 0.145 |
| Card play, trick 8 | 1.075 / 1.550 | 1.955 / 2.754 | 0.134 / 0.140 |

All three reported zero incomplete decisions in every phase; model read,
hash and load took 42.7 ms separately. Pre-play uses the Classic fallback
and was below 0.01 ms median and p90 for each player in this sample. Ten
positions are descriptive and do not establish a general speedup. Neural's
Kontra path differs from Hybrid's Classic fallback; late exact positions can
also follow different paths.

```sh
cargo run --locked --release -p skat-ai --example latency -- 10 nn:assets/models/card-play.gcnn
```

## Exact retrieval counts (#48)

`counting::retrieval_questions` restricts the existing M6 quiz to the exact
`remaining-trumpf` and `proven-void` families. `derive` counts Trumpf outside
or other seats with public failure-to-follow evidence for one class. Partial
constraints, concealed distributions and model outputs never prove a void.
An empty side class (the trump suit in Farbspiel) is inapplicable. Null has
no Trumpf question; absent public void evidence it offers none. App grading
and explicit help use one frozen `Knowledge`, not a later analysis result.
Legal replay fixtures in `tests/retrieval.rs` cover both roles, positive,
boundary and counterexample cases; `tests/counting_fixtures.rs` additionally
covers known discards, Hand and Ouvert. Hidden-card substitution checks both
baseline and focused questions and their derivations.

## Null opening lead (#176)

The [bounded opening-lead dialogue](guided-dialogue.md#null-opening-lead-176)
uses the unchanged shared Null minimum-cost rule, FIRST exact tie. Only the
non-Ouvert Vorhand declarer's own first lead is admitted; Augen are legacy
sorting cost, not Null scoring. All legal leads remain available. Empty-table
candidate status never predicts either defender reply or completed winner.

Default-feature `cargo test -p skat-analysis --test null_lead_policy` checks
accepted1695's complete literal inputs/all eleven fields and all 286 first
tricks (94 immediate Null losses); `skat-coach --test guided_null_lead` and
app tests matching `null_lead` cover full non-absent lessons/dialogues, both
hidden substitutions including God Mode, legal alternatives, actual two replies,
individual winner, consent, stale/reentry/replay/reset and original estimates.
Run `cargo test -p gui-egui --test null_lead -- --include-ignored --nocapture`
in skat-dev for ordinary widgets and labelled component fixtures in both
languages/sizes. `SKAT_NULL_LEAD_WIDGET_FIXTURES_OUT` intentionally regenerates
the component JSON from checked app sessions; ordinary renders never inject it.
Source-derived `null-lead-inventory.json` joins the existing both-OS gallery.
Push gate and the configured workspace non-GPU/include-ignored suite remain
required, including nn-train and vulkan-search. Terminal owns review and final
actual-main/both-OS archive/hash/visual acceptance. No learner results follow.

### Farbspiel early-winner dialogue (#178)

The coach now admits the shared `WinEarly` branch only for a non-Ouvert
Farbspiel current-second-seat view with one contiguous public card, correct
turn/leader, at least two exact ordered core-legal Play actions and consistent
Hand/pickup knowledge. Shared policy and AI choices remain unchanged. Every
current winner is a candidate; minimum `(is_trump, power)` keeps the first
exact tie. The selected winner itself must be Trumpf or a conservative
follow-class master. Another own master cannot qualify a cheaper non-master.
All Buben and the announced suit are Trumpf. A side master remains vulnerable
to trumping, a lower Trumpf to overtrumping. No unseen holder is assigned.

The [walkthrough](guided-dialogue.md#farbspiel-winearly-178) separates accepted
physical oracle facts from disclosed source-informed numeric/API calibration.
All356 represented next-trick completions and four late terminal paths are
checked; numeric final settlement and learning efficacy are not inferred.
The complete guided model, including separately requested estimates, uses
the original permitted view even in God Mode. Privileged cached advice is
excluded from this family; ordinary God Mode advice keeps its own provenance.

### Farbspiel last-seat feeding dialogue (#185)

The unchanged shared `FeedPartner` rule considers every legal card, maximizes
`(!is_trump, printed Augen)` and retains the last exact core-order tie, without
power. The bounded coach admission is a valid non-Ouvert Farbspiel last-seat
defender, partner currently ahead, exact ordered pure legal list of at least
two and consistent Hand/pickup. All Buben and the announced suit are Trumpf.
A selected card may overtake the partner; only the actual individual winner
is observed. No strategic optimum, game guarantee or concealed holder follows.
The [walkthrough](guided-dialogue.md#farbspiel-last-seat-feedpartner-185) separates
the accepted independent physical predictions from source-informed API metadata.
The cloud-owned default-feature policy target covers all 37 represented legal
completions, four terminal paths and two full-view-equal hidden pairs.

The [second-seat Grand feeding dialogue](guided-dialogue.md#grand-second-seat-feedpartner-131)
uses unchanged `baseline::rule`: `FeedPartner`, `partner_ahead`, `table_master`,
not `last`; all legal candidates, maximum nontrump/Augen and last exact tie.
A table master excludes higher cards outside own/played sets in its follow
class only. It predicts neither the remaining declarer's reply nor the winner.
The original #163 core witness covers every represented reply; the coach/app
check complete permitted-view equality and actual observations separately.
