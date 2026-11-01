# Analysis: `skat-analysis`

How the shared analysis of M3 evaluates decisions (D-37, D-38). AI (M4)
and coach (M5) use it; neither computes strategy of its own. Details are
in the code (`crates/skat-analysis`).

## Information boundary

- `analyze(view, budget, seed, cancel)` takes one seat's `PlayerView` and
  nothing else (D-23). It cannot reach the real hidden cards.
- Sampled deals (`World`) and fully specified card-play positions
  (`double_dummy::Position`) are hypothetical simulation inputs.
- The only bridges from a real game to full knowledge are
  `Position::from_omniscient(&OmniscientView)` and
  `World::from_omniscient(&OmniscientView)`, for open mode, post-game
  analysis (M7), the Mephisto opponent (D-41) and God Mode (D-62). Their
  names and argument type make every use visible.
- `analyze_known(view, world, budget, cancel)` evaluates every option in
  one given distribution, exactly. Its estimates are marked
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
  own first call (`Snapshot::own_calls`). Contract version 4: the
  observations `Structure`, `GameValues` and `Commitment` and
  `Snapshot::own_calls` (version 3: `Comparison::mechanisms`; version 2:
  `Source::Trick` names the led card and the card played).
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
  commits to). An unknown Skat counts for the Spitzen, so ranges run over
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

Achieved on 2026-10-29 (10 positions each; `se`: median standard error of
the best option's expected score; `regret`: mean Seeger-Fabian points lost,
as valued by one run, when choosing the best option of a second run with
another seed):

| Decision | Median | p90 | Max | Incomplete | se | regret |
|----------|--------|-----|-----|------------|----|--------|
| Reizen | 0.9 s | 4 s | 4 s | 0 | 4.1 | 6.3 |
| Hand/pickup | 0.9 s | 1 s | 4 s | 0 | 19.4 | 0.0 |
| Drücken | 6 s | 8 s | 10 s | 0 | 17.0 | 15.0 |
| Announcement | 3 s | 3 s | 5 s | 0 | 19.5 | 2.2 |
| Kontra | 0.3 s | 1 s | 3 s | 0 | 2.7 | 0.0 |
| Card play, trick 1 | 0.7 s | 1 s | 3 s | 0 | 3.7 | 1.5 |
| Card play, trick 4 | 5 ms | 17 ms | 37 ms | 0 | 3.9 | 0.2 |
| Card play, trick 8 | 1 ms | 2 ms | 2 ms | 0 | 0.0 | 0.3 |

All rows meet the target. Quality: card play and Kontra/Re estimates are tight;
before card play the standard errors (17–20 points) and the Drücken
regret show that 24 samples separate only clearly different options.
More samples or a faster solver improve this; see the open points in
`decisions.md`. Monte Carlo against exact enumeration is tested in
`tests/analysis.rs` (tolerance: 4 standard errors plus 1 point).
