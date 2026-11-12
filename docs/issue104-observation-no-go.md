# Issue #104 observation/tool admission: no-go

HQ531 selects the explicitly permitted unsupported-mode stop after the
HQ520 diagnostic, rather than shipping a subset of the required four-tool
increment. This is a tool-capability no-go, not evidence that two-stage
sampling cannot help. No runtime code, defaults, CLI switches, datasets or
teacher labels change. No benchmark, generation, training, calibration,
arena or validation was launched. Issue #104 remains open and all six
acceptance checkboxes remain unticked.

## Source-backed capability assessment

Inspection baseline: merged prototype source
`58411d6e44ae5cc446e484049760592148f7b9e8` (PR #120).
The [actual pre-fix pilot](analysis.md#actual-bounded-tuning-diagnostic-hq520-audited-by-hq521)
is diagnostic evidence only. The following are missing capabilities of
this baseline, not implemented switches or claims of new test coverage.

| Mode | Actual interface | Admission blocker |
|---|---|---|
| Paired positions | [`two_stage` example](../crates/skat-ai/examples/two_stage.rs), `main` | Reference always timed first; no explicit repeat/order, seat, stable view ID, actions, full estimates, completion/fallback or labels in TSV. |
| Latency | [`latency`](../crates/skat-ai/examples/latency.rs), `main` | Existing `experimental` mode compares different search kernels; it is not the two-stage candidate. No paired two-stage decision/report output. |
| Analysis bench | [`bench`](../crates/skat-analysis/examples/bench.rs), `main` | The second analysis uses seed +1000000 to assess estimation quality. Only the first is timed. This is not a same-input paired comparison. |
| Generator | [`teach` and `play`](../tools/nn-train/src/selfplay.rs), [`generate`](../tools/nn-train/src/shards.rs), [`generate` CLI](../tools/nn-train/src/main.rs) | `teach` retains status/fallback/options but discards effort; only `decide_analysed` runs. `play` advances the trajectory with that action. No same-view reference/candidate labels, stage provenance, paired order or candidate identity in the manifest/resume contract. |
| Calibration | [`Side`, `Seated`, `parse` and `seat`](../crates/skat-ai/examples/calibrate.rs) | Classic, Experimental and Neural dispatch exists, but no two-stage spec or report capture. Experimental denotes the other kernel, not this candidate. |

The common permitted-view sampler already exists in
[`analyze_two_stage`](../crates/skat-analysis/src/two_stage.rs).
[`Opponent::decide_two_stage`](../crates/skat-ai/src/opponent.rs) uses it and
filters survivors before actual selection, including nonzero temperature
and OneSurvivor. `best_option`/`agreement` alone are not an authoritative
record of that actual opponent selection. The adapter also returns no
analysis/report for rules-only decisions or failed analysis; these must
receive explicit unavailable/not-analysed reasons, never fabricated
`Complete` statuses.

A generator flag alone is insufficient. The trainer's completeness
predicate in [`train`](../tools/nn-train/src/train.rs) checks `Complete`,
legal-option count and presence of every estimate. It does not require
identical per-option sample counts. Two-stage's completed schedule can
retain stage-1-only estimates for dropped options. Existing labels are
valid under their existing semantics; treating candidate outputs as
full-budget reference labels would require an explicit separate
provenance/completeness contract. Likewise global `Effort::effective_samples`
in [`analysis`](../crates/skat-analysis/src/analysis.rs) is not a substitute
for missing per-option ESS (notably existing Drücken screening).

Supplying all requested modes honestly therefore requires coordinated
observation, teacher-record/manifest/resume and consumer semantics, plus
four CLI integrations and behavioural tests. That is broader than adding
observation fields to the existing paired example. HQ531 stops this
bounded increment here: no partial adapter, invented scalar label,
relabelling of the Experimental kernel, duplicate sampler, speculative
sequence of follow-up PRs or silent expansion into training behaviour.
This is a scope/admission decision, not a demonstrated technical
impossibility. The required instrumentation and new behavioural tests
remain unimplemented.

## Requirements still blocking measurement admission

A capable instrument must emit both paths' full estimates and sample
counts/per-option ESS, actual selected action, analysis status and
planned/finished/nodes, decision fallback and explicit unavailable reasons,
and stage outcome/skips/survivors. Include exact seeds, role, trick, seat,
stable permitted-input identifier, teacher options and engine spec.
Capture and hash actions and complete label records from identical teacher
inputs; nn-train still owns shard writing and verification.

Timing must alternate deterministically from explicit repeat and position
indices, recording raw order, wall time and nodes with identical
views/seeds/budgets/workers/threads. Distinct holdout keys identify distinct
role/trick/seat positions; timing repeats never increase the position count.
Evaluate differing-choice regret against reference full estimates, without
inferring estimate equivalence from matching selected actions.

Before admission, behavioural tests must demonstrate unchanged legacy/default
outputs, actual selection/report consistency (OneSurvivor and nonzero
temperature), partial/cancelled/fallback/unavailable serialization,
deterministic order alternation and permitted-view invariance. No counts-only
substitute. This docs-only no-go adds none of those capabilities or tests.

## Frozen conditional proposal and ceilings

This records the HQ503/HQ521 predeclared plan before any later compute;
it authorises no campaign. The withdrawn tuning grid is not reinstated.
Fresh data must never tune the proposal. Use 24 worlds, screen 8,
SE multiple 2.5, floor 2, minimum 2 and maximum 4 survivors. No parameter
search, Medium 6-world enablement, Strong/Medium/coach/generator default
change or automatic dataset regeneration.

| Conditional stage | Fixed population/repeats | Hard wall ceiling |
|---|---|---:|
| Fresh paired validation | 400 distinct role/trick/seat positions, seeds ≥2000001, 3 timing repeats | 90 min |
| Paired tools | Latency, analysis bench and generator, each 3 runs; identical deals/seeds/workers/threads between paths | 30 min total |
| Strength | 200 common fresh underlying deals with seat/role rotation; deal-paired 95% interval | 60 min |

All jobs must be deterministic, resource-admitted shell jobs: 4 CPU,
4 GiB, no GPU, explicit affinity to four admitted CPUs. Record exact worker
and analysis-thread limits. Before launch, freeze source/archive/binary
identity and concrete build/run deadlines in the shell script. Build deadline
is at most 300 seconds per job; each run deadline must fit its stage's
remaining wall ceiling. All builds, runs, writes and verification count
inside stage and aggregate ceilings: at most 180 min wall and 12 CPU-hours.
No budget expansion, retry or replacement run on timeout; stop the chain
with a concrete inconclusive/no-go record.

Generator timing includes shard writing and `nn-train verify`; retain and
hash actual same-input teacher actions/options/labels, not a scalar proxy.
Report throughput separately from playing strength. A completed measurement
must report ≥97% action identity and differing-decision regret. Strength's
paired interval must not lie clearly below zero; overlap with zero is
inconclusive and supports no noninferiority claim. Equal actions alone
prove neither label equality nor strength. No default changes or issue
closure without the full evidence.

## Ownership and local assistance

HQ534 owns this PR's single independent fresh-session review, at most one
bounded fix round, named-blocker checks and final-head green CI/integration
or a concrete terminal no-go. It must verify actual artifacts and tool
capabilities before any later resource admission. This no-go supplies no
permission to start compute or another speculative implementation chain.

Source assistance was checked against current code: HQ523 was accepted by
HQ524; HQ519, HQ533 and HQ536 independently corrected/rederived the paired
CLI, generator and bench baselines. Their original local jobs remain
rework, not accepted execution evidence. No baseline inventory was repeated.
HQ531 found free local capacity and delegated only the new teacher-label
predicate source question as HQ539, with separate acceptance owner HQ540.
Its output is pending independent acceptance and is not used as evidence
for this decision. HQ534 owns the product closure; HQ540 owns checking that
local artifact and the next useful bounded assignment.
