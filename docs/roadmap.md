# Implementation status and next work

The playable desktop app includes the rule engine, series and score sheet,
Classic strength levels and special tables, German and English card and coach
views, six lessons, an optional learning focus and a debriefing of up to three
decisions, an optional counting aid and counting questions. God Mode exposes
the deal to the player and coach. The debriefing can show the table as it
appeared when the player acted. Platform coverage and release checks are in
[testing.md](testing.md).

| Area | Implemented | Remaining work |
|------|-------------|----------------|
| Offline engines | Optional Neural test table with PPO v2 4×384 and Strong fallback under the [operational selection](neural-operational-selection.md). Training, role tournaments and PPO for v2 completed with a [Phase 2 no-go](phase2-outcome.md): 17/18 comparisons timed out; historical retain-v1 outcome preserved, Classic still default. | No further training or arenas authorised by this outcome. Unresolved evaluation evidence, further engine selection, learned pre-play, hybrid search and learned coach integration remain in [the neural epic](https://github.com/naturelle137/grand-coach/issues/67). |
| Analysis performance | Opt-in [two-stage prototype and bounded tuning diagnostic](analysis.md#two-stage-card-play-experimental-104) completed; defaults unchanged. | [#104](https://github.com/naturelle137/grand-coach/issues/104) remains open: [observation/tools unsupported-mode no-go](issue104-observation-no-go.md); missing instrumentation prevents admission. Fresh validation, labels, paired throughput and strength remain pending under frozen conditional budgets; no further tuning or compute authorised. |
| Card counting (M6) | Analysis tracks played cards, Trumpf, Augen and proven voids. Coach lessons use these facts. An optional [counting aid and in-game quiz](coach.md#counting-aid-and-quiz-d-64) show exact facts from the player's view. [Focused retrieval](coach.md#focused-retrieval-48-extension-to-11) adds two exact lesson families, explicit support levels and in-memory opportunity/answer/help/skip events. | Practice sessions and persistent attempt history belong to M8. |
| Open play and replay (M7) | Seeded deals, game reconstruction and immediate decision snapshots. | Save and load games, step through tricks, retry a deal and explain what closed-play evidence would reveal in open mode. |
| Training and release (M8) | A [release test plan](release-test-plan.md) and [learner protocol](coach-validation.md). | Let the player choose a theme and practise it across varied positions, with gradually less help and a fresh unaided task. Record which attempts needed assistance. Package the game and notices for Linux and Windows 11, perform learner and platform checks, fix findings and retest. |
| Second frontend (M9) | Frontend-independent session API. | Add Godot via gdext after the desktop training and release work. |

For coach changes, read the [live behaviour](coach.md) and
[evidence checklist](coach-validation.md). Use the [opponent guide](ai.md)
for strength calibration and [model guide](learned-models.md) for training.
