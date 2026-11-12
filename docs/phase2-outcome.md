<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Phase 2 outcome and evaluation card

**2026-11-12: no-go due to incomplete evidence; retain supervised v1 and
Classic as the default.** No replacement model is approved. This does not
establish that the candidates are stronger or weaker. The
[predeclaration](phase2-evaluation.md), integrated in
[PR #118](https://github.com/naturelle137/grand-coach/pull/118)
(commit `44d1d6a20b214f2cd7859e81ead6514c2f2b576f`), remains unchanged.

## Coverage and decision

The frozen matrix requires 18 comparisons, each using both `calibrate` and
`tournament` on 2,000 common validation deals (indices 8000000–8001999).
**17 comparisons failed with `subprocess.TimeoutExpired`; only HQ #401,
4×896 versus Strong, completed.** There are 232 hash-verified 100-deal chunks:
23,200 of 36,000 required pair-deals, with 12,800 missing. A pair-deal means
one underlying deal for one comparison, completed by both tools. Comparisons
reuse the same underlying deals; these totals are not independent samples.

| HQ job | Comparison | Verified pair-deals | Missing |
|---|---|---:|---:|
| 387 | 4x384--v1 | 1,300 | 700 |
| 388 | 4x384--4x384-start | 1,300 | 700 |
| 389 | 4x384--easy | 1,200 | 800 |
| 390 | 4x384--medium | 900 | 1,100 |
| 391 | 4x384--strong | 1,200 | 800 |
| 392 | 4x512--v1 | 1,000 | 1,000 |
| 393 | 4x512--4x512-start | 1,200 | 800 |
| 394 | 4x512--easy | 1,100 | 900 |
| 395 | 4x512--medium | 1,000 | 1,000 |
| 396 | 4x512--strong | 1,300 | 700 |
| 397 | 4x896--v1 | 1,200 | 800 |
| 398 | 4x896--4x896-start | 1,400 | 600 |
| 399 | 4x896--easy | 1,300 | 700 |
| 400 | 4x896--medium | 1,200 | 800 |
| 401 | 4x896--strong | 2,000 | 0 |
| 402 | 4x384--4x512 | 1,300 | 700 |
| 403 | 4x384--4x896 | 1,400 | 600 |
| 404 | 4x512--4x896 | 1,900 | 100 |

The tournament files contain 143,731 game rows, including partial chunks:
23,971 pair-deal indices have rows, 23,941 have all six match slots and
30 have partial slots. Thrown-in reference deals produce no tournament rows.
These counts do not replace completed chunks from both tools. All checked
rows have twelve fields; no duplicate `(deal, match)` keys occur within a file.

Completeness already fails the shipping rule. No winner, strength interval,
role advantage or Null/Hand/Ouvert competence is claimed from a selected
subset. The frozen simultaneous lower-bound requirement above +1 SF point
against v1 and each candidate's supervised start, role and technical criteria
still apply. History use, ablations, option quality and learning efficacy
remain unproven; the other criteria of #75 and #93 remain open.

## Model identities and runtime limits

The three candidates are frozen tuning winners from `2026-11-10-rl-4x`,
started from supervised models. They use the v2 permitted-view encoder
(634 features); hidden layers × width identify each shape. Full recipes and
supervised starts are pinned in HQ `var/jobs/15/evaluation/manifest.json` and
its `provenance/` files. Training data, logs and candidate weights stay outside git.

| Candidate | Iteration | Bytes | SHA-256 |
|---|---:|---:|---|
| 4x384 | 175 | 5621708 | `6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75` |
| 4x512 | 224 | 8278988 | `1d8f901a5e1cf0c79824e61234bc3fc92c9fe72d9a3d71fe61704c1f8e1984bf` |
| 4x896 | 341 | 18610124 | `dcdacea8058da28d534a1ca2f75077f59520bc2687e7c8da350eb30636311432` |

All three candidates, their three supervised starts and v1 match the frozen
manifest. **4×896 remains evaluation-only:** its 18,610,124 bytes exceed
D-63's strictly-below-10,000,000-byte limit ([decisions](decisions.md)).
The other two fit the size limit but lack complete acceptance evidence.
Retained v1 is 5,481,164 bytes, SHA-256
`ce64c83cb55b17ec491610bd23da0500ec3d81476d8e4208456d3b06e5524817`;
its [original model card](../assets/models/card-play.md) still defines its
measured strength and limitations. No weights, manifests or app behaviour change.

## Runtime evidence and limits

Runtime/smoke HQ #385 and pilot #386 finished successfully. The known-local
seed audit passed; it does not exclude unrecorded external datasets or equal
card distributions from different seeds. Configurations and binaries match
their pinned hashes. Technical tests are not playing-strength evidence.

The four-deal pilot froze N=2,000 with estimated serial duration 57,533 seconds
and conservative duration 115,066 seconds at four reserved CPUs. The campaign
cap was 96 summed four-CPU wall hours (384 CPU-hours); the runner also imposed
pilot-derived per-comparison time limits. Their 17 timeouts show the pilot did
not adequately predict later runtime; load or rare deals are not isolated causes.
Successful chunks alone account for 107,995 seconds (30.00 summed wall hours),
a lower bound excluding interrupted chunks and preparation. This is neither
full resource accounting nor authorisation to spend a calculated remainder.

## Audit and handoff

HQ #406 `audit.py` reads markers, output hashes, TSVs and terminal job logs;
`coverage-audit.json` records every pair and file hash. Its SHA-256 is
`9ced8ffa23757965c9297074a78e312857f8ce47057839a55a4e18b4b41b08fe`.
A read-only replay for this card reproduced it exactly, with no hash failures,
malformed rows or duplicate keys. HQ #406's fresh independent review approved
the final no-go card without blockers. That card was already accepted by
Telegram in thread 5, message 264; repository persistence does not resend it.
Public outcome records: [#75](https://github.com/naturelle137/grand-coach/issues/75#issuecomment-6825590440)
and [#93](https://github.com/naturelle137/grand-coach/issues/93#issuecomment-6825591760).
Local HQ paths identify evidence archives, not files shipped in this repository.

Blocked selection HQ #405 was cancelled. There is no new evaluation or training
mandate: no new arenas, trainer starts, STOP files or signals. Seeds, shortlist,
N, acceptance rules and original artefacts remain frozen. Any future evaluation
needs a credible runtime plan agreed before execution. Issues are not closed
and unrelated checklist items are not marked complete by this outcome.
