# Didactic coach work packages

Status, 2026-11-01: A–G and K implemented; H–J and L open. D-61 defers
manual validation to [#81](https://github.com/naturelle137/grand-coach/issues/81)
and #54 before public 1.0. Concept and historical implementation comparison:
[coach-didactics.md](coach-didactics.md). Tracking feature:
[#39](https://github.com/naturelle137/grand-coach/issues/39).

## Assignments and dependencies

Each issue is a bounded Claude Code or Codex assignment with its own
acceptance criteria, relevant files, exclusions and prerequisites. S means
one focused PR; M means one agent and normally one or two focused PRs,
not a promised elapsed time. Split an M package further if its inspected
implementation requires a broad architectural change.

| Package | Feature request | Outcome | Size | New-package prerequisites |
|---------|-----------------|---------|------|---------------------------|
| A | [#41](https://github.com/naturelle137/grand-coach/issues/41) | Compact debriefing, estimates off by default, honest fallback | S | None |
| B | [#42](https://github.com/naturelle137/grand-coach/issues/42) | Shared evidence/provenance contract, no new strategy engine | M | None |
| C | [#43](https://github.com/naturelle137/grand-coach/issues/43) | Two exact lessons: remaining Trumpf and proven voids | M | B, K |
| D | [#44](https://github.com/naturelle137/grand-coach/issues/44) | Two narrow card-play mechanisms with conditions/counterexamples | M | B, C, K |
| E | [#45](https://github.com/naturelle137/grand-coach/issues/45) | Pre-play structure and legal-value lessons | M | B, K |
| F | [#46](https://github.com/naturelle137/grand-coach/issues/46) | One optional focus, staged hints and teaching-based moment selection | M | A, C, D, K |
| G | [#47](https://github.com/naturelle137/grand-coach/issues/47) | Historical own hand and current trick in moment cards | M | A, B, K |
| H | [#48](https://github.com/naturelle137/grand-coach/issues/48) | Optional retrieval questions and explicit assistance levels | M | C, F; M6 quiz APIs |
| I | [#49](https://github.com/naturelle137/grand-coach/issues/49) | Local assistance-aware descriptive progress | M | B, H; coordinate M8 storage |
| J | [#50](https://github.com/naturelle137/grand-coach/issues/50) | Twelve legal practice cases and varied unaided transfer | M | D, G, H, I; M7/M8 foundations |
| K | [#51](https://github.com/naturelle137/grand-coach/issues/51) | M5 evidence checklist, learner UAT preparation and protocol | S | B |
| L | [#54](https://github.com/naturelle137/grand-coach/issues/54) | M8 learner UAT, new unaided tasks and delayed follow-up | M | K, J; M8 foundations |

Agreed sequential order:

**#41 → #42 → #51 → #43 → #44 → #45 → #46 → #47 → M5 technical delivery (#10; manual checks → #81) → #11 → #48 → #12 → #13 foundations → #49 → #50 → #54 preparation → #54 sessions / #81 → public 1.0 acceptance.**

Issue #51 prepares the checklist/protocol before lessons exist
([coach-validation.md](coach-validation.md), D-54); each later M5
PR applies its part A and supplies applicable learner UAT steps (part B). Record combined
M5 learner UAT after #47 in #81 before public 1.0; it does not block
technical #10 completion or #11 (D-61). #51 does not wait for those runs.
issue #54 prepares and later performs the integrated evaluation after #50.

Issue #13 delivers the storage/training foundations first. Final M8/release
acceptance is reconciled after #49/#50/#54 and all remaining #13 criteria;
do not require closing #13 before those follow-ups or create a circular
dependency. #14 follows completed M8 and is not a prerequisite above.

The inspected local D-49 commit `9eda0d9` supplies staged hints and move
reviews and was ahead of inspected main `edc0db0`.
[PR #38](https://github.com/naturelle137/grand-coach/pull/38) merged during
preparation; this documentation was rebased onto main `f733d70`. The
prerequisite is now available. Reinspect its merged APIs before coding.

## Delivery slices

1. **Readable review (A):** immediate relief from the screenshot's overload.
   No invented replacement explanations; existing comparisons stay in details.
2. **Teaching foundation (B/C/D/F/G; E separately):** exact lessons, supported
   mechanisms, focus and understandable historical context. Engineering
   readiness needs these tests; educational validation still needs the
   maintainer's learner UAT (not expert review).
3. **Independent practice (H/I/J):** selective retrieval, local observations
   and new tasks without help, after the milestone-owned foundations exist.
4. **Validation (K in M5, L in M8):** prepare the evidence/UAT protocol
   after B; prepare scenarios as features arrive, execute learner UAT and
   unaided transfer/delayed follow-up after J before public 1.0 (D-61).

A package's PR should demonstrate the changed player experience, include
its independent fixtures, record the actual gate result and update the
relevant coach/analysis/decision docs. GUI packages include screenshots per
D-45. New lesson definitions need positive, boundary and counterexample
cases, not just snapshots of the implementation's own output.

## Existing issue ownership

| Existing issue | Keeps ownership | New work relationship |
|----------------|-----------------|-----------------------|
| [#10 M5](https://github.com/naturelle137/grand-coach/issues/10) | Live coach lifecycle; manual smoke/UAT transferred to #81 | A–G are implemented; no manual validation is claimed |
| [#11 M6](https://github.com/naturelle137/grand-coach/issues/11) | Counting aid and baseline in-game quiz | C supplies lesson evidence; H adds retrieval/fading through that API |
| [#12 M7](https://github.com/naturelle137/grand-coach/issues/12) | Replay/save/load, open mode and same-deal attempts | G adds immediate permitted snapshots; J reuses reconstruction |
| [#13 M8](https://github.com/naturelle137/grand-coach/issues/13) | Baseline learning history/training and release packaging | I/J refine progress/practice; reuse storage/session APIs, no parallel subsystem |
| [#67](https://github.com/naturelle137/grand-coach/issues/67) | Additional offline learned backends; supersedes closed #34/#35 | Independent; neither a prerequisite nor a strategic explanation source by itself |

Do not silently rewrite or close existing milestone issues. At implementation
start reconcile delivered APIs and overlaps. New checklists track only the
additional outcomes; technical parent closure requires its functional/automated
scope and explicit manual handoff to #81. #13 final readiness still needs
actual #81/#54 results. Consolidation of this document is tracked by #83.

## Shared acceptance boundaries

- Analysis computes all strategic facts/mechanisms; the coach only selects
  and verbalises structured IDs/parameters. No separate counting or search.
- Frontends use `skat-app`; plain serde-capable data, Fluent German/English.
- Preserve D-49 before live moves, including implicit recommendation leaks.
- Historical debriefing uses the information available at the decision,
  even after actual hidden cards become available elsewhere.
- Preserve seeds, budgets, cancellation, stale-result rejection, warnings
  and ordinary play. Repeated help requests do not restart search.
- Record tests through `scripts/check.sh`; tick only verified work.
- No human Skat expert is available or required for sign-off. The maintainer
  performs learner UAT for clarity/usability, not expert certification.
  Factual/strategic claims need independently specified evidence and tests;
  another AI or the same solver's agreement is not an independent oracle.
  The checklist, UAT script and templates are in
  [coach-validation.md](coach-validation.md).
- Distinguish technical correctness, learner comprehension, assistance and
  new unaided performance. K closes on preparation; L and the applicable
  human-run items of #39 remain pending until actual execution. No invented
  participants, expert approval, mastery or general efficacy claims.
