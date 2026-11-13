# Testing and verification gate

How Grand-Coach makes sure nothing with failing checks reaches `main`
(D-30). One script, `scripts/check.sh`, defines the checks; git hooks, CI
and agents only call it, so they cannot drift apart.

## Enforcement layers

| Layer | Runs | Tier | Binding? |
|-------|------|------|----------|
| Claude Code hooks (`.claude/settings.json`) | agent sessions, local and CI | – | activate the git hooks; block `--no-verify` and changing `core.hooksPath`; in CI a Stop hook keeps the agent working until everything is committed and `check.sh push` is green |
| `pre-commit` (`.githooks/`) | every commit | `quick` | local feedback; can be skipped by humans |
| `pre-push` (`.githooks/`) | every push | `push --base` | local feedback; can be skipped by humans |
| `implement-issue.yml` | after the agent, before the PR | `full --base` | red gate → draft PR with a warning |
| CI `ci.yml`, job `ci-ok` | every PR and push to `main` | `lint`; code: Ubuntu `pr`, Windows `native`, MSRV; main adds `regression`; nightly/manual adds `complete` on both platforms | **required status check on `main`; the only hard gate** |
| CI `ci.yml`, job `deep` | weekly and on demand | `deep` | scheduled-run failure mails the maintainer |

Hooks are fast feedback, not security: anyone can skip them. Merge
protection rests on the required check `ci-ok`, which also covers the
agent in GitHub Actions, where no developer PC exists. Changes to the gate
itself (`scripts/check.sh`, `.githooks/`, workflows) are visible in review;
`implement-issue.yml` runs the gate from the base commit so the agent's
branch cannot weaken it.

Activate the git hooks once per clone (Claude Code does this automatically):

```sh
git config core.hooksPath .githooks
```

The hooks check the working tree, not exactly the staged or pushed
snapshot; CI checks the real commit.

## Tiers of `scripts/check.sh`

- `lint` (seconds, no compilation): whitespace, SPDX headers, rustfmt,
  markdownlint (`.markdownlint-cli2.yaml`), offline Markdown links and their
  positive/negative fixtures, shellcheck, actionlint.
- `complete` (alias `test`): clippy of the workspace with `--all-features`, clippy of
  each package with features without them, and every test including the
  slow ones (`--include-ignored`) with all features except GPU backends,
  all `--locked`.
- `quick` (pre-commit): `lint`, plus clippy when code files are staged.
- `push` (pre-push): `lint`; when code files changed since REV, clippy and
  the complete FAST workspace (conservative dependency closure).
- `full` (default; agents and `implement-issue.yml`): `lint` and `test`,
  for one complete platform. With `--base REV`, `push` and `full` skip compiling and testing
  when no code file changed since REV.
- `deep`: in release mode, `SKAT_RANDOM_GAMES` seeded random games
  (default 1,000,000) and `SKAT_MEPHISTO_GAMES` games with Mephisto on
  every seat (default 100).

**Slow tests.** A test that takes 2 s or more (measured alone,
`--test-threads=1`) gets `#[ignore = "slow: CI only (docs/testing.md)"]`.
Hooks and the FAST workspace skip it. Main Ubuntu regression and nightly,
manual and RC complete suites on both platforms run every ignored case.
No test or assertion is removed or reclassified by this change. Randomized
core tests (10,000 games) still run in FAST; ignored analysis/AI strength,
GUI full-series/variant/coach-soak cases move to those complete owners.
The ignored AI fairness and app-series async/stale-move/privileged tests remain
mandatory in Ubuntu PRs. Anything that takes minutes belongs in `deep`, with
a small default run in `complete`.

**Features.** One `cargo test --workspace` with every feature except GPU
backends covers everything: `skat-app` enables `serde` in every crate
anyway (feature unification), so a workspace run without features would
execute the same binaries. Without `serde` only derives and wire types are
missing, so the `test` tier checks that configuration with clippy per
package. If a feature ever changes behaviour, that package's tests also
run without it. GPU backends (`nn-train`'s `vulkan`) stay off in tests:
they need a GPU adapter, which the Linux runner lacks, they would make
Burn's default device a GPU in every crate (unification), and they double
the compile time. Clippy still checks them with `--all-features`; GPU
training is verified by hand (`nn-train smoke vulkan`).

## PR and platform routing

- `pr`: clippy all-target/all-feature, feature-off clippy, complete FAST
  workspace with the existing CPU test feature set, plus ignored
  `skat-ai --test fairness` privacy and `skat-app --test series` integration. No package filtering. Fixed rules,
  scoring, permitted-view and coach/app regressions run before merge.
- `native`: `cargo check --workspace --all-targets --all-features` and
  per-feature-package `cargo check --all-targets` without features, then
  selected CPU tests and a required headless `launch` render. No Windows
  clippy or duplicate full portable suite in a default PR.
- `regression`: complete workspace tests including ignored tests, without
  repeating clippy. Every main code push owns this broader Ubuntu regression
  alongside prompt FAST integration and Windows native validation. A failure
  needs immediate agent triage, not deferral to release.
- `route --base REV --event EVENT`: conservative classifier; `ci-ok` consumes
  `CI_NEEDS` JSON and strictly checks every mandatory job result. Python 3
  and Git are required. `lint` runs real temporary-repository routing and
  aggregation fixtures (`python3 scripts/ci/test_route.py`).

Only `.md` under `docs/` and root README/CONTRIBUTING/AGENTS/CLAUDE are
allowlisted as docs-only. Every other path (including build.rs, .cargo,
all manifests/lockfiles, toolchain, assets, scripts, hooks, workflows and
unknown files) runs complete FAST plus native. Deleted and renamed paths
are checked on both sides without rename detection; local tracked changes
count too. Empty diffs and unknown bases conservatively run code checks.
Diff/decoding/classifier errors fail the workflow, never produce docs-only.
Hooks use the same classifier; on classification errors they compile/test.

`changes` emits exactly five boolean strings. `ci-ok` rejects missing/invalid
or inconsistent routing, classification failure, and any mandatory failure,
cancellation or unexpected skip. Only documented inactive jobs may skip.
Windows render runs for every code change inside mandatory `native`; a
missing/empty PNG fails it. The informational full gallery is limited to
frontend/assets changes or unknown-base/full runs.
Protection and the required name `ci-ok` are unchanged.

Nightly schedules and every manual dispatch ignore diffs and run complete
suites on **both** platforms, plus PR/native/MSRV checks. Dispatch CI on the
RC ref before release (`deep=true` additionally runs the deep soak). Monday's
nightly also owns weekly deep; the other six nights run complete platforms.
The deep command keeps its million-game/100-Mephisto defaults and GPU
exclusion policy. `ci-ok` also requires complete/deep results when selected.

### Exact critical and native inventory

FAST includes all non-ignored tests, including `skat-core` targets bidding,
kontra, play, scoring, variants and views; `skat-nn` encoder/model;
`skat-ai` fairness/trained_model; all `skat-coach` targets and `skat-app`
assist/coach/series (M6 count privacy included). Additionally the PR tier
runs `series::opponent_jobs_match_advance_and_reject_stale_moves` and
`series::mephisto_mode_sees_all_cards_where_configured` despite their
existing slow markers. The PR also runs ignored `skat-ai --test fairness`
cases `goldfish_is_fair_and_covers_every_phase`,
`medium_is_fair_and_covers_every_phase` and
`strong_is_fair_and_covers_every_phase`. Whole-workspace execution covers
transitive dependents.

Native Windows executes all non-ignored `skat-nn --test model` and
`skat-ai --test trained_model` cases (disk manifest/hash, legal decisions,
permitted views and fallback); `nn-train --test data` exact cases
`the_cache_returns_exactly_the_loaded_data` and
`cache_files_keep_wide_features` (temp-file roundtrips); `gui-egui --test gui`
exact cases `launch_shows_the_new_series_settings`, `every_card_face_renders`,
`language_and_faces_switch_without_touching_the_game` and
`free_play_ends_on_request`. The screenshots example renders `launch` using
WARP on Windows and validates a nonempty PNG; `native-render-Windows` retains
it. These GUI harness and backend checks do not certify a real native window.
App recording IO tests cover real temporary-directory save/reopen, capped growing
readers, malformed/unsupported/illegal/oversized files, missing/unwritable paths
and no-overwrite byte preservation. Bilingual egui recording tests exercise
file dialogs, navigation, recovery and live-session isolation. Full platform suites retain all remaining tests.

### Timing evidence

Each shared-script command prints elapsed seconds and exit status. For PR
comparisons record run/head/event, runner/toolchain, queue-start proxy,
checkout/setup, cache restore, clippy/feature passes, cargo-reported build,
test-binary durations, render and run-created-to-ci-ok separately. Restored
caches may still require rebuilding. Different scope/head/cache states are
not controlled A/B measurements; claim only observed changes, no promised
percentage. HQ223 baseline: PR 43109220476 reached ci-ok in 25:08 (Windows
critical); main 43041261025 in 10:42. Full-platform dispatch has broader
scope than a default PR and must be reported separately.

Linters missing locally are skipped with a warning; CI sets
`CHECK_STRICT=1`, where a missing linter fails. Local install (Fedora):
`sudo dnf install nodejs ShellCheck python3` (markdownlint runs via `npx`),
actionlint from its [releases](https://github.com/rhysd/actionlint/releases).

## Offline Markdown links

The lint tier runs `python3 scripts/markdown-links/test_check.py` and
`python3 scripts/markdown-links/check.py`. Both need only Python 3 (3.9 or
newer) and Git; Python 3 is available in `skat-dev` and the Ubuntu CI lint
runner. The fixtures create temporary Git repositories and assert success,
exit status 1 for broken targets, and source/line/target diagnostics. They
also cover maintained hidden skills/templates and untracked scratch files.
The existing missing-linter policy applies to Python too.

The checker reads working-tree contents of **all tracked `.md` files** from
`git ls-files -z`, including templates, skills and historical documentation.
Untracked files are neither scanned nor accepted as link targets. Stage new
documentation before checking it. Targets may be tracked files or directories
containing tracked files; root-relative paths start at the repository root.
No HTTP requests are made, even for external reference definitions.

Supported syntax:

- Inline links/images, angle-bracket destinations, balanced parentheses,
  optional titles, full/collapsed/shortcut references and reference definitions.
  Reference labels are case-insensitive and whitespace-normalised; undefined
  references are plain text, as in Markdown.
- Relative paths, `..`, fragment-only links, URL-encoded paths/fragments and
  query strings (ignored for local file lookup). Paths cannot escape the repo.
- ATX (`#`) and Setext headings; lowercase GitHub-style anchors with punctuation
  removed, spaces replaced by hyphens, and duplicate suffixes `-1`, `-2`, etc.,
  including collisions with explicitly numbered headings. Inline code and
  common emphasis/link formatting are stripped when computing anchors.
- Top-level backtick/tilde fences, indented code, inline code and HTML comments
  are ignored. URI schemes and protocol-relative external links are skipped.

Boundaries: this is a repository Markdown subset, not a complete CommonMark
or GitHub renderer. Nested list/blockquote code and headings, raw HTML links
or explicit HTML anchor IDs, extension-generated anchors and complex inline
HTML/formatting are not supported. Keep maintained links/headings in the
supported syntax or extend the checker with fixtures before introducing such
constructs. Fragments in non-Markdown targets (e.g. PDF pages) are not checked.
The checker proves local target existence, not network availability or semantic
correctness. No maintained Markdown file is excluded to suppress a failure.

## Test coverage

Fixed deals and seeded random games check legal play, settlement, replay
and seat isolation. Analysis fixtures compare against hand-computed results
and exhaustive minimax. AI arenas measure relative strength. Coach tests
cover lesson evidence, disclosure, warnings and cancelled or stale jobs.
Session and headless GUI tests play series through the public interfaces.
Model tests cover legal inference, fallbacks, shard replay, training and
checkpoint resume. The commands for [calibration](ai.md#calibration-protocol)
and [training](learned-models.md) exercise the standalone tools.

Recording tests supplement the fixed Farbspiel/Grand/Null legal-history
fixtures with file round trips, incomplete and all-pass records. Navigation
boundaries are checked against independently applied core events and seat
views. GUI checks retain the original score sheet, recording and coach view;
recording windows cannot apply moves and analyse only a requested replay comment. These checks do not
claim session resumption.

Practice tests (`skat-app/tests/practice.rs`) start attempts from a real
late-series game with a non-default dealer, reproduce the recorded game from
the same human choices (cards, seats, levels and seeds), check Goldfish, God
Mode, Mephisto and Kontra/Re configuration and visibility, complete and
abandon attempts with alternative legal choices without changing the live
view, recordings or saved file bytes, reject stale moves and coach results
across sessions and attempts, and recover from invalid/unsupported records.
GUI tests park and deliver pending live opponent and coach work, cancel and
disconnect practice work, and drive the German and English
viewer-to-practice-to-return flow through real widgets.

Replay-comment tests (`skat-app/tests/replay_coach.rs`, D-67) check the exact
pre-action view (hand, own Skat, Ouvert cards, bidding, current trick, no
later trick) against the documented fixture deal, a late-series dealer that
moves the human, the chosen action and legal alternative with role, seat and
basis, forced/cancelled/all-pass/in-progress honest status, and stale-result
rejection after every change and across identical or reopened recordings,
while the live session, its pending coach job and file bytes stay unchanged.
A `skat-app` unit test substitutes concealed cards under an identical
permitted view and gets identical historical jobs and comments; only
hindsight jobs differ. A `compile_fail` doctest keeps replay results out of
`Session::finish_coach`. `gui-egui/tests/replay_coach.rs` drives selection,
historical and hindsight requests, reveal, rapid changes, cancel/retry,
close/reopen and practice start in both languages. Latency:
`cargo run --locked --release -p skat-app --example replay_coach_latency`.

## Screenshots of the frontend

`cargo run --locked -p gui-egui --example screenshots -- OUT_DIR
[SCENARIO...]` renders named scenarios (bidding, trick, complete trick,
result, …) to `OUT_DIR/<name>.png` without a display (D-45). It needs a
wgpu adapter and prefers software ones: lavapipe on Linux (Fedora/Ubuntu
`mesa-vulkan-drivers`), WARP on Windows.

- CI job `screenshots` (Linux and Windows) runs all scenarios when
  `crates/gui-egui/` or `assets/` changed and keeps the PNGs as run
  artifacts `screenshots-Linux` and `screenshots-Windows`. It is
  informational, not part of `ci-ok`.
- `implement-issue.yml` embeds screenshots in the PR body of every
  frontend change: the agent's own (skill step "Screenshots"), otherwise
  all built-in scenarios. They live on the orphan branch `screenshots`
  under `issue-<n>/`.
- `recording`, `recording-en`, `recording-narrow`, `recording-error` and
  `recording-error-en` exercise actual menu save/reopen before rendering the
  detached viewer or a recoverable corrupt-file error with the prior viewer
  retained. The temporary files belong to each scenario. These headless runs
  do not establish real-window Linux/Windows file-dialog usability.
- `practice`, `practice-play`, `practice-play-en`, `practice-finished`,
  `practice-return` and `practice-return-en` start a practice attempt from
  that viewer, play other legal cards than the original, finish it or return
  early to the unchanged live series.
- `replay-before`, `replay-historical(-en)`, `replay-hindsight(-en)` and
  `replay-none(-en)` show a selected recorded decision before any request,
  its historical and labelled hindsight comments, and an early file without
  a recorded own decision.
- `open-display-new-series(-en)`, `open-display(-en)`, `open-display-trick`
  and `open-display-coach` choose the open display in the new-series dialog
  and show every hand and the Skat on the ordinary table during bidding and
  card play, with the display and the closed-view coach labelled apart.
- They support the independent agent review of a frontend change; they do not
  replace looking at the change in a real window.

## Platform verification (M2)

The desktop app has been played interactively on Linux/Wayland. CI builds
and exercises the headless GUI on Linux and Windows Server. Interactive
Windows 11 verification remains open. For a platform smoke test, launch,
play a series through to the score sheet and switch language and card faces.

## Release 1.0 validation

Once the release features and packages are ready, agents execute
the [release test plan](release-test-plan.md) on Linux and Windows 11,
fixes findings and repeats affected checks before publication. For coach
changes, automated fixtures establish the tested facts. Missing target-platform
evidence remains pending. The maintainer's
[learner sessions](coach-validation.md) assess comprehension and unaided
transfer. Manual validation takes place at release time (D-61).
