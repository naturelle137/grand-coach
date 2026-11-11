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
  markdownlint (`.markdownlint-cli2.yaml`), offline Markdown links and their
  positive/negative fixtures, shellcheck, actionlint.
- `test` (CI): clippy of the workspace with `--all-features`, clippy of
  each package with features without them, and every test including the
  slow ones (`--include-ignored`) with all features except GPU backends,
  all `--locked`.
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

**Code files** (changes trigger `test`): `*.rs`, `Cargo.toml`,
`Cargo.lock`, `rust-toolchain*`, `assets/`, `scripts/check.sh` and
`.github/workflows/ci.yml`. Docs-only changes run only `lint`. CI decides
this in the job `changes` instead of a `paths:` filter, so `ci-ok` is
always reported and never blocks a docs PR.

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
