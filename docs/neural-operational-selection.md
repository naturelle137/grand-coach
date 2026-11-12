<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Optional Neural operational selection (CEO274)

2026-11-12. This PR proposes replacing the optional Neural table's single active
file with frozen PPO v2 4×384 iteration 175, subject to independent review of
this revised rule AND the actual assets before protected merge. Classic remains
the default. This decision supersedes retain-v1 for this narrow deployment only;
the [historical Phase 2 no-go](phase2-outcome.md) and
[predeclaration](phase2-evaluation.md) remain intact.

## Revised operational rule

Use the original tuning winner and smallest eligible shape, without further
checkpoint search. Require all completed candidate-versus-v1/start prefixes to
have descriptive six-comparison lower bounds above +1 SF/seat-game and role
lower bounds above -3, with consistent common-900 and early/later checks.
These exposed-data thresholds are operational safeguards, NOT fresh acceptance
or error guarantees. All three candidates meet these observed safeguards;
4×896 remains evaluation-only under unchanged D-63. Larger shapes have no
established advantage sufficient to justify their additional size/latency.
The selected PPO checkpoint also exceeds its own supervised start on observed
pairs; that supports this file choice, not a causal claim of PPO efficacy.

We explicitly accept administrative and potentially informative censoring,
and sparse Null/Hand/Ouvert evidence, for a reversible opt-in model. No old
experiment is relabelled passed. History use, PPO efficacy, option quality and
broad superiority over Strong remain unproven. No unrelated #75/#93 criteria
are closed. No new training, arenas or changed default are authorised.

## Analysis method and integrity

The committed read-only helper `scripts/phase2/operational_selection.py` reproduces the
statistics from the frozen evaluation directory. Only whole marker-complete 100-deal blocks are included; rechecked every declared TSV/calibrate stdout/stderr SHA256, all seven model hashes/sizes, contiguous prefixes for all 18 pairs. The existing HQ406 audit supplies coverage and partial-tail inventories. n is underlying deals per pair; six allocations/three contexts and reused deals across pairs are never pooled as independent samples. All 18 pairs have a common 900-deal prefix; this is a sensitivity check, not a new independent sample.

Primary is native calibrate paired A-minus-B SF/seat-game across six seat allocations,
including majority/minority roles. Its rounded per-deal values get nominal normal 95%
intervals widened by 0.01. Six-primary simultaneous z = 2.638 intervals are retained
descriptively. Role runner pairs contexts 0/3 and 1/4 as declarer, 2/5 as defender
(candidate column follows odd/even defender swap); aggregate correlated contexts within
deal before SE. Exclude contract/declarer mismatch or error; thrown-in reference deals
have no role TSV rows. All completed blocks have zero mismatched/error context
exclusions; thrown-in counts are not called missing or failures.

| Pair A--B | Deals | Missing of 2,000 | SF delta | Nominal 95% interval | Common 900 delta |
|---|---:|---:|---:|---|---:|
| 4x384--4x384-start | 1300 | 700 | +5.074 | [+3.911, +6.237] | +5.255 |
| 4x384--4x512 | 1300 | 700 | +0.386 | [-0.790, +1.563] | -0.073 |
| 4x384--4x896 | 1400 | 600 | -0.956 | [-2.021, +0.109] | -0.769 |
| 4x384--easy | 1200 | 800 | +16.696 | [+15.064, +18.328] | +16.826 |
| 4x384--medium | 900 | 1100 | +4.833 | [+3.541, +6.125] | +4.833 |
| 4x384--strong | 1200 | 800 | -0.023 | [-1.052, +1.005] | -0.351 |
| 4x384--v1 | 1300 | 700 | +13.354 | [+12.005, +14.704] | +13.524 |
| 4x512--4x512-start | 1200 | 800 | +4.753 | [+3.507, +5.999] | +4.406 |
| 4x512--4x896 | 1900 | 100 | -0.638 | [-1.521, +0.245] | -0.247 |
| 4x512--easy | 1100 | 900 | +16.385 | [+14.653, +18.117] | +16.274 |
| 4x512--medium | 1000 | 1000 | +4.338 | [+3.092, +5.584] | +4.188 |
| 4x512--strong | 1300 | 700 | -0.047 | [-1.074, +0.979] | -0.054 |
| 4x512--v1 | 1000 | 1000 | +12.060 | [+10.549, +13.570] | +12.649 |
| 4x896--4x896-start | 1400 | 600 | +4.645 | [+3.471, +5.820] | +4.592 |
| 4x896--easy | 1300 | 700 | +17.122 | [+15.539, +18.705] | +16.915 |
| 4x896--medium | 1200 | 800 | +5.315 | [+4.145, +6.486] | +5.307 |
| 4x896--strong | 2000 | 0 | +0.889 | [+0.080, +1.698] | +1.014 |
| 4x896--v1 | 1200 | 800 | +12.800 | [+11.408, +14.193] | +13.005 |

## Candidate-specific safeguards

4x384--v1 has 1,300 independent deals. Descriptive six-comparison interval
[+11.541,+15.167], versus own SL start [+3.513,+6.636]. Candidate-v1 common 900 mean
+13.524, nominal interval [+11.931,+15.118]. Early 600 mean +13.050 [11.065,15.034],
later 700 +13.615 [11.781,15.450]; start margins also consistent (+4.901 early,+5.223
late). No demonstrated drift sufficient to explain the large v1 margin.

Candidate-v1 role sample 1,294 deals (six reference thrown-ins omitted), zero
mismatched/excluded contexts. Declarer paired delta +25.200 [21.506,28.894]; defender
+1.236 [0.538,1.935]. Thus neither observed role regresses under the old -3 SF guard.
Candidate-start declarer +9.839 (lower 6.79), defender +0.71 (lower 0.09). Full exact
values and all other pairs are reproducible with the committed helper.

Reference declarer seat strata for candidate-v1: seat 0 n = 437, seat 1 n = 431, seat 2
n = 426; their paired overall intervals all positive. These seat labels are reference
declarer seats, NOT six rotated role-runner seats. Actual absolute rotation is
calibrate. Contract sample for candidate-v1: Kreuz 369, Karo 181, Herz 225, Pik 319,
Grand 193; Hand only 5, Null only 2 (one Ouvert), Schwarz/Schneider announced absent. No
competence claim for sparse/absent contracts. Both models only replace supported
card-play policy; pre-play follows Classic rules.

| Candidate vs comparator | Descriptive six-comparison lower | Declarer lower | Defender lower |
|---|---:|---:|---:|
| 4x384--4x384-start | +3.513 | +6.786 | +0.091 |
| 4x384--v1 | +11.541 | +21.506 | +0.538 |
| 4x512--4x512-start | +3.079 | +5.924 | +0.617 |
| 4x512--v1 | +10.030 | +18.837 | +1.113 |
| 4x896--4x896-start | +3.068 | +5.068 | +0.141 |
| 4x896--v1 | +10.929 | +20.093 | +0.405 |

## Timeout/censoring assessment

Old matrix 232/360 chunks = 23,200 pair-deals, not 23,200 distinct deals. Only
4x896--strong complete. Administrative cumulative pair caps derived from a four-deal
pilot stopped sequential 100-deal chunks: complete data are prefixes, not random
missingness. Difficult/slow deals and shared machine load could affect where a prefix
stops. Complete-block exclusion avoids selecting fast individual decisions, but does NOT
eliminate informative censoring. Exclude ALL unmarked tails even if tournament rows
exist (calibrate may not have completed). Never impute missing outcomes as losses/zero.

Common 900 and early/later checks retain the v1 advantage; they do not prove unobserved
suffixes have the same distribution. Hypothetical remaining 700-deal mean would have to
be below about -23.37 SF/seat-game to pull the full 2,000 candidate-v1 point estimate
down to+0.5, or -21.94 to pull it to+1. This is sensitivity arithmetic, NOT a bound or
probability. Source chunk times for candidate-v1: total 3,979.44 s, median 309.93 s, max
391.56 s per 100 deals; pilot caps allowed too little margin. There is no claim that the
old 2,000-deal criterion passed.

## Runtime and integration scope

Recorded machine: i9-10900K, Linux 7.2.11, four CPU workers, no GPU; reservation not
exclusive idle access. Existing candidate latency: cold read/hash/load 43.65 ms,
card-play p90 0.153-0.188ms across reported tricks (30 positions per kind), no
incomplete decisions. These are sampled latency checks, not end-to-end UX latency or
exhaustive fallback counters. The archived smoke log reports
NN/permitted-view/legal-mask/corrupt/incompatible tests and the app manifest test;
current checks are rerun with the bundled v2 weights.

Final source audit (base `843e3978151df72140afe8fad4733ef54dfc08c9`):
`crates/skat-ai/src/opponent.rs` makes Easy exactly `Profile::RULES` and Strong
RULES plus an analysis budget. Both have `search_bidding`, `search_declaring`
and `search_kontra` false, zero temperature/forget/mistakes. `analysis_input`
disables the budget before Spiel, so both use the same Baseline decisions.
`crates/skat-ai/src/neural.rs` applies `kontra_rule` before inference: a rule
Kontra decision returns immediately; otherwise Kontra actions are removed.
Re is not removed and is covered by the model's Spiel phase mask (16).
The same CPU Policy inference therefore chooses on both healthy supported
paths. `crates/skat-app/src/session.rs` supplies Strong, whereas the frozen
comparison uses `nn:PATH@easy`. Unsupported pre-play uses identical rules;
unexpected Spiel failure would break equivalence because Strong then searches.
The real-weight regression `bundled_v2_matches_app_and_evaluation_profiles`
checks matching explained decisions and absence of unexpected Spiel fallback on
eight fixed deals, header/size and corruption rejection. Other existing
real-weight and generic tests cover permitted views, legal masks and broken,
incompatible and non-finite model fallback. This is bounded source/runtime
evidence, not an exhaustive fallback census or a new strength experiment.

Only `card-play.gcnn` is active. D-63, UI, coach and defaults remain unchanged.
The original v1 weights, manifest and card are recoverable as described below.

## Rollback

The matching original v1 weights, manifest and card remain in immutable base
`843e3978151df72140afe8fad4733ef54dfc08c9`. From a repository clone:

```sh
mkdir -p /tmp/grand-coach-v1
for name in card-play.gcnn card-play.json card-play.md; do
  git show 843e3978151df72140afe8fad4733ef54dfc08c9:assets/models/$name > /tmp/grand-coach-v1/$name
done
sha256sum /tmp/grand-coach-v1/card-play.gcnn
GRAND_COACH_MODELS=/tmp/grand-coach-v1 cargo run --locked --release -p gui-egui
```

Expected v1 SHA-256:
`ce64c83cb55b17ec491610bd23da0500ec3d81476d8e4208456d3b06e5524817`.
Exit any running app before restarting: the loader caches success AND failure.
Keep both weights and manifest together. The frozen evaluation archive also
retains `models/v1.gcnn`. This is a directory override, not another selectable
runtime mode or additional bundled model.
