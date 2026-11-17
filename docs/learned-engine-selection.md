<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Selectable learned engine: implementation plan (#77)

Planning increment, 2026-11-17. This describes proposed work for
[#77](https://github.com/naturelle137/grand-coach/issues/77), not shipped
selectors, calibrated learned levels or authorisation for training/arenas.
Classic remains unchanged, available and the default (D-07).

## Verified starting point

Source audited at `63f0ce0bbfa90a52724798cc9f4265a482cd320d`:

- [Session configuration](../crates/skat-app/src/session.rs) stores either
  `Opponents::Levels([Level; 2])` or a special table mode. Its default is
  Classic Medium on both seats. `Opponents::Neural` has no level setting;
  each `OpponentJob` captures a permitted view and a Neural opponent with
  Classic Strong fallback. Position lineage rejects results from another series.
- [Model loading](../crates/skat-app/src/neural.rs) is app-owned and cached
  once, including failure. The shipped optional file is PPO v2 4×384,
  encoder 2 with 634 inputs, SHA-256
  `6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75`.
  The [manifest](../assets/models/card-play.json) and
  [model card](../assets/models/card-play.md) identify its provenance.
- [NeuralOpponent](../crates/skat-ai/src/neural.rs) uses Classic Kontra
  rules before model inference. The model can choose cards and Re;
  unsupported phases or failed evaluation fall back to Classic.
  [`Choice::Policy`](../crates/skat-nn/src/model.rs) is greedy with first
  legal-order tie breaking; it has no temperature or difficulty parameter.
- The [new-series dialog](../crates/gui-egui/src/app.rs) edits a draft.
  Start replaces the session and drops pending GUI work; Cancel leaves the
  live series intact. There is no live opponent-settings command.
- [Recordings](../crates/skat-app/src/recording.rs) already exist (#12):
  format 1 stores configuration, opponent seeds and applied actions, but
  no model hash. Reconstruction applies recorded actions without running
  AI. Same-deal practice in
  [skat-app](../crates/skat-app/src/session.rs) creates a separate session.

[#73](https://github.com/naturelle137/grand-coach/issues/73) is closed.
[#93](https://github.com/naturelle137/grand-coach/issues/93) remains open:
existing v2 features do not establish useful history-sensitive strategy.
Do not repeat the encoder change or infer strength from its version.

## Increment A: selection, immutable configuration and provenance

Keep the existing Classic path and special tables. Add a plain serde
`OpponentEngine` choice, Classic or Neural, independent of the two ordinary
opponents' levels in `SessionConfig`. Default and omit Classic on the wire,
so old Classic configurations retain their bytes. Only ordinary level tables
use this field; validate and reject Neural combined with a special table.
Continue accepting the old `Opponents::Neural` configuration with its exact
greedy/Strong-fallback semantics. Do not reinterpret historical recordings.
Do not add a selectable Hybrid before #76 implements it.

The ordinary-table GUI uses an engine selector and separate seat difficulty
selectors. Until increment B is implemented, Neural exposes the existing
single experimental policy only, explicitly without learned difficulty
claims; retained Classic difficulty drafts are not presented as Neural levels.
Keep Goldfish, Mephisto, God Mode and Experimental as special table modes.
All labels, help and fallback notices belong in Fluent de/en; frontends
consume only `skat-app` types. Missing, corrupt and incompatible models show
a translated Classic-fallback notice both before Start and at the live table.

Freeze the selected engine, model load result and model identity at session
creation, sharing the immutable model through the existing `Arc` ownership.
No filesystem IO below `skat-app`, no hot reload during a series, and no
filesystem path in the model input. Keep the current loader search/hash rules;
retrying after a failed cached load requires an application restart.
Settings apply only on deliberate Start of a new series. Cancelling a draft
keeps the game, score sheet and pending decision. Starting a new series drops
pending work and preserves the existing lineage guard against late results.

Expose plain diagnostic provenance: requested engine, actual loaded SHA-256
and encoder version or structured load failure, policy revision, configured
levels and Classic fallback profile. Report decision-level fallback reasons
from `NeuralDecision`; a loaded model alone does not mean a decision used it.
Keep identity out of the deal RNG and model inputs. Existing opponent seed
streams remain independent of deal seeds, preserving Classic seeded games.

Include provenance in recordings now that #12 exists: use a new wire format
for records containing the new context; retain explicit format-1 reading and
reconstruction, marking absent provenance unknown. Do not bump the core replay
engine identifier merely for AI metadata. Legacy files stay unchanged.
Replay needs only recorded actions, not installed weights. Same-deal practice
must retain recorded opponent settings, compare the recorded model hash with
the available file, and disclose Classic fallback on absence/mismatch rather
than silently substituting another model. Test that returning from practice
leaves the live series untouched.

## Increment B: one seeded learned difficulty mechanism

Add Easy/Medium/Strong as Neural choices; no Neural Rookie in this increment.
Reject unsupported combinations at the app boundary, including deserialised
configuration. Healthy Neural difficulty affects only supported card/Re
policy selection. Pre-play and Kontra rules remain unchanged. A failed model
uses the corresponding Classic level, visibly labelled as fallback; the
legacy Neural table continues using Strong.

Use temperature sampling of legal policy logits in `skat-nn`, shared by all
consumers. Strong uses the existing greedy path (temperature zero), preserving
its tie rule. Easy and Medium use finite positive temperatures, initially
larger for Easy, with values frozen in a versioned profile before evaluation.
These are candidate controls, not an established strength ordering. Use stable
masked softmax and an explicit opponent decision seed. Illegal/non-finite
outputs retain structured fallback; never alter hidden information, legal
masks, exactness, Monte-Carlo sample counts or `Comparison::Reliable`.

Extend `calibrate` to select these same profiles, not a second implementation.
Record the full model hash, encoder/profile revision, temperatures, fallback,
suite and first-deal range. Under separately authorised compute, tune first,
then compare each adjacent pair on 200 fresh validation deals, six seatings
and paired deal-level 95% intervals as specified in
[the existing protocol](ai.md#calibration-protocol). Record every attempt in
`docs/ai.md`. Unordered or unmeasured levels stay experimental; equal level
names never imply equal strength to Classic. Existing Phase 2 comparisons
cannot calibrate these new profiles.

## Required evidence before implementation acceptance

| Boundary | Verification required in the implementing PRs |
|----------|----------------------------------------------|
| Compatibility/default | Old Classic config bytes/defaults and seeded regression unchanged; legacy Neural semantics; reject unknown/invalid combinations. |
| Model failure | Tiny missing, corrupt, hash-mismatched, incompatible and non-finite fixtures; legal fallback through a complete game/series; bilingual live notice. |
| Settings/workers | Draft cancel preserves series and in-flight job; Start rejects old result; cancelled computation remains legal; next-game and process restart use documented frozen settings. |
| Determinism/privacy | Fixed bytes, profile revision, seed and human commands reproduce actions; hidden-deal substitution invariant; RNG isolated from dealing/coach/portraits. Cancellation timing is excluded from identical-action promises. |
| Sampling | Zero-temperature parity including ties; seeded finite-temperature repeatability and legal support; tiny fixtures, no strength claim from unit tests. |
| Recording/practice | Format-1 migration; new-format round trip; unknown provenance explicit; hash mismatch fallback; action replay independent of model; live-series isolation. |
| GUI | Headless clicks independently choose engine and levels in de/en; Start/Cancel/failure paths; screenshot scenarios for selectors and live fallback, visually inspected and attached to PR. |
| Strength | Actual separately authorised adjacent-pair runs with full identities and paired intervals; otherwise explicitly unmeasured and experimental. |

Run the project gate through `skat-dev`, including the existing app integration,
model/privacy and egui checks when code changes. Implementation PRs update
D-07's concrete contract if needed, affected app/AI/testing docs and the roadmap.
No additional dependency, model file, training or benchmark is needed to review
this plan. No issue acceptance checkbox is satisfied by planning alone.

## Holds and delivery ownership

The [operational selection](neural-operational-selection.md) authorises no new
training or arenas. Preserve that hold; implementation of selection and tiny
fixture tests can proceed independently. A subsequent compute work order must
name the exact model/profile, fresh validation ranges, resource/time limits and
evidence owner before calibration. Keep #93's remaining data/history-quality
acceptance separate from #77; load by actual model-header version, never by
relabelled weights or generation-manifest metadata.

The managed issue puller owns the next implementation increment, independent
review and integration. This planning PR uses `Refs #77` and leaves all four
issue criteria open. Remaining delivery is increment A, increment B and the
separately admitted calibration evidence; no duplicate issue owner is needed.
