# Godot frontend (M9): parity checklist

Contract for the second frontend (D-79). The egui app stays; Godot adds the
same trainer behaviour on `skat-app` (D-12). Functional parity is required,
pixel-identical layout is not. A row is ticked only when the Godot flow
exists **and** its verification is recorded in the last column.

## Architecture

- `crates/gui-godot` is the only crate with Godot types (gdext 0.5.5, API 4.4,
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
| Basic table: hand, phase, selectable legal actions | adapter | `godot/tests/interaction.gd` played a seeded series in German and English on Godot 4.4.1, checking bids, current-trick labels, SVG hand and trick faces and a legal card click; remaining table presentation open |
| Opponent moves off the UI thread, stale results | done | `restart_discards_a_running_computation`, `human_move_makes_pending_result_stale`, `dropping_a_busy_adapter_is_safe` |
| Structured errors, unchanged session on error | done | `errors_are_structured_and_leave_the_session_unchanged` |
| Permitted view only; open display; God Mode | adapter | `view_is_the_permitted_view_only` (closed/open); God Mode open |
| Same results as direct `skat-app` | done | `adapter_matches_direct_session_and_is_deterministic` |
| Reizen, Skat, Drücken, Ansage, Hand/Ouvert, all game types | done | Bids and Skat/Hand choice are the generic legal-action buttons. Drücken marks two hand cards and confirms one legal `Discard` (no 66 buttons); `godot/tests/interaction.gd` checks selection, confirm state, language switch and the resulting phase on Godot 4.4.1. Announcements use a picker over exactly the legal `Announce` actions with a confirm button, and the discarded Skat is shown after Drücken; the interaction test covers both in DE and EN and plays seeded games from deal to the finished series. |
| Optional Kontra/Re | open | |
| Opponent levels, Goldfish, Mephisto, God Mode, engines | done | Separate Rookie/Easy/Medium/Strong selections persist and start a new series; `godot/tests/interaction.gd` checks both seats and languages on Godot 4.4.1. The mode selector (individual levels, Goldfish, Mephisto, God Mode) persists in `[game] mode`, restarts the series after confirmation and discloses the mode in a note; God Mode is labelled apart from the open display and its note states that every card, the Skat included, is visible and that the coach sees them too. Learned engines and non-level modes exclude each other (as `SessionConfig::validate` requires). `opponent_mode_opening_views_match_skat_app` and the slow `opponent_modes_match_skat_app_and_disclose_only_in_god_mode` show equality with the direct `skat-app` result and `open_cards` only in God Mode; `interaction.gd` checks selector, persistence, disclosure and disabled level selectors in both languages. At 1152 × 648 it also scrolls to both bidding actions and submits an action in Levels and God Mode, in both languages. The engine selector (Classic, Neural, PrePlay, Hybrid; Rookie maps to Easy for learned engines) persists and travels over the wire (`engine_selection_travels_over_the_wire_and_classic_is_omitted`); `interaction.gd` checks the selector, its persistence and tooltip in both languages on Godot 4.4.1. |
| Card faces (SVG) and German/English texts from `assets/i18n/` | open | `godot/i18n.gd` reads `assets/i18n/*.ftl`; `godot/tests/interaction.gd` checks language switching, labels and a seeded series. Card faces (hand and trick) are loaded from `assets/cards/german`; selector messages, card selection and export bundling remain open |
| Live coach modes, warnings, proceed/reconsider | open | |
| Counting aid and quiz | done | The aid and quiz toggles persist in `user://settings.cfg` (`[assist]`) and are sent as `SetAssist` on every new session. The adapter records the `Retrieval` events of answer, help and skip in the existing learning-history file (`quiz_events_reach_the_persistent_learning_history`, `damaged_learning_file_is_left_unchanged`); `counting_aid_and_quiz_show_only_the_permitted_view` checks that aid and help equal the direct `skat-app` view, are withheld while a question holds the move and that game actions are refused. `godot/tests/interaction.gd` `assist_flow` runs in German and English on pinned Godot 4.4.1 in the CI `portable` job (`scripts/godot-headless.sh`, green on main at 8f22827). |
| Guided dialogues and Abstich question | open | Deliberate entry buttons open the `skat-app` guided dialogue (card-play follow-trick branches: orientation, check, candidate, deliberate reveal, the learner's own card, observation) and the live Abstich question (select, Possible/Certain/Absent, help, skip, return). The adapter exposes `guided()` and `live_abstich()`; `guided_dialogue_runs_to_observation_with_the_permitted_view_only` and `live_abstich_question_matches_the_direct_session_and_hides_evidence_until_answered` compare every command, event and view with a direct `Session` and check permitted-view containment, hidden answers and stale refusal. `interaction.gd` `dialogue_flow` runs both in German and English on Godot 4.4.1 (`scripts/godot-headless.sh`), with legal cards available during the dialogue and restored after the question. The Reizen dialogue (hold/name orientation, ceiling attempt, worked rule, retry, reveal) is worded too and offered at bidding decisions; `reizen_dialogue_matches_the_direct_session_and_stays_in_the_permitted_view` and `interaction.gd` `reizen_dialogue_flow` (German and English) verify it. Skataufnahme, Drücken and Ansage (`skat_pickup_discard_and_announce_dialogues_match_the_direct_session`, `interaction.gd` `preplay_dialogue_flow`), Kontra and Re with the offered-call exclusions (`kontra_and_re_dialogues_match_the_direct_session_and_never_call`, `kontra_dialogue_flow`), the lead and Null branches (`lead_and_null_dialogues_match_the_direct_session`, `lead_null_memory_figures_flow`), card memory after the worked answer and the separately requested model comparison behind the opt-in "Show figures" setting (`card_memory_and_model_comparison_match_the_direct_session`) are worded from the permitted views and compared with a direct `Session`; each Godot flow runs in German and English. The rendered DE/EN screenshots are still missing (`GODOT_DIALOGUE_SCREENSHOTS` needs a display; headless runs draw nothing), so the row stays open. |
| Score sheet, series result, series figures | done | `sheet_and_series_result_cross_the_wire_like_the_egui_view_model` and `cast_resolves_to_catalogue_keys_like_the_egui_view_model` compare the adapter with `skat-app`. `interaction.gd` `check_sheet` verified the sheet, ranking, figure names and portraits in German and English on Godot 4.4.1, headless and with a rendered display. |
| Save/open recordings, replay window, comments, review, hindsight | open | Finished current game saves to a new local file after a concealed-information warning; validated recordings open in a read-only window with action-by-action navigation. `recording_round_trip_exposes_only_the_human_historical_view` checks every action boundary and exclusive creation; `godot/tests/interaction.gd` exercises save, open and navigation in German and English on Godot 4.4.1. Comments, review and hindsight remain open. |
| Same-deal practice | open | The current deal can be practised and retried in isolation, then the unchanged series resumed. `same_deal_practice_parks_the_series_and_keeps_cards_private` checks the hand, closed view and return; `interaction.gd` exercises both languages. Selection of earlier recordings and comparison remain open. |
| Learning history, statistics, focused training | open | The counting-history overview shows the descriptive `skat-app` counts by topic and context. `quiz_events_reach_the_persistent_learning_history` checks the saved summary; `interaction.gd` exercises the overview in both languages. Other history sections and focused training remain open. |
| Settings persistence | open | The language selector persists in `user://settings.cfg`; `godot/tests/interaction.gd` verifies both languages after recreating the scene. The optional Kontra/Re checkbox and both opponent levels persist in the same file and start a new series after confirmation; `interaction.gd` checks both values, all four levels, both seats and both languages on Godot 4.4.1. The open-display checkbox persists the same way and shows all hands and the Skat from the permitted open view; `interaction.gd` checks both values and languages. The counting-aid and quiz checkboxes persist in the same file (`[assist]`). Other settings remain open. |
| Linux and Windows 11 export, launch smoke, performance | open | |

## Build and run

```sh
cargo build -p gui-godot            # target/debug/libgui_godot.so (gui_godot.dll)
godot --headless --editor --path godot --import --quit # register the extension on first use
godot --path godot                  # Godot 4.4+, from the repository root
cargo test -p gui-godot             # adapter contract tests, no Godot needed
godot --headless --path godot --script res://tests/interaction.gd # interaction smoke; a failed assert prints SCRIPT ERROR (exit code stays 0)
scripts/godot-headless.sh           # build, pinned checksum-verified Godot 4.4.1, DE/EN run; fails on SCRIPT ERROR (CI `portable`)
```

The `.gdextension` file loads the library from `target/`. Export presets,
packaged artifacts and target-platform smoke tests are open work; M9 stays
incomplete until every row above is `done` and the lower layers are verified
unchanged.
