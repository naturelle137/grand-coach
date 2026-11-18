<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Selectable learned engine: selection and remaining levels (#77)

Increment A, 2026-11-17: independent engine selection, frozen model provenance,
bilingual fallback and legacy recording/practice compatibility are implemented.
Classic remains unchanged, available and the default (D-07). Increment B, 2026-11-18, implements the seeded profiles, per-seat
selection and calibration-runner plumbing described below. A subsequent admitted job ran the adjacent calibration; the inconclusive
results are recorded in [the opponent guide](ai.md#neural-engine-selection).
The implementation itself authorised no training or arena run.

## Historical starting point for the plan

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

## Selection, immutable configuration and provenance (increments A/B)

`SessionConfig::engine` selects Classic or Neural for ordinary level tables.
Classic is default and omitted on the wire, preserving historical config bytes.
Deserialisation and `Session::try_new` reject independent Neural combined with
a special table. The legacy `Opponents::Neural` variant remains readable with
its greedy policy and Strong fallback. No Hybrid option is exposed.

The bilingual GUI selects the engine separately from per-seat levels. Neural
shows Easy/Medium/Strong as experimental controls without measured ordering or
Classic-equivalence claims; Rookie is Classic-only. Special tables remain
Goldfish, Mephisto, God Mode and Experimental. Missing/corrupt/incompatible
models have a translated fallback notice before Start and in live status.

The app freezes the selected engine, model load result and actual hash/header
at creation; sessions/jobs share the immutable model through Arc. It preserves
the loader search/hash rules and caches failure as well as success; retry
requires an app restart. Only deliberate Start replaces a session and cancels
workers; Cancel retains the game, sheet and pending decision. The lineage guard
rejects stale results even for otherwise identical replacement settings.

`NeuralProvenance` contains the actual SHA-256/encoder or structured load
failure and policy revision. `greedy-policy-1` keeps Strong fallback;
`temperature-policy-1` uses SessionConfig's per-seat levels. The legacy fallback
field remains Strong for wire compatibility. Requested engine and levels stay
in the plain serde SessionConfig. Decision diagnostics
are exposed through `OpponentJob::run_explained`; a loaded model does not mean
every decision used inference. Identity never enters deal RNG or model input.
Classic and opponent seed streams are unchanged.

New Neural records use format 2 with provenance; legacy format 1 remains
readable, its absent identity unknown. Classic format-1 bytes and the core
replay identifier are unchanged. Replay applies recorded actions without
weights. Practice retains historical settings and compares model provenance:
unknown identity gives Missing, mismatch gives HashMismatch, and a historical
load failure keeps that fallback even if a model is now installed. All are
visible Classic fallbacks at recorded levels (Strong for historical greedy
or unknown provenance). Old greedy records may retain unused Rookie settings;
new Neural configurations reject Rookie. Practice neither scores nor records and
leaves the parked live series untouched.

## Increment B: one seeded learned difficulty mechanism

Easy/Medium/Strong are Neural choices; no Neural Rookie in this increment.
Reject unsupported combinations at the app boundary, including deserialised
configuration. Healthy Neural difficulty affects only supported card/Re
policy selection. Pre-play and Kontra rules remain unchanged. A failed model
uses the corresponding Classic level, visibly labelled as fallback; the
legacy Neural table continues using Strong.

The implementation uses temperature sampling of legal policy logits in `skat-nn`, shared by all
consumers. Strong uses the existing greedy path (temperature zero), preserving
its tie rule. Easy and Medium use temperatures 2 and 1; `temperature-policy-1` freezes
these values and the sampler before evaluation.
These are candidate controls, not an established strength ordering. Use stable
masked softmax and an explicit opponent decision seed. Illegal/non-finite
outputs retain structured fallback; never alter hidden information, legal
masks, exactness, Monte-Carlo sample counts or `Comparison::Reliable`.

`calibrate` selects these shared profiles via `nn-easy:PATH`,
`nn-medium:PATH` and `nn-strong:PATH`; legacy syntax stays unchanged.
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
Neither increment adds a dependency/model file or authorises training/arenas.
Tiny fixture and widget tests establish controls, legality and lifecycle only,
not calibrated strength.

## Holds and delivery ownership

The [operational selection](neural-operational-selection.md) authorises no new
training or arenas. Preserve that hold; implementation of selection and tiny
fixture tests can proceed independently. A subsequent compute work order must
name the exact model/profile, fresh validation ranges, resource/time limits and
evidence owner before calibration. Keep #93's remaining data/history-quality
acceptance separate from #77; load by actual model-header version, never by
relabelled weights or generation-manifest metadata.

The managed issue puller owns independent review and integration. This
increment uses `Refs #77`; learned controls are implemented but calibrated
strength remains incomplete. The subsequent admitted adjacent-level calibration is recorded in the opponent guide;
no duplicate issue owner is needed.
