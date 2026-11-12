# AGENTS.md – Grand-Coach

Canonical guide for coding agents (Codex, Claude Code and others).
`CLAUDE.md` only imports this file; edit agent instructions here.

Grand-Coach is an interactive Skat coach for Linux and Windows 11. The goal
is independent, expert play at a real table. Live lessons help the player
notice what matters in a deal and reason from other seats. Planned training
sessions let the player practise individual tactics across different deals.

## Reading paths

Start with `docs/vision.md`, the status and next work in `docs/roadmap.md`,
and the relevant parts of `CONTRIBUTING.md`. Then read only the contracts
that the task touches:

- Core/rules: `docs/architecture.md`, relevant D entries in `docs/decisions.md`.
- Analysis/AI: `docs/analysis.md`, `docs/ai.md`.
- Coach: `docs/coach.md`; lessons additionally use the evidence checklist in
  `docs/coach-validation.md`.
- App/GUI: the relevant analysis/coach contracts and `docs/testing.md`.
- Verification and release: `docs/testing.md`.
- Learned engines: `docs/learned-models.md` for the shipped model, current
  training phase and commands, `docs/ai.md` for calibration and D-07,
  D-11 and D-63 in `docs/decisions.md` for runtime and packaging.
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
  full state. Mephisto takes the full game explicitly. God Mode's table and coach use
  privileged views.
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
scripts/check.sh          # complete platform gate: lint + test
scripts/check.sh lint     # whitespace, SPDX, rustfmt, Markdown links/fixtures, markdownlint, shellcheck, actionlint
scripts/check.sh pr       # Ubuntu PR: clippy/features, FAST workspace + app integration
scripts/check.sh native   # Windows compile/features, IO/model/GUI and render smoke
scripts/check.sh complete # full platform clippy/features and tests incl. slow ones
scripts/check.sh push --base origin/main  # pre-push: lint, clippy, complete FAST workspace
scripts/check.sh deep     # 1,000,000 random games, 100 Mephisto games (weekly in CI)
```

The git hooks in `.githooks/` run `quick` before each commit and `push`
before each push (complete FAST workspace on code changes); Claude Code
activates them via `.claude/settings.json`. CI always runs `lint`; code
PRs run Ubuntu `pr`, Windows `native` (including required render smoke) and
MSRV. Main adds broader Ubuntu regression. Both platform suites run nightly,
on manual dispatch and for RCs. `ci-ok` remains the required check and rejects
failed, cancelled or unexpectedly skipped mandatory jobs and invalid routing.
Tests taking 2 s or more carry `#[ignore = "slow: CI only (docs/testing.md)"]`:
hooks skip them; main regression and complete platform suites run them.
The PR tier additionally retains ignored AI privacy and deterministic app integration.
Locally missing linters are
skipped with a warning; CI runs them all. `cargo test` includes 10,000 seeded random
games (`crates/skat-core/tests/random_games.rs`). The lint tier checks tracked
Markdown file/heading links offline and runs positive/negative fixtures.
Document new commands here and in `CONTRIBUTING.md` only once verified.

For local setup, running the app, generated assets and licence notices, see
[CONTRIBUTING.md](CONTRIBUTING.md#development-setup). For training and
assessment commands, see the [model guide](docs/learned-models.md).
For the Phase 2 seed audit, bounded pilot and HQ evaluation entrypoints,
see [the evaluation protocol](docs/phase2-evaluation.md). Analysis regression
fixtures run with `python3 -m unittest discover -s scripts/phase2 -p 'test_*.py'`.

Run the docs-only gate with `scripts/check.sh full --base origin/main`.
For focused link checks use `python3 scripts/markdown-links/check.py`; supported
syntax and fixture commands are in [testing](docs/testing.md#offline-markdown-links).

## Testing

- Rule engine and analysis are test-driven; rule cases get unit tests with
  fixed deals and known positions.
- The rule-engine milestone also requires thousands of seeded random games
  without rule violations (see `docs/roadmap.md`).

## Review and delivery ownership

Agents own implementation, technical validation, review and integration.
Every PR needs an independent review by a provider other than its author
(Claude reviews Codex changes; Codex reviews Claude changes). Record the
reviewed commit, provider, evidence and `VERDICT: APPROVE` or blocking
findings in the PR. Re-review relevant changes after fixes; merge only with
no unresolved blockers and green required CI. The CTO agent integrates.
The CEO does not review PRs or resolve technical findings. Changes to
repository protection settings require a separate CEO decision.

Learner participation is separate from technical review (D-51, D-61).
Agents prepare and run technical checks; actual learner observations remain
pending until collected and must never be simulated by an agent.

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
