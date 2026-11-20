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

Recording review (D-69): `skat-app/tests/replay_review.rs` checks the moments
against independent single historical comments and `skat_coach::moments`
(game order, decision/prefix/context mapping with forced decisions skipped,
legal alternative, navigation shows the cached historical comment), jobs
holding only the core-reconstructed permitted view and its view seed,
determinism and focus-only reselection, cancel/time limit/retry/budget/stale
results, hindsight kept out, all-pass and in-progress scope, and the
untouched live session and file. Its ignored production-budget case runs the
fixtures with the real limits. `gui-egui/tests/replay_review.rs` drives
request, cancel, continue, moments, navigation, hindsight, close/reopen,
practice start and a saved in-progress file in both languages. Latency and
cancellation:
`cargo run --locked --release -p skat-app --example replay_review_latency`.

### Combined M7 acceptance

`skat-app/tests/closed_evidence.rs` pins a public observation on the late
Herz fixture (the human defends in Hinterhand): the declarer's S9 on CA in
trick 3 proves a Kreuz void, taught as the proven-void lesson at the human's
trick-4 card and as that decision's review moment; at trick 2 no void exists,
and the declarer's Pik void (H9 on S8, played after the human's trick-4
card) appears only at the trick-6 lead. Trumpf counts 6/6/3 follow from the
documented deal. For every analysed card decision of all six fixture
variants (declarer and defender) the stated voids and Trumpf count equal an
independent public-trick deduction, and a void or Trumpf lesson assigns no
concealed card. Open-display recordings yield the same review jobs, seeds
and results as their closed twins. Review moments keep the D-58 role,
metric sign, noise and provisional rules as declarer and defender.

`gui-egui/tests/replay_review.rs` drives the combined flow in both languages:
the open display chosen in the new-series dialog (not God Mode), a game
played by clicks and saved, the next live game waiting, reopening, trick
navigation back and forth, an exact recorded decision reached back and
forth, its historical comment equal to the application's closed-view
comment, the whole review with every frame timed while it works, reveal and
hindsight that change neither review nor historical comment, replacement by
another open recording while a comment is requested, that recording's
proven-void lesson, and a same-deal practice attempt that keeps the open
display, deviates and returns. Live view, recordings, both files and the
score sheet stay unchanged; the live decision stays playable. The ignored
`frames_stay_responsive_while_the_production_budget_review_works` times the
frames of a production-budget review. Measured locally (test profile):
small budget 36 frames per language while the review worked, slowest about
3 ms; production budget about 350 frames, slowest about 10 ms, review about
0.8 s for 7 analysed decisions. These are fixture measurements, not latency
guarantees; whether a superseded comment was still running at a GUI
transition depends on timing, so deterministic stale-result rejection is
pinned by the application tests. These runs do not establish human comprehension
or transfer. Successful Windows CI suffices for Windows verification (D-36).

### Learning history (D-70)

`skat-app/tests/learning.rs` records the actual
retrieval events of seeded sessions, saves and reopens them, and checks the
summary, deduplication of repeated batches and reloads, distinct series,
practice and retry attempts with coinciding question IDs, help and visible
aid never counted as unaided, empty and reset histories with the kept
attempt counter, and malformed, unsupported, inconsistent and oversized
files reported, unchanged and set aside without replacing an earlier one.
`skat-app/tests/training.rs` replays the four training fixtures move by move
through the rule engine and compares them with facts frozen by hand
(seat, role, game, tricks, hand, expected count and its parts) and with
oracles over the full deal and the public tricks; it checks both roles,
Farbspiel and Grand per family, the void boundary one card earlier, Null
without a Trumpf question, the answer/help/skip/retry/next/topic contract,
deterministic replays, training attempts kept apart from series and
practice with duplicates and context mismatches refused, reset, lossless
migration of a literal format 1 file and unchanged invalid or newer files.
A unit test swaps unseen cards of one follow class between the two other
hands and requires identical views, help, feedback and events.
`skat-app/tests/learning_responses.rs` checks format 3 provenance against
hand-written expectations: exact, too high, too low, unrecorded legacy
mismatch and skips with and without help per topic and context, empty and
sparse histories, window boundaries at 19, 20 and 21 settled responses with
positions, attempt ranges and contexts, windows over interleaved attempts,
duplicates, reloads and reset, refused missing or inconsistent provenance,
literal format 1 and 2 files migrated unversioned and written back
unchanged in shape, and files with impossible provenance kept unchanged.
`skat-app/tests/learning.rs` additionally checks that real session and
training events carry their lesson revision, that only answers carry the
exact count, and that a pending question keeps its lesson when the question
set changes.
`gui-egui/tests/training.rs` runs the training through the real widgets in
both languages and returns to an identical live table.
`gui-egui/tests/learning.rs` answers and skips questions through the real
widgets, restarts on the same file, reads the window, cancels and confirms a
reset, and keeps a damaged or newer file until it is set aside, in both
languages; it reads exact category cells, window labels, the sparse note and
provenance lines of a hand-built legacy-plus-new history in de/en.
Screenshot scenarios `learning-*` show the empty, recorded,
reset-confirmation and damaged states, `learning-categories(-en, -narrow)`
a format 2 file extended by real answers and `learning-compare(-en)` the
earlier/recent windows; `training-*` show the topic choice,
both families and roles, help, retry, feedback, the boundary, the learning
history with training rows, a narrow view and the return to the live table.

## Screenshots of the frontend

`cargo run --locked -p gui-egui --example screenshots -- OUT_DIR
[SCENARIO...]` renders named scenarios (bidding, trick, complete trick,
result, …) to `OUT_DIR/<name>.png` without a display (D-45). It needs a
wgpu adapter and prefers software ones: lavapipe on Linux (Fedora/Ubuntu
`mesa-vulkan-drivers`), WARP on Windows.

The `coach-jump-context` and `coach-jump-context-en` scenarios reproduce
seed 1020, game 2, the human call 20 followed by a jump to 35. The shared
`qa146` widget regression checks that the lesson labels the historical
pre-decision floor 22 in both languages while the actual bid/action is 35.

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
  a recorded own decision (its review reported unavailable).
- `review-running`, `review-cancelled(-en)`, `review-moments(-en)`,
  `review-moments-narrow` and `review-navigate(-en)` request the recording
  review of the reopened game, cancel it (partial scope), let it finish with
  its moments and open a moment's decision with its historical comment.
- `table-*` show the modern table of #134 (D-72) with series seed 1116,
  whose figures carry the longest names of both pools: the human bids,
  discards from twelve cards, announces, plays tricks, sees the result and
  the series end. Suffix `-1600` renders at 1600 × 1000, `-960` at
  960 × 640, the rest at 1280 × 820; `table-augen(-960)` holds the reveal
  button down, `table-large-text(-en)-960` enlarges every text style by
  30 %, `table-zoom-1280` uses egui zoom 125 %, and `table-en-960`,
  `table-german-deck-960`, `table-french2-en`, `table-mephisto-960`,
  `table-goldfish-960`, `table-god-960`, `table-open-960`,
  `table-neural-960` and `table-coach-off-960` cover languages, faces and
  every table mode. Real widget tests for the concealed Augen, the figures
  and the language switch are in `crates/gui-egui/tests/table.rs`.
- `open-display-new-series(-en)`, `open-display(-en)`, `open-display-trick`
  and `open-display-coach` choose the open display in the new-series dialog
  and show every hand and the Skat on the ordinary table during bidding and
  card play, with the display and the closed-view coach labelled apart.
- `open-display-proven-void(-en)` reopen an open-display recording at the open
  live table and show the historical comment on the human's trick-4 card with
  its proven-void lesson (only the trick-3 evidence).
- They support the independent agent review of a frontend change; they do not
  replace looking at the change in a real window.

The guided-dialogue scenarios `guided-orient`, `guided-act`, `guided-memory`,
`guided-observe`, `guided-complete` and `guided-figures` (plus English variants
and `guided-orient-narrow`) use seed 21, free play and two Easy opponents without
Kontra/Re. They enter through real widgets. The ignored bilingual `guided`
widget test also opens/closes a real recording, cancels the old dialogue and
re-enters with a fresh identity. The Reizen scenarios `guided-bid-orient`,
`guided-bid-threshold`, `guided-bid-act` (plus `-en` variants and
`guided-bid-act-narrow`) enter the human's first bidding decision with the same
seed through real widgets; the ignored bilingual `guided` Reizen widget test
covers wrong/retry, reveal, a free alternative call and skip/re-entry.
The scenarios `guided-pickup-orient`, `guided-pickup-threshold`,
`guided-pickup-act`, `guided-discard-threshold`, `guided-discard-act` (plus
`-en` variants and `guided-discard-act-narrow`) reach the human's Skataufnahme
and Drücken by holding/bidding with the same seed; the ignored bilingual
`guided` pre-play widget test covers hidden answers, skip/re-entry, retry,
reveal and a free alternative Hand/pickup.
The Ansage scenarios `guided-ann-orient`, `guided-ann-threshold` (a wrong
value attempt), `guided-ann-act`, `guided-ann-alternative` (Grand announced
instead of Easy's Kreuz), the `-en` variants and `guided-ann-act-narrow` use
seed 7: the human holds/bids to 36 and plays Hand; `guided-ann-pickup-act(-en)`
use seed 21 with a pickup. The ignored bilingual `guided` Ansage widget test
covers both seeds: hidden answers, skip/re-entry, wrong/retry, correct attempt
or reveal, rendered value comparison and candidate, and a free alternative
announcement actually applied.
The Kontra/Re scenarios switch Kontra/Re on: `guided-kontra-orient`,
`guided-kontra-threshold`, `guided-kontra-act` (correct attempt) and
`guided-kontra-declined` (the human plays a card, Easy said Kontra) use seed
18, `guided-kontra-reply` seed 0 (the human says Kontra after a reveal, the
declarer replies), `guided-re-orient`, `guided-re-threshold`, `guided-re-act`
(reveal) and `guided-re-resumed` (the human says Re, play resumes) seed 1,
each with `-en` variants, plus `guided-kontra-act-narrow` and
`guided-re-act-narrow`. Grand and Null (#155): `guided-kontra-grand-threshold`,
`guided-kontra-grand-act` (correct "no") and `guided-kontra-grand-reply` (the
human says Kontra, the declarer replies) use seed 848, first game;
`guided-kontra-null-threshold`, `guided-kontra-null-act` (correct "does not
apply") and `guided-kontra-null-declined` (the human plays a card) seed 1282,
first game; each with `-en` variants and `-act-narrow`. The Null ouvert round
exclusion `guided-kontra-fallback(-en)` uses seed 848, second game. The ignored
bilingual `guided` Kontra, Re and Grand/Null widget tests drive the same seeds
through real widgets. No automatic opponent says Kontra in a non-Ouvert Grand
or Null, so the human's Grand/Null Re reply is covered by core-replayed app
fixtures only, not by an ordinary-app walkthrough or screenshot.
Grand card play (#157, Kontra/Re off): `guided-grand-orient`,
`guided-grand-act` (correct non-Easy winner, worked answer CT) and
`guided-grand-observed` (the losing C8 and the actual trick) use seed 19,
second game, with the human defending last; each has an `-en` variant, plus
`guided-grand-act-narrow` and the honest fallback `guided-grand-fallback(-en)`
at a later decision of that game. `guided-grand-declarer-act(-en)` (seed 2,
first game) reveals Easy's HA for the human declarer. The ignored bilingual
`guided` Grand widget test drives the same paths.
Grand `UnderPartner` (#159): `guided-grand-under-orient`,
`guided-grand-under-act` (worked answer H7 revealed) and
`guided-grand-under-observed` (the non-Easy H9 and the declarer's actual HQ)
use seed 119, third game, sixth trick, with the human defending in Mittelhand;
each has an `-en` variant, plus `guided-grand-under-act-narrow` and
`guided-grand-under-observed-narrow` at 960x640. The route comes from the
bounded skat-app seed search (`guided-dialogue.md`). The shared widget and
screenshot route helper distinguishes the public feeding orientation from
`UnderPartner`, passing earlier supported `FeedPartner` decisions without
revealing a rule answer. The sixth-trick H8/H7/H9/HQ witness stays fixed.
These are technical checks, not learner UAT.

## Platform verification (M2)

The desktop app has been played interactively on Linux/Wayland. CI builds
and exercises the headless GUI on Linux and Windows Server. Successful Windows
CI is sufficient Windows verification (D-36); no separate Windows device or
CEO interactive acceptance is required. This does not claim an observed
interactive Windows 11 session. Agents own functional and visual checks and
inspect actual results, including any model-assisted screenshot findings.
Platform smoke checks exercise launch, a series through to the score sheet,
languages and card faces.

## Release 1.0 validation

Once release features and packages are ready, agents execute the required
functional, package and visual checks in the [release test plan](release-test-plan.md),
fix findings and repeat affected checks. Green complete CI on both platforms,
one fresh independent review and protected PR integration remain required.
Successful Windows CI suffices; no CEO Windows/manual/UAT/learner test is a
development or release gate (D-36, D-61). Human participation is considered
only for a concretely necessary test demonstrably impossible otherwise.
Optional [learner research](coach-validation.md) remains unperformed until
actual observations exist; technical fixtures prove only their tested claims,
not comprehension, learning efficacy or transfer. Missing observations do not
block progress or release. Publication authorisation is separate.

### Catalogue focus sessions (D-73)

`skat-app/tests/focus.rs` exercises independently specified count/judgement
responses across all four families, answer-before-feedback, the possible/certain
counterexample, boundaries, unsupported counts, unknown IDs/revisions, durable
unanswered exposure, retries, sticky assistance, deduplication, reset and lossless
formats 1/2/3 migration. The original catalogue oracle and hidden-substitution
witness tests remain unchanged. `gui-egui/tests/focus.rs` exercises the actual
German/English widgets through answer/help/skip/retry/next/exit and restart;
compares live game and recording before/after, and preserves corrupt/newer files.

Screenshot scenarios `focus-count`, `focus-changed`, `focus-help`,
`focus-mechanism`, `focus-feedback`, `focus-certain`, `focus-boundary`, `focus-practice` (and
`-en`) and `focus-void(-en)-narrow` show the actual session states. The existing
Linux/Windows screenshot CI renders them and records individual PNG SHA-256
hashes alongside the gallery. These are technical checks; optional learner
observations remain uncollected, and release publication requires separate
authorisation.

## Mechanism-response evidence (#13)

`crates/skat-app/tests/mechanism_responses.rs` independently specifies the
complete possible/certain/absent answer matrix and unsupported inputs, legal
catalogue positive/negative/uncertain responses, mixed help/skip/unanswered
opportunities, repeats, dedup, restart/reset, incomplete exposure and
19/20/21-settlement windows. Counting summaries remain unchanged.
`crates/gui-egui/tests/learning.rs` asserts the actual German/English
mechanism labels, denominators, sparse fallback and window category cells.
The screenshot scenarios `mechanism-responses`, `mechanism-trends`, their
`-en` variants and `mechanism-trends-narrow` show the changed overview;
`learning-empty`/`learning-empty-en` cover its empty state. Both platform CI
galleries produce individual SHA256SUMS. The terminal owner verifies final
technical platform evidence. Human learning evidence remains unobserved
optional research, not a release gate.

## Reserved validation technical checks (#54)

`skat-app --test validation54` exercises both reserved sets without rendering
or printing their permitted views: first answers, exact equality, external
exposure, sticky help/repeats, unclear/skip/stop, actual timestamps, format 4
migration and forged versions/provenance. Catalogue unit tests use handwritten
facts, independent core positive/negative witnesses and concealed-hand/Skat
substitutions. These are technical inputs, never learner observations.

`gui-egui --test focus` drives actual de/en entry, hidden feedback, verbatim
fields, help/retry/skip/unclear/exit and durable history. The only new gallery
scenarios are `validation-entry[-en]` (no position) and `validation-form[-en]`
(the same form with an explicitly labelled **public** practice case).
Never add a reserved position to a gallery. `scripts/validation54_kit_test.py`
checks source/build mismatch, immutable task hashes, separate blank records,
withheld key and refusal to overwrite existing personal data. The same RC
packager verifies the installed executable's notices/resources and hashes.
Packaged lifecycle checks remain required technical work; successful Windows
CI suffices for Windows verification. Actual human Initial/Delayed sessions
remain optional research, unperformed until observed; this suite cannot
supply human responses or learning evidence.

## Grand WinEarly verification (#161)

`skat-coach/tests/guided_win_early.rs` preserves the frozen handwritten A–E
expectations: ten case/role combinations, 16 named winners, 58 legal completion
witnesses, equal permitted-view pairs, extra unseen substitutions and
exclusions. `skat-app` tests named `grand_win_early` cover consent, hidden
pairs/God Mode, free play, actual replies and lifecycle boundaries.
`gui-egui/tests/win_early.rs` is an ignored bilingual ordinary-widget test;
run it explicitly with `cargo test --locked -p gui-egui --test win_early --
--include-ignored --nocapture` in the development container.

Screenshot scenarios `guided-grand-early-{orient,act,observed}` each have an
`-en` variant. Action and observation also have `-narrow` and `-en-narrow`
variants at 960x640; desktop is 1440x1000. All replay the frozen seed0 ordinary
route through real widgets, MH declarer only. Side-master and defender roles
are core/app fixture evidence, not ordinary renders. Main regression,
independent review and both-OS archive/hash/visual acceptance remain required
technical checks. Optional human research supplies no results until observed.

## Null last-seat dialogue verification (#165)

`skat-coach --test guided_null_last` ports the independently frozen #165
oracle and separately labelled addendum. App tests matching `null_last`
exercise actual new lessons, every legal fixture/ordinary completion,
privacy, consent, return and stale/reset boundaries. Run the ignored GUI
suite explicitly with `cargo test --locked -p gui-egui --test null_last --
--include-ignored --nocapture` in `skat-dev` (lavapipe). It covers both
languages/sizes and every legal card on the ordinary seed6 route, plus
explicit fixture components.

The screenshots example includes forty `guided-null-last-*` ordinary views
and thirty-two `guided-null-fixture-*` component views. Names and evidence
boundaries are in [the walkthrough](guided-dialogue.md#null-declarer-last-to-the-current-trick-165).
The fixture JSON is checked against actual Session output; it is not the
handwritten domain oracle or ordinary-route evidence. Local inspection must
cover every new PNG without cropping, scaling or repairing source pixels.
Both full OS archives, their API digest/size, exact sets and every file hash,
merged-main regression and personal cross-platform inspection remain the
terminal delivery owner's checks.

### Null middle-seat dialogue (#169)

`null_middle_policy` (skat-analysis, default features) freezes QA1591's
corrected legal oracle; `guided_null_middle` (skat-coach) checks scope,
whole non-absent lessons and privacy. App `null_middle` tests replay every
admitted candidate/continuation, full hidden-substitution dialogue models,
ordinary seeded recording reconstruction and actual individual winners.
The ignored `gui-egui --test null_middle` tests exercise ordinary widgets,
all legal alternatives, both languages and 1440×1000 / 960×640.

The existing all-scenarios screenshot example includes all new ordinary
stages and explicitly labelled boundary components on each OS. Its
`null-middle-inventory.json` is generated from the same source that selects
views, with name, stage, kind, language and exact dimensions. Names beginning
`guided-null-middle-` are new; existing #165 scenarios are retained without
duplication. App-emitted component models live in
`crates/gui-egui/tests/fixtures/null-middle.json` and are compared byte for
byte with checked session output; they are neither an ordinary route nor
the independent oracle. Personal PNG inspection and final two-OS archive
verification are separate evidence, never inferred from a passing renderer.

### Grand lead dialogue (#171)

`skat-analysis --test lead_master_policy` runs with default features and
literal accepted1633 expectations (3,324 next-trick, 106 terminal
continuations). `skat-coach --test guided_lead_master` covers whole lessons,
role/master answers, malformed guards and two non-absent hidden-view pairs.
App tests matching `lead_master` check all legal completions, whole dialogs
including God Mode, ordinary seed3 commands, consent, stale identities,
memory, actual replies/winners and original-decision estimates.

Run the ignored GUI suite in skat-dev with `cargo test --locked -p gui-egui
--test lead_master -- --include-ignored --nocapture`. It exercises every new
ordinary stage and labelled fixture in both languages/sizes and every legal
ordinary card. The screenshots example retains the old inventory and adds
`guided-lead-master-*` plus `lead-master-inventory.json`, using the same source
for names and rendering. `lead_master_fixture_widget_models_match_checked_sessions`
checks committed component JSON; set `SKAT_LEAD_MASTER_WIDGET_FIXTURES_OUT`
to its absolute destination when intentionally regenerating those models.
Full push and configured non-GPU/include-ignored regression remain required;
local rendering does not establish both-OS archive/hash/visual acceptance.

## Null opening lead (#176)

The [bounded opening-lead dialogue](guided-dialogue.md#null-opening-lead-176)
uses the unchanged shared Null minimum-cost rule, FIRST exact tie. Only the
non-Ouvert Vorhand declarer's own first lead is admitted; Augen are legacy
sorting cost, not Null scoring. All legal leads remain available. Empty-table
candidate status never predicts either defender reply or completed winner.

Default-feature `cargo test -p skat-analysis --test null_lead_policy` checks
accepted1695's complete literal inputs/all eleven fields and all 286 first
tricks (94 immediate Null losses); `skat-coach --test guided_null_lead` and
app tests matching `null_lead` cover full non-absent lessons/dialogues, both
hidden substitutions including God Mode, legal alternatives, actual two replies,
individual winner, consent, stale/reentry/replay/reset and original estimates.
Run `cargo test -p gui-egui --test null_lead -- --include-ignored --nocapture`
in skat-dev for ordinary widgets and labelled component fixtures in both
languages/sizes. `SKAT_NULL_LEAD_WIDGET_FIXTURES_OUT` intentionally regenerates
the component JSON from checked app sessions; ordinary renders never inject it.
Source-derived `null-lead-inventory.json` joins the existing both-OS gallery.
Push gate and the configured workspace non-GPU/include-ignored suite remain
required, including nn-train and vulkan-search. Terminal owns review and final
actual-main/both-OS archive/hash/visual acceptance. No learner results follow.

## Farbspiel WinEarly verification (#178)

`cargo test --locked -p skat-analysis --test suit_win_early_policy` runs with
project default features. The cloud-owned literal module checks full legal
histories, PlayerViews, ten Knowledge fields, eleven CardRule fields,
ordered partitions/selection and all 356 third-card completions. The original
and sole supplement provenance is in the [walkthrough](guided-dialogue.md#farbspiel-winearly-178).
`skat-coach --test guided_suit_win_early` checks the 22 named complete API
inputs, full non-absent lessons, conditions and both positive hidden pairs.
App `suit_win_early` tests cover complete dialogue privacy including God Mode,
all legal choices, actual replies/winners, consent, stale/reentry/reset,
recording reconstruction and separately requested original-decision estimates.

The ignored `gui-egui --test guided_suit_win_early` drives ordinary widgets
and labelled fixture views in both languages/sizes. The screenshots example
emits `suit-win-early-inventory.json` and includes its exact names in the
existing all-scenarios inventory; both-OS CI collects these through the
existing full render job. Render only the new names for author inspection.
The minimum-size long explanation scrolls below the always reachable return.
These automated checks establish no learner or release acceptance.

## Farbspiel last-seat FeedPartner regression (#185)

`cargo test --locked -p skat-analysis --test suit_feed_partner_policy` uses the
repository lock and default features. Its accepted literal fixture checks full
legal histories/views, Knowledge, all eleven rule fields, partitions, last ties,
37 completions, four terminal paths and source-informed API mutations.
`guided_suit_feed_partner` in coach and `suit_feed_partner` in app cover strict
admission, whole non-absent lesson/dialogue equality over both hidden pairs,
God Mode privacy, original frozen estimates, consent, lifecycle and all legal
choices. Existing family regressions and Easy frozen hashes remain binding.

The ignored GUI target `guided_suit_feed_partner` exercises ordinary widgets
and labelled component views in both languages and sizes. The established
screenshots example emits the source-derived `suit-feed-partner-inventory.json`
and includes its PNGs in `all-scenarios-inventory.json`; both-OS CI runs the
example without a filter. Fixture JSON is checked against actual app sessions by
`suit_feed_partner_fixture_widget_models_match_actual_sessions`. Regenerate only
those technical models with `SKAT_SUIT_FEED_WIDGET_FIXTURES_OUT` set to their
absolute path and that test; this never changes the accepted handwritten oracle.
See the [scope and ordinary walkthrough](guided-dialogue.md#farbspiel-last-seat-feedpartner-185).

## Farbspiel LeadMaster regression (#182)

`cargo test --locked -p skat-analysis --test suit_lead_master_policy` holds five
handwritten legal deals plus an independent legal-witness oracle written from
the ISkO rank/Augen tables only (no `skat-analysis` code, no core trick
evaluator). It re-derives candidates, Easy's pick and the winner of every legal
next-trick completion (1,803 paths; the core is used only to enumerate legal
actions). Per-case counts and digests are frozen; they were calibrated from the
oracle, so they are source-informed regression guards, not CEO-independent
evidence. `guided_suit_lead_master` in coach covers admission and boundary
edits. App tests matching `suit_lead_master` drive every completion through
`Session` (waiting for both replies, individual winner), the genuine ordinary
seed 2969 route against Easy with all ten legal leads, both hidden-deal
substitutions with and without God Mode, return/stale/reset and the original
estimate contract (the same as Grand `LeadMaster`; Farbspiel `LeadMaster` has
no permitted-only estimate hold).

The ignored GUI target `guided_suit_lead_master` runs the same ordinary route
through real widgets in both languages and at 1440x1000 and 960x640. The
screenshots example emits `suit-lead-master-inventory.json` (eight stages per
language and size) and includes the PNGs in `all-scenarios-inventory.json`.
See the [scope and evidence](guided-dialogue.md#farbspiel-lead-with-a-follow-class-master-182).

## Learned engine selection (#77, increments A/B)

`skat-app` engine lifecycle tests cover Classic JSON/default compatibility,
invalid special-table combinations, frozen identity and format-2 round trips,
legacy format-1 Neural practice, model mismatch, cancelled legal fallback through
a two-game series and stale worker rejection. Existing Classic tests are retained.
Tiny fixed-model tests check temperature repeatability/legal support, Strong
greedy ties, finite-extreme masking and matching Classic fallback through a
complete seeded series. App validation rejects Neural Rookie; legacy profiles
stay readable. The calibrate parser tests inspect shared profiles without arenas.
The egui headless test switches engines and levels independently in de/en, preserves draft levels,
checks Cancel with a pending worker and checks Start cancellation/stale results.
Screenshot scenarios `neural-selection`, `neural-selection-en`, `neural-fallback`
and `neural-fallback-en` show the controls and live notice; the `-narrow` variants
also check the 960 × 640 minimum; both now include `-en-narrow`. Fallback scenarios
require a process started with an unavailable/corrupt model override. These are
technical lifecycle checks, not learned-strength or human-learning evidence.

## Explicit live Abstich observations (#13)

The [live question contract](live-abstich.md) fixes independent Possible/Certain
fixtures, positive-risk selection limits, immutable permitted-view provenance,
unanswered/skip/help semantics and the separate persisted overview. App unit
tests exercise actual Session commands and migration/reset/deduplication;
`crates/gui-egui/tests/live_abstich.rs` drives DE/EN widgets and persisted restart.
PR and native tiers explicitly retain these bounded widget tests despite their
slow-test annotation. The screenshot example includes the `live-abstich-*` entry, candidate, answer,
help, skip, unsupported, return and overview views, including narrow answers.
Compare exported live recordings before/after these interactions to check
nonmutation. No unchanged RC/gallery campaign or learner observations are implied.
