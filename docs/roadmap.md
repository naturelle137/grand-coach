# Roadmap

**Current status:** M0–M5 are implemented. M4 opponent calibration and
historical measurements are in [ai.md](ai.md); the M5 coach, six lessons,
focus and historical debriefing are in [coach.md](coach.md).

The didactic direction and remaining follow-up are specified in
[coach-didactics.md](coach-didactics.md), with bounded assignments in
[coach-work-packages.md](coach-work-packages.md) and tracking issue
[#39](https://github.com/naturelle137/grand-coach/issues/39). Readable review
comes first, verified lessons next, practice/progress afterwards. The work
refines M5–M8. Package A
(#41, D-52: compact debriefing, figures off by default) and package B (#42,
D-53: evidence contract) are implemented. Package K
(#51, D-54) prepared the evidence checklist and learner UAT protocol in
[coach-validation.md](coach-validation.md). Package C (#43, D-55) ships the
first two exact lessons, remaining Trumpf and proven voids
([coach.md](coach.md#exact-lessons-d-55)). Package D (#44, D-56) adds two
narrow card-play mechanisms, a possible or certain Abstich and partner
overtaking in the current trick
([coach.md](coach.md#card-play-mechanisms-d-56)). Package E (#45, D-57)
adds the pre-play lessons on own hand structure and legal game values
([coach.md](coach.md#pre-play-lessons-d-57)); the pre-play estimates stay
coarse. Package F (#46, D-58) adds one optional learning focus, lesson-backed
automatic hints with a neutral comparison question, skipping, and debriefing
moments chosen by lesson, focus and concept
([coach.md](coach.md#focus-and-teachable-moments-d-58)). Package G (#47,
D-59) shows each debriefing moment with the hand, trick, bidding and
history the player saw at that decision, from a permitted snapshot
([coach.md](coach.md#decision-snapshots-d-59)). D-60 (#65) records the
maintainer's decision to keep the opt-in figures stage as the one
exception to live non-disclosure.

God Mode (#28, D-62) is implemented: against Mephisto, the player and the
coach see every card, with its own card faces, back and blue table. It
does not replace open mode (M7), in which the coach explains how the
opponents' hands could have been deduced.

## Next work and dependencies

The [neural-network epic #67](https://github.com/naturelle137/grand-coach/issues/67)
has its foundation from [#69](https://github.com/naturelle137/grand-coach/issues/69):
Burn 0.22 passed the smoke test (D-11), `skat-nn` encodes a `PlayerView` and
runs models on the CPU with Classic fallbacks, `NeuralOpponent` plays in the
arena, `tools/nn-train` trains on CPU or GPU
([ai.md](ai.md#learned-engine-runtime)). Next are
[#70](https://github.com/naturelle137/grand-coach/issues/70) (training data),
then the card-play network #73 and the selectable engine with
difficulty levels #77. Pre-play #74, hybrid search #76, debriefing #78 and
self-play #75 follow in any order. The epic replaces closed proposals #34
and #35; no learned engine is shipped yet. Classic stays default and learned
engines may ship labelled experimental (D-07, stack per D-11). It needs M1–M4 only; M6–M8 and Godot (#14) are not
prerequisites.

The independent didactic path remains #11 → #48, followed by #12, then
issue #13 storage/training foundations → #49 → #50 → #54. Replay metadata in
issues #77/#78 reuses #12's delivered artifacts; model packages in #80 extend
issue #13's packaging foundations. Neither milestone depends on the neural
engines. Separate from #67: the local language-model experiment #79 and the
classical debriefing optimisation #72.

Documentation follow-ups: [#82](https://github.com/naturelle137/grand-coach/issues/82)
(decisions/rule details),
[#83](https://github.com/naturelle137/grand-coach/issues/83) (coach consolidation,
after #82 and before #48), then
[#84](https://github.com/naturelle137/grand-coach/issues/84) (navigation/link
checks, before 1.0). They do not block #67.

## Milestones

Milestones build on delivered technical artifacts (see [vision.md](vision.md)).
The criteria below define completion.

- [x] **M0 – Project foundation**
  Done when vision, decision log, CLAUDE.md, licences and contributor docs are
  in the repository.

- [x] **M1 – Rule engine (`skat-core`)**
  Done when the engine fully covers all ISkO games (bidding, Hand, Skat,
  discarding, suit, Grand, Null and Ouvert, Schneider/Schwarz, game value
  including overbidding, Seeger-Fabian) plus Kontra/Re per D-03, and thousands
  of random games (seeded) run through without rule violations.

- [x] **M2 – First playable version**
  Done when a series against two simple opponents can be played on Linux and
  Windows. Includes the application
  interface (`skat-app`), the egui frontend, self-made cards with switchable card faces, and a German/English
  UI.

- [x] **M3 – Analysis layer (`skat-analysis`)**
  Done when card counter, opponent model (distributions from bidding,
  following suit and discards) and option evaluation (Monte Carlo and
  double-dummy) deliver winning chance and expected points for every
  decision, in playable time, backed by tests against known positions.

- [x] **M4 – Opponent AI with strength levels**
  Done when 3–4 levels play measurably differently strong (AI-vs-AI series)
  and no level sees other players' cards.

- [x] **M5 – Live coach**
  Done when the coach shows the line of thought (considerations, options,
  evaluation with reasoning) for bidding, discarding, game choice and play,
  supports the levels off / on request / important / every move plus the
  optional pre-move warning, works in German and English, and every
  statement comes from the analysis layer. Without telling the move ahead
  (D-49): staged hints before the move, a note after it and a debriefing
  of up to three teachable moments after the game (D-58); requested figures
  are the opt-in exception in D-60.

- [ ] **M6 – Card counting**
  Done when the optional counting aid (cards played, trumps out, card points,
  known voids) and the in-game quiz work.

- [ ] **M7 – Open mode, replay, post-game analysis**
  Done when open mode can be played and the coach shows how the opponents'
  hands could have been deduced. Also: games can be replayed trick by trick
  and replayed with the same deal. The debriefing of a game's good and bad
  moments already comes with M5 (D-49).

- [ ] **M8 – Training and release 1.0**
  Done when error statistics over time and the training mode work and there
  are installable packages for Linux and Windows 11 that ship
  `THIRD-PARTY-NOTICES` (D-43). Once the release scope is implemented,
  the maintainer validates it using the [release test plan](release-test-plan.md),
  fixes findings and releases 1.0.

- [ ] **M9 – Godot frontend** (long term)
  Done when a Godot frontend (gdext) offers the same functionality via
  `skat-app`, without changes to engine, analysis, AI or coach.
