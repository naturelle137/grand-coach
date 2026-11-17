# Explicit live Abstich responses (#13)

This deliberately entered question records an explicit conditional response,
never a diagnosis of a played card, loss, reasoning, strategy, mastery or
transfer. It extends the single [learning history](architecture.md#learning-history-d-70)
and keeps its denominators separate from counting and
[catalogue responses](mechanism-responses.md).

## Interaction and evidence

In an ordinary closed-display series, at a human card-play decision with more
than one legal action, the learner can select any legal hypothetical card.
Selection neither plays nor recommends it. Non-Ouvert Farbspiel and Grand use
`skat_analysis::mechanism::mechanisms` on the human's permitted `PlayerView`.
The first positive Abstich opponent in `Seat::ALL` order is the named target.
Possible means some but not all compatible hidden distributions; Certain
means all. Neither predicts actual play or identifies hidden holders.

The question hides certainty, derivation, counting aid, coach results (including
late results) and replay panels until answer/help. Coach/assistance settings
remain changeable; enabling an aid during the question does not disclose it or
create a second pending quiz. Possible, Certain and Absent
are explicit answers. Shared evidence after an answer yields Correct,
OverstatedCertainty, DeniedPossibility or UnderstatedCertainty. Requested help
reveals that same evidence and permanently marks this question assisted.
Skip settles without a score or evidence reveal; return preserves an unanswered
presentation. Reopening preserves the question and its settlement/help state.
Returning restores every legal game action without changing cards, deal,
recording, scores or series. Advancing the decision invalidates old commands.

Empty analysis is **unsupported**, never expected Absent. No absence oracle or
false-presence score exists in v1. Illegal/stale candidates are refused. Practice,
retry, training, recording, open display, God Mode, Null, Ouvert, other phases,
forced actions and final-seat/no-later-target candidates cannot earn scores.
Unsupported modes show an explanation without creating scored observations;
unsupported legal candidates in an eligible decision are recorded separately.

## Durable observations

History format 6 adds `live_abstich` entries to the existing single file. Formats
1–5 migrate losslessly with their event order and existing fields preserved.
Each opportunity uses the store's nonreused `AttemptId`, session lineage,
game/action boundary, chosen card, target (or unsupported), Abstich type and
question/evidence version 1. Different candidates are hypothetical questions at
one decision, not extra decisions. Restart/reset never reuses an attempt ID.
History completeness makes no novelty claim here.

One bounded frozen permitted `PlayerView` per candidate supplies provenance;
there are no other hands, full deal, seed or privileged recording. Expected
answers are not stored: new provenance and file loads recompute the shared
result, verify the target/version and reject inconsistent evidence. Monotone
updates cannot replace provenance, erase help, change a settled answer or add
repeated presentations. The same retention/file bounds, atomic save, confirmed
reset and damaged-file preservation/recovery apply. Confirmed reset also clears
in-memory questions in the active and parked series, so reopening cannot
resurrect an old answer; a new presentation starts unanswered. Immutable provenance is
validated on recording/load; cached evidence and summaries avoid per-frame
analysis. A session retains at most ten candidates for its current decision.

The separate DE/EN overview reports presented, answered, supported scored,
correct, overstated, denied, understated, skipped, unanswered and unsupported,
split by sticky assistance. Its source is the permitted view/shared analysis,
context ordinary closed series, question/evidence v1. Counts are deliberately
entered **and selected for positive risk**: they cannot estimate risk or error
prevalence in ordinary play. Unsupported selections, sparse/missing histories,
expected absence and false presence have explicit limitations. No percentage,
trend, window, strategic error classification or learning-effect claim is made.

## Verification

`crates/skat-app/src/live_abstich_tests.rs` extends the independent legal fixtures
in `crates/skat-analysis/tests/mechanisms.rs` through actual Session commands:
Herz Hand CA Possible (MH, never Trumpf-void HH); Kreuz pickup/HT DA/SA Certain;
Kreuz Hand with C8 C7 in unknown Skat/SA Possible; pickup/discard C8 C7 and HA
with forced Herz following unsupported. Expected categories are fixed explicitly,
not obtained from the classifier. Compatible hidden-deal substitution preserves
the prompt/evidence/category. Tests cover idempotence, sticky help, skipped and
unanswered observations, stale/invalid commands, modes, immutable provenance,
roundtrip, old formats and reset identity. Existing history IO tests cover atomic
writes and corrupt-file recovery.

The real widget flow in `crates/gui-egui/tests/live_abstich.rs` covers both
languages, candidates, answer/help/skip/unsupported/return, unchanged recording,
persisted restart and minimum window size. `screenshots` includes the
`live-abstich-{entry,candidate,answer,help,skip,unsupported,return,overview}-{de,en}`
scenarios and narrow answer views. These are technical checks, never learner
research or a release campaign.
