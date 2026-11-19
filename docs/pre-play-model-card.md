<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Model card: learned pre-play imitation (#74)

**Status: first imitation model trained, not shipped.** The 2026-11-19 run
(below) plays complete games but is clearly weaker than Classic Easy and
Strong. This card fixes what a model must cover and records label evidence
and measured results. Classic stays the default and supplies every pre-play
decision.

| Item | Value |
|------|-------|
| Intended phases | Reizen, Hand/pickup, Drücken (joint pair, 66 legal pairs of 496 indices), Ansage; Kontra/Re only as analysed teacher decisions |
| Encoder / head | shared v2 encoder (634 permitted-view features), 621-action policy head; `train --all-phases` ([tooling](learned-models.md#all-phase-imitation-preparation-74)) |
| Teacher | Strong profile of #70 records; pre-play labels are the shared Classic rules of `skat-analysis` |
| Input | the acting seat's `PlayerView` only |
| Weights / hashes | experimental, not in the repo: SHA-256 `6ed45d3400b71b42adf4dc7333c14a59a613056623914c3c49b628fae5ad9aa8` (4×384, 5,621,708 bytes, trained on `T-2-10k`, `--epochs 4`) |

## Label validation

`nn-train validate-labels DIR` first runs `verify` (full replay, checksums,
score check), then recomputes each Reizen, pickup, Drücken and Ansage rule
from the stored view alone and compares it with the label. It also counts
views carrying a result and discards that are not a pair of the twelve cards.

Measured once on a fresh Strong teacher-only run (seed 0, 40 deals, 10 per
shard, engine commit of this change): Reizen 286/286, Hand/pickup 40/40,
Drücken 39/39 and Ansage 40/40 labels equal the rule; no leaks, no illegal
pairs. The Strong profile has `search_bidding`, `search_declaring` and
`search_kontra` off, so agreement is expected by construction; it confirms
the pipeline stores the rule's choice, unleaked and legal. It is a 40-deal sample: it says nothing about strategic
quality, rare contracts, Kontra/Re labels or double-dummy bias (D-40), and a
production dataset must be validated again.

A further read-only validation of the retained Strong `T-2-10k` dataset
(seed 73, engine `99c91300dee7`, 10,000 deals) found 82,866/82,866
Reizen, 9,931/9,931 Hand/pickup, 9,908/9,908 Drücken and 9,931/9,931
Ansage labels equal the current rules, with zero view leaks or illegal
pairs. `nn-train validate-labels` also verified the shards and replayed the
records. This checks storage and rule agreement, not strategic quality or
Kontra/Re label quality; the original manifest predates the explicit
`search_kontra` profile field and must not be treated as proof of that
phase's teacher configuration on its own. The recorded engine commit
`99c91300dee7` predates the Kontra rule change: its Strong profile searches
all Spiel decisions, including Kontra/Re, with the recorded 24-sample budget;
`teach` stores that decision and its analysis. This identifies the intended
teacher from the source, subject to the manifest accurately identifying the
generating binary.

A read-only scan of every retained shard found 6,891 views offering Kontra:
6,854 labels said Kontra and 37 played a card (99.46% Kontra). All were legal,
all had recorded analysis options, none recorded a fallback, and 14 recorded
`BudgetExhausted` rather than `Complete`. The resulting 6,854 Re views had
2,550 Re and 4,304 KeinRe labels; all were legal, had options and no recorded
fallback, with one `BudgetExhausted`. Counts describe recorded teacher output,
not independent reproduction of each choice or proof of its strategic quality.
The extreme Kontra rate agrees with the known D-40 concern and does not justify
shipping a model trained on these labels without separate assessment.

## First training run

Details and tables in [ai.md](ai.md#first-all-phase-imitation-run-74). The
model covers all five phases (Kontra/Re stays with the rules in the opponent).
Pooled teacher agreement 71.59 % on 42,322 test rows; in 1,200 complete games
it scored -34.60 [-40.27, -28.94] against Easy and -43.39 [-49.13, -37.65]
against Strong per seat-game. It passes too rarely (14.9 % of 6,883 Reizen
actions, Strong 23.5 %) and never declares Null, Hand or announcements
(0 of 774 declarer games). It is a weak first result.

## Open before a production claim

A stronger dataset/recipe (rare-class handling, per-phase accuracy),
repeated [calibration](ai.md#calibration-protocol) after improvement,
validated Kontra integration (#108), weights hashes, licence and provenance in
[third-party](third-party.md), and the outcome-label pilot (postponed).
