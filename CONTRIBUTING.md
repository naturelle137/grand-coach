# Contributing to Grand-Coach

Thanks for your interest in contributing!

## Before you start

- Read the [Code of Conduct](CODE_OF_CONDUCT.md). It applies to all
  participation.
- Get an overview: [docs/vision.md](docs/vision.md) (goals and architecture),
  [docs/roadmap.md](docs/roadmap.md) (where the project stands) and
  [docs/decisions.md](docs/decisions.md) (what has already been decided).
- For larger changes, please open an issue first so we can discuss the idea.

## Language

The repository language is English (code, comments, commits, docs, PRs).
Skat terms stay German; see the [glossary](docs/glossary.md).

## Development setup

Install Rust stable (minimum 1.95) via [rustup](https://rustup.rs), including
the `rustfmt` and `clippy` components, and a native linker.
Alternatively, in a Fedora Distrobox, install the distribution packages
(verified on Fedora 44 with Rust 1.99.1). The second line adds the
libraries the game window needs at runtime (EGL/OpenGL and Wayland):

```sh
sudo dnf install -y rust cargo rustfmt clippy gcc
sudo dnf install -y mesa-libEGL mesa-libGL libwayland-egl libxkbcommon
```

On Windows 11, install Rust with rustup and the Visual Studio C++ Build
Tools (MSVC linker); the Windows build is checked in CI but has not yet
been tried interactively (see [docs/testing.md](docs/testing.md#platform-verification-m2)).

Start the game (binary `grand-coach` of `crates/gui-egui`, verified on
Linux):

```sh
cargo run --locked --release -p gui-egui
```

With "Settings → Show seed" the status line shows the series seed;
`GRAND_COACH_SEED=<n>` starts that series again and turns the option on. The card faces in `assets/cards/` are generated; after
changing `tools/cardgen`, run `cargo run --locked -p cardgen` and commit
the result (a test compares them). After every `Cargo.lock` change, run
`cargo fetch --locked` and `cargo run --locked -p notices` and commit
`THIRD-PARTY-NOTICES` (a test checks that it lists every crate).

All checks are bundled in
one script; activate the git hooks once per clone so it runs automatically
(`quick` before each commit, `push` with the fast tests of the affected
crates before each push):

```sh
git config core.hooksPath .githooks
scripts/check.sh          # full gate, as in CI
```

Tests only run when code files changed; docs-only changes are linted
(including markdownlint). Tests taking 2 s or more are marked
`#[ignore = "slow: CI only (docs/testing.md)"]`; the hooks skip them, CI
and `scripts/check.sh` run them. For the linters locally:
`sudo dnf install nodejs ShellCheck`; missing ones are skipped with a
warning. CI runs the same gate on every pull request; its check `ci-ok`
must pass before a PR can be merged. Details: [docs/testing.md](docs/testing.md).

Use `cargo fetch --locked` to download the locked dependencies in advance.
Build and test commands can then use `--locked --offline`, for example
`cargo test --locked --offline --all-features -- --include-ignored`. The test suite includes
10,000 seeded games and their deterministic replays, complete seeded
series through `skat-app` and headless GUI tests that play series by
clicks. The latency and estimation quality of the analysis are measured
by hand (see [docs/analysis.md](docs/analysis.md)):

```sh
cargo run --locked --release -p skat-analysis --example bench
```

The strength levels of the opponents are calibrated and timed by hand
(see [docs/ai.md](docs/ai.md)):

```sh
cargo run --locked --release -p skat-ai --example calibrate -- validation medium strong 200 10 0
cargo run --locked --release -p skat-ai --example latency
```

The coach latency per human decision is measured by hand through
`skat-app` (see [docs/coach.md](docs/coach.md#latency)):

```sh
cargo run --locked --release -p skat-app --example coach_latency
```

Screenshots of the frontend are rendered headlessly into a directory
(needs a wgpu adapter, e.g. `mesa-vulkan-drivers`; see
[docs/testing.md](docs/testing.md#screenshots-of-the-frontend)):

```sh
cargo run --locked -p gui-egui --example screenshots -- screenshots
```

New Rust
dependencies must have an [allowed licence](#allowed-third-party-licences)
and be listed in [docs/third-party.md](docs/third-party.md).

## Branching

- `main` is the only long-lived branch.
- Work on a feature branch created from `main`, e.g. `feat/bidding-values`
  or `fix/null-ouvert-scoring`.
- Changes reach `main` only through a pull request; no direct pushes.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): description`, scope optional.

| Type | Use for |
|------|---------|
| `feat` | a new feature |
| `fix` | a bug fix |
| `docs` | documentation only |
| `style` | formatting, no logic change |
| `refactor` | code change that neither fixes a bug nor adds a feature |
| `test` | adding or correcting tests |
| `chore` | build, tooling, dependencies, repository maintenance |

Example: `docs: add roadmap with milestones`

## Pull requests

- Keep PRs small and focused; review your own diff first.
- Describe what changes and why; reference issues where they exist.
- If a change affects a decision, the vision or the roadmap, update the
  matching file in `docs/` in the same PR.

## Automated implementation (Claude)

Maintainers can let Claude implement a single issue (D-27): Actions →
"Implement issue" → enter the issue number, or
`gh workflow run implement-issue.yml -f issue=<n>`. Claude works on the
branch `claude/issue-<n>` and opens a regular PR to `main`, which is reviewed
like any other PR. The procedure is the skill
`.claude/skills/implement-issue/SKILL.md`; the workflow ticks the issue's
checklist items the agent verified as done and accepts no other change to
the issue text (D-29). It runs the full gate before opening the PR (draft if
red) and starts CI for it (D-30). Requires the repository secret `CLAUDE_CODE_OAUTH_TOKEN`
(created with `claude setup-token`). Optional secret `PR_CREATION_TOKEN`:
a maintainer's fine-grained token (repository access to this repository,
permission "Pull requests: Read and write") that opens the PR, so it
triggers CI directly; without it the PR comes from `github-actions` and the
workflow dispatches CI itself.

## Licences

By contributing you agree that code is licensed under the EUPL-1.2 and other
content under CC BY-SA 4.0; see the licence section of the
[README](README.md#licence) for the details, including SPDX headers and
third-party files.

### Allowed third-party licences

Third-party code, libraries, fonts and content may enter the repository or
the build only under these licences (SPDX identifiers, D-42). Record them in
[docs/third-party.md](docs/third-party.md) first.

- **Code and libraries (crates, copied code)**: MIT, MIT-0, Apache-2.0
  (also `WITH LLVM-exception`), BSD-2-Clause, BSD-3-Clause, ISC, Zlib,
  BSL-1.0, Unicode-3.0, Unicode-DFS-2016, Unlicense, 0BSD, CC0-1.0 and
  MPL-2.0 (changed MPL files stay MPL-2.0).
- **Fonts**: OFL-1.1, Bitstream-Vera, Ubuntu-font-1.0 (unchanged only) and the code
  licences above.
- **Graphics, sounds, texts**: public domain, CC0-1.0, CC-BY-3.0,
  CC-BY-4.0, CC-BY-SA-4.0 and the permissive code licences above.
- **Multiple licences** (`A OR B`): allowed if one option is on the list;
  for `A AND B` every part must be. Note the chosen option if it is not
  obvious.
- **Development tools** that are only run, never shipped or copied (linters,
  CI actions): any OSI-approved licence, GPL included.

Not allowed: GPL, AGPL or LGPL only (no allowed alternative), SSPL, BUSL and
other source-available licences, NC and ND licences, proprietary files and
files of unclear origin. Anything not listed needs the maintainer's approval
and a new D entry in [docs/decisions.md](docs/decisions.md).

Every binary or package handed out must ship
[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) next to the program (D-43).
