# Phase 2 held-out evaluation predeclaration

This protocol was fixed on 2026-11-12 before held-out results. It does not
change the shipped weights or Classic default, and cannot establish learning
efficacy. HQ #15 owns logistics for #75 and #93; their other criteria remain open.

## Frozen inputs

Campaign `2026-11-10-rl-4x` ended through HQ #176 and read-only barrier #14.
STOP was created at 05:00:21.502158 UTC; confirmed idle at 05:11:52.090118 UTC.
All 15 checkpoint/model hashes matched on recheck, all five completion markers
were present, and no recorded failure marker was found. Original reaped exit
codes are unavailable. These facts allow evaluation, not a training-success claim.

The final complete `cross-09` tuning arena selects the best retained candidate
per shape by its original equal-weight mean over NN and Medium parts:

| Shape | Iteration | Tuning score | SHA-256 | Bytes |
|---|---|---|---|---|
| 4×384 | 175 | 4.977 | 6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75 | 5621708 |
| 4×512 | 224 | 2.956 | 1d8f901a5e1cf0c79824e61234bc3fc92c9fe72d9a3d71fe61704c1f8e1984bf | 8278988 |
| 4×896 | 341 | 3.670 | dcdacea8058da28d534a1ca2f75077f59520bc2687e7c8da350eb30636311432 | 18610124 |

No runner-up has an independently justified advantage. These are tuning
winners, subject to selection bias, not validated strength claims. The
original campaign is preserved. Separate copies of the three candidates,
their corresponding supervised starts and bundled v1, plus recipes, state,
logs, final arena configs/results and hashes, live in HQ
`var/jobs/15/evaluation/`. Its `manifest.json` records original paths, hashes,
start identities and recipe. Hashes establish identities at freeze time;
the original arenas did not hash each input retrospectively. Snapshot names
alone are never accepted as identity. The trainer is `fdb2109c2f73`;
supervised provenance is in the copied per-shape `train.json` files.

## Seeds and fixed matrix

- Pilot: tournament/calibrate **tuning** indices 7900000–7900003; four
  independent deals per comparison. Pilot outcomes cannot alter the shortlist.
- Held-out: **validation** indices 8000000–8001999 reserved exclusively for
  this protocol. All comparisons use the same prefix of this block.
- `deal_seeds` emits the exact ChaCha8 stream used by both existing tools.
  The preparation shell checks actual u64 seed disjointness against known SL
  manifests (including v1 and embedded start recipes), RL seed/iteration/game
  ranges, and both arena suites through the largest locally observed TSV index.
  It hashes the source inventory and outputs `seed-audit.json`. A collision,
  unknown dataset seed scheme or failed check forbids held-out execution;
  a replacement block requires another predeclaration before results.
- The audit covers known local artefacts, not inaccessible/unrecorded external
  campaigns. Different u64 seeds could in principle produce identical deals;
  this audit does not exhaustively compare card permutations.

For **each** candidate compare against v1, its own supervised start, Easy,
Medium and Strong (15 comparisons). Add the three candidate cross-shape
pairs (18 total). Never omit 4×896 from the final evidence for packaging reasons.
Each pair uses both existing tools, with policy `nn:PATH@easy` and rule Kontra:

1. `calibrate`: six A/B seat allocations per underlying deal, including each
   absolute seat and both majority/minority roles. Its native stderr records
   paired per-deal differences; the stdout retains declarer/defender counts.
2. `tournament`: pairwise replacement in three fixed contexts: candidate or
   comparator as declarer against Easy/Easy, as declarer against Medium/Strong,
   and as defender with Medium against Strong. Odd deal indices swap defenders.
   These six matches share the underlying deal and seat RNGs. The role runner
   fixes each deal's reference declarer seat; absolute rotation comes from
   `calibrate`, not a false claim that tournament rotates it on every deal.

Every pair therefore costs at most twelve games per deal, plus Easy reference
runs. Thrown-in reference deals generate no tournament rows; mismatched
contracts/declarers are excluded in paired context analysis and counted.
The calibrate comparison includes thrown-in games as the existing tool defines.

## Pilot, resources and budget

All builds, latency measurements, seed audits and expensive games run in HQ
shell jobs, initially **4 CPU, 4 GiB RAM, no GPU**. Run the latency/smoke shell
before full arenas; record target CPU/OS, load and the fact that HQ reservations
do not guarantee an otherwise idle machine. Use 30 positions per phase for
all seven files, p50/p90/max, cold file-read/hash/load timing, file size and
fallback counts. Unsupported phases deliberately measure Strong fallback in
the existing latency tool; separate those from covered card-play inference.
Run legal-mask, hidden-card substitution/permitted-view, corrupt-hash,
unsupported-phase and app manifest tests. No frontend behaviour changes.

The four-deal pilot has a one-hour wall budget (individual command cap ten
minutes). It records measured per-pair throughput before a held-out game.
The full campaign's cap is **96 summed wall hours at four CPU = 384 CPU-hours**.
N is frozen from timings only: `min(2000, floor(cap / (2 × summed seconds/deal)))`;
if below 200, no full matrix is launched. The factor two covers timing variation.
`budget.json` publishes measured rates, N, central/conservative duration estimates
and timestamp before full execution. N below 2000 means reduced precision;
never extend N after reading results. A timeout/failure is incomplete evidence,
not a win/loss or permission to select a convenient subset.

Tournament resumes existing whole `(deal,match)` rows. The wrapper rejects
interior corruption/duplicates and saves a truncated final row before repair.
Calibrate has **no native resume**: only the interrupted 100-deal chunk reruns;
completed chunks have hashes and completion markers. Never append different
configs/binaries/models to an existing output. Pilot timings are not resumable.

## Analysis and acceptance fixed before results

`analyze.py` pairs comparisons within each deal and aggregates correlated
contexts before computing uncertainty. Report independent deals, total games,
excluded contexts, declarer/defender results, seat and contract strata (including
sparse/empty Null/Hand/Ouvert limitations). Role and contract tables are
descriptive; small strata do not certify competence. Native calibrate per-deal
values are rounded to 0.01; intervals are widened by 0.01. Report nominal 95%
normal intervals and simultaneous intervals with z=2.638 for the six primary
candidate-v1/start comparisons (Bonferroni family alpha 0.05). Classic and
cross-shape comparisons are exploratory; avoid selecting another candidate
from validation scores after a predeclared candidate fails.

A shipping candidate requires **all** of the following:

- Complete, hash-verified matrix for all three shapes and all comparisons;
  seed audit passed; actual measurements and smoke tests passed.
- Simultaneous lower bound **above +1 SF point per seat-game** against both
  v1 and its own start. Otherwise the strength decision is inconclusive/no-go.
- No material role regression: paired declarer and defender nominal lower
  bounds must exceed −3 SF points; report sparse strata rather than extrapolate.
- Covered card-play latency p90 below one second, no unexpected fallback or
  illegal action; manifests/hash checks and permitted-view tests green.
- D-63: each model file **strictly below 10000000 bytes**. 4×896 is evaluation
  only unless a separately reviewed technical decision revises that limit.

Retain current v1 on incomplete, inconclusive or no-go evidence. At most one
replacement is proposed initially. Two weights require a separate explicit
runtime/packaging decision and justified user benefit; current runtime selects
one filename. Any weights/selection change needs model cards, SHA-256,
manifests, original provenance, PR, one independent review, gates and green CI.
Neither inference tests nor this protocol close other #75/#93 criteria.

## Reproduction and delivery

The operational commands and actual HQ shell IDs are recorded in #75 and the HQ #15
`notes.md`; the downstream Codex selection job must depend on the **actual
arena shell IDs**, and the runtime smoke shell, rather than only this preparation
job. The reproducible entrypoints are
[scheduling and execution](../scripts/phase2/campaign.py) and
[paired analysis](../scripts/phase2/analyze.py).
The downstream owner publishes actual final evidence/selection through the
standard HQ outcome delivery to Grand-Coach **thread 5**. Report delivery only
on confirmed success. Internal working papers are not CEO deliverables.
