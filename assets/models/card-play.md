<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Model card: optional Neural v2 card-play network

The single active `card-play.gcnn` is PPO 4×384 iteration 175 from
`2026-11-10-rl-4x`. Classic remains the default. The
[reviewed operational selection rule and all 18 effect/coverage comparisons](../../docs/neural-operational-selection.md)
are part of this card. This is an exploratory, reversible opt-in replacement;
the historical Phase 2 protocol did not pass.

| Item | Value |
|------|-------|
| File | 5,621,708 bytes, model file format 2 |
| SHA-256 | `6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75` |
| Encoder | v2, 634 permitted-view features |
| Network | Four ReLU hidden layers of width 384; policy 621, value 1, option score 621, option win 621 |
| Phases | Spiel including Re; Kontra follows Classic rules before inference |
| Runtime | CPU policy with legal mask; Strong fallback for missing/broken/unsupported/unusable inference |
| Licence | CC BY-SA 4.0, generated project self-play only |

## Recipe and provenance

`card-play.json` records the actual frozen PPO recipe (trainer
`fdb2109c2f73`), start hash, seed 0, 1,500 games/iteration, batch 512,
two passes, learning rate 0.0001, clip 0.2, losses and mixed opponent pool.
Iteration 175 was the original cross-09 tuning winner (a0175, score 4.977).
The supervised start is `4e1621cd97a001deb44342ea2c6b21f314841b3f59773d9f136c421ed6ca4bd5`,
trained at `55f5916bbce7`, best epoch 28. Its full recipe, data hashes,
coverage and immutable source manifest hashes are retained separately from PPO.
Raw datasets use their ORIGINAL record/encoder schemas, including reused v1
records; training re-encodes permitted views with v2. They are not rewritten
or relabelled as v2 datasets. Raw data and frozen campaign artifacts stay outside git.
Supervised metrics describe the START, not the selected PPO snapshot.

## Evidence and limits

On 1,300 completed paired deals: v2 minus v1 +13.354 SF/seat-game,
nominal 95% interval [+12.005,+14.704]; descriptive six-comparison interval
[+11.541,+15.167]. Against own start: +5.074 [+3.911,+6.237].
Against Strong: -0.023 [-1.052,+1.005] on 1,200 deals, not broad superiority.
Six rotated seat allocations are aggregated within deal. Reused deals across
pairs and role contexts are not independent samples. See the linked decision
for every pair, common-900, early/later and role strata, censoring and methods.

Only whole hash-verified completed blocks count. Seventeen pairs timed out;
prefix censoring can be informative. Exposed-sample intervals have no fresh
post-selection coverage guarantee. Candidate-v1 has only 2 Null, 5 Hand and
1 Ouvert reference games; no competence claim follows. History benefit,
causal PPO benefit and reliable option estimates remain unproven. Pre-play
is shared Classic rules, not learned. Coach behaviour is unchanged.

Existing sampled latency: cold read/hash/load 43.65 ms; card-play p90
0.153–0.188 ms, 30 positions per kind. These are neither exhaustive latency
nor fallback measurements. The runtime equivalence audit and real-weight
regression cover bounded positions, not all possible games.

The [rollback instructions](../../docs/neural-operational-selection.md#rollback)
pin matching v1 weights, manifest and original model card at the immutable
base commit. The historical outcome's original-card reference is available at
the [immutable v1 card](https://github.com/naturelle137/grand-coach/blob/843e3978151df72140afe8fad4733ef54dfc08c9/assets/models/card-play.md).
Restart the cached loader after selecting that directory.
