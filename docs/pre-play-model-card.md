<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Model card: learned pre-play imitation (#74)

**Status: no production model.** No all-phase imitation weights are shipped;
the Phase 2 compute hold still forbids production training and arenas. This
card fixes what such a model must cover and records the label evidence that
exists. Classic stays the default and supplies every pre-play decision.

| Item | Value |
|------|-------|
| Intended phases | Reizen, Hand/pickup, Drücken (joint pair, 66 legal pairs of 496 indices), Ansage; Kontra/Re only as analysed teacher decisions |
| Encoder / head | shared v2 encoder (634 permitted-view features), 621-action policy head; `train --all-phases` ([tooling](learned-models.md#all-phase-imitation-preparation-74)) |
| Teacher | Strong profile of #70 records; pre-play labels are the shared Classic rules of `skat-analysis` |
| Input | the acting seat's `PlayerView` only |
| Weights / hashes | none yet |

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

## Open before a production claim

Production dataset and training run (separate admission), behavioural
calibration of complete learned games (Reizen/pass/Einpassen rates, contract
mix, declarer/defender scores; [calibration protocol](ai.md#calibration-protocol)),
validated Kontra integration (#108), weights hashes, licence and provenance in
[third-party](third-party.md), and the outcome-label pilot (postponed).
