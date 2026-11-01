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
| CI `ci.yml`, job `ci-ok` | every PR and push to `main` | `lint`, `test` (including slow tests) on Linux and Windows, MSRV | **required status check on `main`; the only hard gate** |
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
  markdownlint (`.markdownlint-cli2.yaml`), shellcheck, actionlint.
- `test` (CI): clippy of the workspace with `--all-features`, clippy of
  each package with features without them, and every test including the
  slow ones (`--include-ignored`), all `--locked`.
- `quick` (pre-commit): `lint`, plus clippy when code files are staged.
- `push` (pre-push): `lint`; when code files changed since REV, clippy and
  the fast tests of the affected packages. Affected are the packages whose
  directory changed and every package depending on them (`cargo tree -i`);
  a change to any other code file (manifests, lock file, toolchain,
  `assets/`, the gate) affects all.
- `full` (default; agents and `implement-issue.yml`): `lint` and `test`,
  as in CI. With `--base REV`, `push` and `full` skip compiling and testing
  when no code file changed since REV.
- `deep`: in release mode, `SKAT_RANDOM_GAMES` seeded random games
  (default 1,000,000) and `SKAT_MEPHISTO_GAMES` games with Mephisto on
  every seat (default 100).

**Slow tests.** A test that takes 2 s or more (measured alone,
`--test-threads=1`) gets `#[ignore = "slow: CI only (docs/testing.md)"]`.
Hooks skip it, CI runs it on every code change. Anything that takes
minutes belongs in `deep`, with a small default run in `test`.

**Features.** One `cargo test --workspace --all-features` covers
everything: `skat-app` enables `serde` in every crate anyway (feature
unification), so a workspace run without features would execute the same
binaries. Without `serde` only derives and wire types are missing, so the
`test` tier checks that configuration with clippy per package. If a
feature ever changes behaviour, that package's tests also run without it.

**Code files** (changes trigger `test`): `*.rs`, `Cargo.toml`,
`Cargo.lock`, `rust-toolchain*`, `assets/`, `scripts/check.sh` and
`.github/workflows/ci.yml`. Docs-only changes run only `lint`. CI decides
this in the job `changes` instead of a `paths:` filter, so `ci-ok` is
always reported and never blocks a docs PR.

Linters missing locally are skipped with a warning; CI sets
`CHECK_STRICT=1`, where a missing linter fails. Local install (Fedora):
`sudo dnf install nodejs ShellCheck` (markdownlint runs via `npx`),
actionlint from its [releases](https://github.com/rhysd/actionlint/releases).

## Test levels

| Level | Here | Where | Tier |
|-------|------|-------|------|
| Static analysis | rustfmt, clippy, markdown, shell and workflow linters | `scripts/check.sh` | lint, test |
| Unit | functions and types inside a crate | `#[cfg(test)]` modules | test |
| Component (service) | one crate through its public API, fixed deals | `crates/*/tests/` | test |
| Interface / contract | serde round-trips, determinism (seed → deal, replay); `skat-app` commands, events, view models and errors as JSON | `skat-core/tests/serde.rs`, `skat-app/tests/series.rs` | test |
| Analysis | card counting, voids, visibility and hidden-card distributions on fixed positions; double-dummy against hand-solved positions and an exhaustive minimax; every decision type, information isolation, reproducibility, cancellation, Überreizen, Kontra/Re, Monte Carlo against exact enumeration | `skat-analysis/tests/` | test |
| Performance | latency and estimation quality of the analysis per decision phase ([analysis.md](analysis.md#latency-target-and-benchmark)); coach latency per human decision through `skat-app` ([coach.md](coach.md#latency)) | `skat-analysis/examples/bench.rs`, `skat-app/examples/coach_latency.rs` | manual |
| AI | strength profiles, information isolation by swapping concealed cards for every level and the Goldfish, reproducibility, fallback on cancellation and exhausted budgets, complete games, Rookie deviation rates, Goldfish forgetting, Mephisto never losing as declarer (100 games in `deep`); background opponent moves and table modes through `skat-app` | `skat-ai/tests/`, `skat-ai/src/arena.rs`, `skat-app/tests/series.rs` | test, deep |
| Calibration | AI-vs-AI strength ordering and decision latency per level ([ai.md](ai.md)) | `skat-ai/examples/calibrate.rs`, `skat-ai/examples/latency.rs` | manual |
| System | whole games end to end: 10,000 seeded random games against an independent rule model; complete seeded 36-game series with a random human, information boundary of the human's view, opponent self-play | `skat-core/tests/random_games.rs`, `skat-app/tests/`, `skat-app/src/tests.rs`, `skat-ai` | test (10k), deep (1M) |
| GUI | the egui frontend driven headlessly by clicks (`egui_kittest`): series by clicks, every decision and game variant clicked at least once, table elements shown, language and card-face switch, translated errors, coach levels, warning and a responsive window while the coach analyses, every card face renders; translation completeness; committed card SVGs match `tools/cardgen`; `THIRD-PARTY-NOTICES` lists every crate in `Cargo.lock` | `gui-egui/tests/gui.rs`, `gui-egui/src/i18n.rs`, `tools/cardgen/tests/`, `tools/notices/tests/` | test |
| Screenshots | named scenarios of the real frontend rendered headlessly to PNG (`egui_kittest` with wgpu, lavapipe or WARP); frontend PRs show them, CI keeps them as artifacts (D-45) | `gui-egui/examples/screenshots.rs`, CI job `screenshots`, `implement-issue.yml` | on frontend changes |
| Manual smoke run | launch the window on each platform and play (see `CONTRIBUTING.md`) | – | release 1.0 |

New crates follow the same pattern.

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
- They support the human validation of a frontend change; they do not
  replace looking at the change in a real window.

## Platform verification (M2)

Evidence that the first playable version builds and runs. A platform
counts as verified only with an interactive smoke test: launch, play a
series to the score sheet, switch language and card faces.

| Platform | Build | Launch | Interactive smoke test |
|----------|-------|--------|------------------------|
| Linux (Fedora 44 Distrobox, Wayland, Rust 1.99.1) | ✅ `cargo build --locked --release -p gui-egui`, 2026-10-29 | ✅ window opens and renders the first deal (`GRAND_COACH_SCREENSHOT`), 2026-10-29 | ✅ 2026-10-29 |
| Windows | ✅ CI job `test (windows-latest)` (Windows Server) builds everything and passes all tests, including the headless GUI tests, 2026-10-29 | ❌ not verified on Windows 11 | ❌ not verified on Windows 11; open by D-36 |

## Release 1.0 validation

Once the release scope is implemented, the maintainer manually validates
it using the [release test plan](release-test-plan.md), fixes findings and
retests before publishing 1.0 (D-61). Manual validation is a release task;
feature and milestone completion requires no manual-validation record,
status note or handoff. Required automated tests, benchmarks, calibration
and model/target measurements remain part of implementation.

## Verification vs. validation

- **Verification** ("built right"): everything above. Automated, binding.
- **Validation** ("right thing built"): does the coach help a learner, does
  the AI feel right, are the rules the ones players expect. Not automatable;
  done by the human PR review (D-27) against the issue's intent and by
  release playtesting.
  Measurable parts move into verification once
  they exist (e.g. AI strength levels via the AI-vs-AI calibration runner,
  M4, [ai.md](ai.md)).
- **Coach lessons** (D-51, D-54): correctness of each lesson's facts and
  conditions is verification, by fixtures with independently specified
  expected results; learner comprehension and unaided transfer are
  validation by the maintainer as a learner. No human Skat expert sign-off
  is required or claimed, so passing tests support only the tested claims,
  not strategic validity or learning efficacy. Checklist, learner UAT
  script and templates: [coach-validation.md](coach-validation.md).

## Later (CD)

Since M2, `test` compiles and tests the frontend on Linux and Windows, so
CI builds the executable on both. With M8: tagged releases build
installable packages after `full` passes. Public 1.0 additionally requires
the manual release gate above; an automated build is not release validation.
