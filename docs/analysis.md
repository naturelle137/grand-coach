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
can still be trumped. Unsupported branches are not explained as safe tactics.

The guided exception to D-49 exposes action-bearing rule data only after an
attempt or deliberate reveal inside that explicitly entered dialogue. Ordinary
lesson `Evidence` below remains action-free. The two bounded guided families
and their core-legal fixtures are specified in [the walkthrough](guided-dialogue.md).
Exact facts come from own hand, known own Skat, public trick cards, seat order
and rules; unknown holders, including unknown Skat, stay unknown. Observations
cite `Source::Trick` and use the announced game's follow class (Buben are
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

The allowed observation/tool increment has an explicit
[unsupported-mode no-go and frozen conditional budgets](issue104-observation-no-go.md).
The existing `two_stage` example remains a diagnostic TSV tool, not an
admitted holdout runner. Issue #104 stays open with all unsatisfied criteria
unticked; no further compute campaign is authorised.

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
