---
name: implement-issue
description: Implement one GitHub issue of this repository end to end - read it, implement, verify, commit, tick the issue's checklist items that are genuinely done and write the PR description. Use when asked to implement, work on or resolve issue #N.
argument-hint: <issue-number>
---

<!-- SPDX-License-Identifier: EUPL-1.2 -->

# Implement an issue

Issue number: `$ARGUMENTS`. If it is missing, ask for it (in CI, stop).

## 1. Understand

- Read the issue with `gh issue view <n> --comments`.
- Read `AGENTS.md` and the documents it lists that the issue touches. Their
  hard rules, SPDX headers, language rules and conventions are binding.
- Apply D-61 (`docs/testing.md`, "Technical completion and release
  validation"): required automated verification stays due now; manual
  validation may be handed off to release register #81 before technical
  closure. Read any explicit scope-transfer note in the issue. The normal
  checkbox-only workflow cannot rewrite scope: if the issue still requires
  human results for implementation closure, report the mismatch for an
  authorized text update; do not tick or silently ignore that item.
- Collect the issue's **checklist**: every task-list item (`- [ ]` /
  `- [x]`) in the issue body, typically the acceptance criteria. Track each
  item as a todo; they define "done".
- Unclear points: ask the user in a local session. Where nobody can answer,
  pick the most reasonable interpretation and document it (PR body, and a
  new D entry in `docs/decisions.md` if it is fundamental).

## 2. Implement

- Work on a feature branch from `main` (see `CONTRIBUTING.md`); never on
  `main`. Local default: `Codex/issue-<n>`.
- Test-driven for rule engine and analysis code, as `AGENTS.md` requires.
- Update affected docs (vision, roadmap, decisions, third-party, glossary)
  in the same change.
- Before technical closure, add/update owned scenarios in #81 (functions,
  platforms, reproducible steps and required evidence), link the handoff in
  the issue/PR and update the roadmap. Record pending manual checks; a
  transfer is not a performed check. In CI, include the proposed register
  update in the PR handoff for the maintainer; do not edit issues beyond the
  workflow's authorized checkbox output or claim closure until recorded.
- Commit in focused Conventional Commits, only when all checks pass.

## 3. Verify

Run `scripts/check.sh` (the full gate, see `docs/testing.md`) plus
`git diff --check main`. Fix everything you introduced. Do not finish with
failing checks, and never bypass the git hooks or weaken a check or test
to get green.

- Wait for every check to finish; builds and tests take minutes. Run them
  in the foreground with a long timeout, never in the background.
- A failing pre-commit or pre-push hook is a finding to fix, not a reason
  to stop: read its output, fix the lint, clippy or test failure, commit
  again. You are done only when `git status` is clean and the gate is green.

## 3a. Screenshots (frontend changes)

If the change affects what the frontend shows (`crates/gui-egui/`,
`assets/`), show it in screenshots (D-45):

- Add or adjust a scenario in `crates/gui-egui/examples/screenshots.rs`
  that reaches the changed state, so the picture says something about the
  change; reuse an existing one only if it already shows it.
- Render: `cargo run --locked -p gui-egui --example screenshots -- <dir>
  <scenario>...` (headless; needs a wgpu adapter, e.g. lavapipe
  `mesa-vulkan-drivers`).
- Look at every PNG (Read) and check it against the issue's intent before
  claiming the behaviour is checked.
- Local session: render into a scratch directory and show the images to
  the user. When the PR is opened, commit them to the orphan branch
  `screenshots` under `issue-<n>/` and embed them in the PR body via
  `https://raw.githubusercontent.com/<owner>/<repo>/screenshots/issue-<n>/<name>.png`.
- If the caller says where to put them instead (CI does), do that.

## 4. Assess the checklist honestly

For each checklist item decide **done** or **not done**. An item is done
only if all of these hold:

1. It is fully implemented on the branch, not partly or as a stub.
2. You verified it: a passing test or command demonstrates it, or for
   documentation items you re-read the resulting file. Name the evidence.
3. Nothing in that item's current scope was deferred, skipped or left as
   a TODO. A separately authorized D-61 scope transfer can make the
   implementation issue technically closable; the transferred human check
   stays unchecked and pending in #81, never "done" here.

Anything partial, untested, failing, unverifiable by you (e.g. needs a
human or external access) or uncertain is **not done**.

**Never tick an item to make the work look complete.** A wrongly ticked
item is worse than an open one: it hides missing work from the reviewer.
Leave items that others already ticked unchanged; if you think one does
not hold, say so in the PR body.

## 5. Tick the done items in the issue

Change nothing in the issue body except `[ ]` → `[x]` for items judged
done in step 4.

- Fetch the current body: `gh issue view <n> --json body --jq .body > current.md`
  (use a scratch directory, not the repository).
- Write `edited.md` with only those markers changed and check it with
  `python3 .github/scripts/merge-checklist.py current.md edited.md > merged.md`;
  it fails if anything besides checkbox state changed.
- Local session: apply with `gh issue edit <n> --body-file merged.md`.
- If the caller says where to write the edited body instead (CI does), write
  it there and do not edit the issue yourself.

## 6. Pull request

Title: one Conventional Commit line. Body: follow
`.github/pull_request_template.md` and add:

- `## Issue checklist`: every checklist item with ✅ or ❌ and its
  evidence (test name, command, file) or the reason it is open.
- Decisions taken without being able to ask.
- `Closes #<n>` only if all technical scope is verified and any manual
  scope has an explicit authorized D-61 transfer recorded in #81 and linked
  from the issue/PR. A historical unchecked item explicitly transferred
  out of implementation scope is not a performed test. Otherwise use
  `Refs #<n>` plus `## What remains`.
- Do not close #81, #54's actual human evaluation or #13 final public 1.0
  readiness through a transfer: they require their actual results. Model
  quality/resource/target gates and automatic tests are not deferred.
- Tick the template's own checklist by the same honesty rule.
- End with the PR attribution line given by your environment.

Local session: push the feature branch; open the PR only when the user asks.
If the caller says where to write title and body instead (CI does), write
them there and do not push or open the PR.

## 7. Report

Summarise what was done, what is open (with reasons) and which checklist
items you ticked.
