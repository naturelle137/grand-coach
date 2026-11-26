# Godot frontend (M9): parity checklist

Contract for the second frontend (D-79). The egui app stays; Godot adds the
same trainer behaviour on `skat-app` (D-12). Functional parity is required,
pixel-identical layout is not. A row is ticked only when the Godot flow
exists **and** its verification is recorded in the last column.

## Architecture

- `crates/gui-godot` is the only crate with Godot types (gdext 0.5.5,
  MPL-2.0). `adapter` is Godot-free: JSON text in/out of the serde forms of
  `skat-app` `SessionConfig`, `Command`, `TableView` and `AppEvent`.
  `node` exposes it as the GDScript class `SkatApp`.
- `godot/` is the Godot 4.4 project (`project.godot`, `grand_coach.gdextension`,
  scene and scripts). Scripts only render views and send commands; no rule,
  analysis or coach logic.
- Lifecycle: opponent moves run on a worker thread tagged with the session
  generation. `restart`, `stop` and drop cancel the work; a result of an
  older generation is discarded and a result for a changed position is
  refused by `Session::finish_opponent`.
- Data: recordings and the learning history use the existing `skat-app`
  formats and file helpers, so switching frontends needs no conversion.
- No change to `skat-core`, `skat-analysis`, `skat-ai` or `skat-coach`.

## Checklist

Status: `done` = implemented and verified, `adapter` = reachable through the
adapter contract only, `open` = not started.

| Capability (egui) | Status | Godot verification |
|-------------------|--------|--------------------|
| Session start/restart/stop, seed, fixed or free series | adapter | `crates/gui-godot/tests/adapter.rs` (determinism, restart, rejected restart) |
| Basic table: hand, phase, first-legal-action play | adapter | `godot/main.gd` (not run: no Godot binary in the agent environment) |
| Opponent moves off the UI thread, stale results | done | `restart_discards_a_running_computation`, `human_move_makes_pending_result_stale`, `dropping_a_busy_adapter_is_safe` |
| Structured errors, unchanged session on error | done | `errors_are_structured_and_leave_the_session_unchanged` |
| Permitted view only; open display; God Mode | adapter | `view_is_the_permitted_view_only` (closed/open); God Mode open |
| Same results as direct `skat-app` | done | `adapter_matches_direct_session_and_is_deterministic` |
| Reizen, Skat, Drücken, Ansage, Hand/Ouvert, all game types | open | |
| Optional Kontra/Re | open | |
| Opponent levels, Goldfish, Mephisto, God Mode, engines | open | |
| Card faces (SVG) and German/English texts from `assets/i18n/` | open | |
| Live coach modes, warnings, proceed/reconsider | open | |
| Counting aid and quiz | open | |
| Guided dialogues and Abstich question | open | |
| Score sheet, series result, series figures | open | |
| Save/open recordings, replay window, comments, review, hindsight | open | |
| Same-deal practice | open | |
| Learning history, statistics, focused training | open | |
| Settings persistence | open | |
| Linux and Windows 11 export, launch smoke, performance | open | |

## Build and run

```sh
cargo build -p gui-godot            # target/debug/libgui_godot.so (gui_godot.dll)
godot --path godot                  # Godot 4.4+, from the repository root
cargo test -p gui-godot             # adapter contract tests, no Godot needed
```

The `.gdextension` file loads the library from `target/`. Export presets,
packaged artifacts and target-platform smoke tests are open work; M9 stays
incomplete until every row above is `done` and the lower layers are verified
unchanged.
