# AGENTS.md – Grand-Coach

Canonical guide for coding agents (Codex, Claude Code and others).
`CLAUDE.md` only imports this file; edit agent instructions here.

Skat trainer for PC (Linux, Windows 11): Skat per ISkO against two AI
opponents, plus an optional coach that explains the line of thought (What to
think about? Which options? How do they compare?). Focus: card counting and
putting yourself in the opponents' shoes.

## Reading paths

Start with `docs/vision.md`, the status and next work in `docs/roadmap.md`,
and the relevant parts of `CONTRIBUTING.md`. Then read only the contracts
that the task touches:

- Core/rules: `docs/architecture.md`, relevant D entries in `docs/decisions.md`.
- Analysis/AI: `docs/analysis.md`, `docs/ai.md`.
- Coach: `docs/coach.md`; lessons additionally use the evidence checklist in
  `docs/coach-validation.md`. Read `docs/coach-didactics.md` for product
  direction, not as a description of current implementation.
- App/GUI: the relevant analysis/coach contracts and `docs/testing.md`.
- Verification and release: `docs/testing.md`.
- Learned engines (#67), starting with #69/#70: D-07 and D-11 in
  `docs/decisions.md`, `docs/analysis.md`, `docs/ai.md`, the lifecycle/latency
  in `docs/coach.md`, `docs/testing.md`, and the issue's prerequisites. The
  full didactic history is not required.
- Dependencies/assets: `docs/third-party.md` and CONTRIBUTING's licence policy.
- German Skat terminology: `docs/glossary.md` as needed.

Historical records and raw research are optional background. New fundamental
product, architecture or verification decisions get a D entry in
`docs/decisions.md`; a changed decision is edited in place to its current
state (no history, only the date of the last change). Routine
implementation details belong in code/PRs. Keep documents short.

## Language

- **Repository language is English**: code, comments, commit messages, docs,
  PR titles and descriptions.
- **Chat with the user in German.**
- `README.md`: German first, English section second.
- Skat terms stay German in code and docs; see `docs/glossary.md`.

## Tech stack

Rust (Cargo workspace), GUI with egui. Later additionally Godot via
godot-rust/gdext (D-11).

## Architecture (crates)

```text
crates/skat-core      rule engine: pure, deterministic (seed), no IO, no GUI
crates/skat-nn        learned models: permitted-view encoder, Burn CPU inference, fallback reasons (docs/ai.md)
crates/skat-analysis  card counter, opponent model, option evaluation (Monte Carlo, double-dummy; docs/analysis.md)
crates/skat-ai        opponent AI with strength levels, AI-vs-AI calibration (docs/ai.md)
crates/skat-coach     structured explanations (text keys + parameters), no texts
crates/skat-app       application interface: session, commands, events, view models
crates/gui-egui       egui frontend
tools/cardgen         generates card SVGs into assets/cards/
tools/notices         generates THIRD-PARTY-NOTICES from Cargo.lock (D-43)
tools/nn-train        trains the skat-nn networks outside the app (CPU, GPU via Vulkan; D-11)
```

Dependencies only point downwards: core ← nn ← analysis ← ai/coach ← app ← gui
(`skat-nn` needs only `skat-core`; AI and coach may use it directly).
Translations live in `assets/i18n/`.

## Hard rules

- **Coach and AI do not compute separately.** Both use `skat-analysis` (and
  `skat-nn` below it for learned models); the coach only verbalises what was
  computed there.
- **Separate information sets.** AI and coach get one player's view, never the
  full state. Exceptions (open mode, post-game analysis) are visible in the
  type system.
- **Frontends use `skat-app` only.** No egui or Godot types below `gui-*`.
  API types are plain data (serde-capable) so Godot can attach later without
  core changes.
- **No user-facing text in logic.** German/English texts live in translation
  files under `assets/i18n/`.
- **Determinism.** All randomness goes through a passed-in seed/RNG so tests,
  replays and "same deal again" work.
- **Green gate only.** Never commit or push with a failing
  `scripts/check.sh`, never bypass the git hooks (`--no-verify`, changing
  `core.hooksPath`), never weaken checks or tests to get green.
- Rule questions are decided by the ISkO; Kontra/Re follows D-03. No Ramsch,
  no Bock.

## Build and test

All checks run through one script (D-30, `docs/testing.md`):

```sh
scripts/check.sh          # full gate as in CI: lint + test
scripts/check.sh lint     # whitespace, SPDX, rustfmt, markdownlint, shellcheck, actionlint
scripts/check.sh test     # clippy (also without features) and all tests incl. slow ones
scripts/check.sh push --base origin/main  # pre-push: lint, clippy, fast tests of affected crates
scripts/check.sh deep     # 1,000,000 random games, 100 Mephisto games (weekly in CI)
```

The git hooks in `.githooks/` run `quick` before each commit and `push`
before each push (fast tests of the affected crates only); Claude Code
activates them via `.claude/settings.json`. CI always runs `lint`, and
`test` on Linux and Windows plus an MSRV check when code changed; its job
`ci-ok` is required to merge into `main`. Tests taking 2 s or more carry
`#[ignore = "slow: CI only (docs/testing.md)"]`: hooks skip them, CI and
`full` run them. Locally missing linters are
skipped with a warning; CI runs them all. `cargo test` includes 10,000 seeded random
games (`crates/skat-core/tests/random_games.rs`). For documentation
changes, also check relative links (not automated).
Document new commands here and in `CONTRIBUTING.md` only once verified.

Local Fedora 44 Distrobox setup is documented in `CONTRIBUTING.md` and was
verified with Rust 1.99.1. `cargo fetch --locked` prepares the dependency
cache; `cargo build --locked --offline`, `cargo test --locked --offline`
and `cargo test --locked --offline --all-features` also pass locally.

The desktop app is the binary `grand-coach` of `crates/gui-egui`
(verified on Linux/Wayland; Windows 11 not yet, see `docs/testing.md`):

```sh
cargo run --locked --release -p gui-egui      # start the game
GRAND_COACH_SEED=42 cargo run -p gui-egui     # replay a series seed
cargo run --locked -p cardgen                 # regenerate assets/cards/
cargo run --locked -p notices                 # regenerate THIRD-PARTY-NOTICES
cargo run --locked --release -p skat-analysis --example bench  # analysis latency
cargo run --locked --release -p skat-ai --example latency       # AI latency per level
cargo run --locked --release -p skat-ai --example calibrate -- validation medium strong 200 10 0
cargo run --locked --release -p skat-app --example coach_latency # coach latency
cargo run --locked -p gui-egui --example screenshots -- DIR  # headless PNGs (D-45)
cargo run --locked --release -p nn-train -- smoke cpu  # Burn stack smoke test (#69)
cargo run --locked --release -p nn-train --features vulkan -- smoke vulkan  # on the GPU
```

On Linux the window needs EGL/GL and Wayland or X11 client libraries
(Fedora: `mesa-libEGL mesa-libGL libwayland-egl libxkbcommon`). After
changing `tools/cardgen`, regenerate and commit `assets/cards/`; a test
fails otherwise. After every `Cargo.lock` change, run `cargo fetch --locked`
and `notices`, and commit `THIRD-PARTY-NOTICES`; a test fails otherwise.

## Testing

- Rule engine and analysis are test-driven; rule cases get unit tests with
  fixed deals and known positions.
- The rule-engine milestone also requires thousands of seeded random games
  without rule violations (see `docs/roadmap.md`).

## Conventions

- Branching, Conventional Commits and PRs: see `CONTRIBUTING.md` (D-17).
  Feature branches from `main`, changes only via PR. Update affected vision,
  roadmap and decision documents in the same PR.
- In local sessions: once changes pass `scripts/check.sh`,
  commit them (Conventional Commits), push the feature branch and open a
  PR to `main`. Never commit failing changes, never push to `main`.
- To implement an issue, follow the skill
  `.claude/skills/implement-issue/SKILL.md` (D-29). Tick an issue's
  checklist items only when they are done and verified.
- In CI via `.github/workflows/implement-issue.yml` (D-27): the workflow
  runs that skill; follow its CI notes and do not push, open the PR or edit
  the issue yourself.
- Every source file starts with the SPDX header:

  ```rust
  // SPDX-License-Identifier: EUPL-1.2
  ```

  For scripts with `#` comments: `# SPDX-License-Identifier: EUPL-1.2`.
- Generated or drawn content under `assets/` and `docs/` is CC BY-SA 4.0
  (`LICENSE-ASSETS`). SVG files get an `SPDX-License-Identifier: CC-BY-SA-4.0`
  comment where practical.
- Third-party graphics, fonts or libraries only under an allowed licence
  (D-42; full list in `CONTRIBUTING.md`, "Allowed third-party licences"):
  code MIT, MIT-0, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, BSL-1.0,
  Unicode-3.0, Unicode-DFS-2016, Unlicense, 0BSD, CC0-1.0, MPL-2.0; fonts
  also OFL-1.1, Bitstream-Vera, Ubuntu-font-1.0; content also public domain, CC-BY-3.0,
  CC-BY-4.0, CC-BY-SA-4.0. `A OR B` needs one allowed option, `A AND B`
  all. Record every third-party file in `docs/third-party.md` before
  committing it. Never add GPL/AGPL/LGPL-only, SSPL, BUSL, NC or ND
  licences or files of unclear origin; anything else: ask the maintainer.
