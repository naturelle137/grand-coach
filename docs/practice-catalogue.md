# Varied practice catalogue and focus sessions (#50)

`skat_app::practice_catalogue` is the unchanged source for catalogue mode in
`TrainingSession`. The session uses the existing reconstruction and shared
analysis; it adds no strategic evaluator.
All cases are revision 1, closed Hand games: no Ouvert, no known Skat.
The seed completes an explicitly anchored deal, not `Deal::from_seed(seed)`
alone. Core validates that deal and every bidding/Hand/announcement/play
in the prefix; only the historical acting `PlayerView` leaves reconstruction.
This follows `TrainingFixture::view` through `Game::apply`; it does not
broaden the recording format's seed/deal identity check to accept anchored deals.

## Independent reference specification

These expectations define the cases before assertions against shared analysis.
V/M/H mean Vorhand/Mittelhand/Hinterhand; A/10/K/Q/J/9/8/7 name ranks.
All claims below are exact facts or conditional feasibility deductions, never
model estimates or best-move judgements. Rule basis is the ISkO, with the [core interpretations](decisions.md#d-28-rule-interpretations-in-skat-core)
and `skat-core` follow/ranking rules (serving the led class). Buben are Trumpf in
Farbspiel/Grand; in Null they belong to their printed suit.

| Stable ID | Seed | Role / game / acting seat | Independent answer and reason |
| --- | --- | --- | --- |
| `remaining-trumpf.intro` | 501 | Declarer / Karo / V | 5 outside: 11 total minus own Karo 7/8/9/10/A and Kreuz J (6); no played Trumpf. |
| `remaining-trumpf.changed` | 502 | Defender / Grand / M | 1 outside: 4 minus own Kreuz/Pik J (2), minus led Herz J (1). Unknown Skat is included, no holder assigned. |
| `remaining-trumpf.boundary` | 503 | Defender / Grand / H | 0 outside: all four Buben in own hand, even though a side suit is on the table. |
| `proven-void.intro` | 504 | Defender / Herz / H | 1 other seat proven Kreuz-void: V leads Kreuz A, M plays Kreuz J (Trumpf). A printed Kreuz does not serve Kreuz here. |
| `proven-void.changed` | 505 | Declarer / Grand / H | 1 other seat proven Trumpf-void: V leads Kreuz J, M plays Herz A. H has not played; do not infer a second void. |
| `proven-void.boundary` | 506 | Defender / Null / H | 0 proven Kreuz-void: V leads Kreuz 9, M serves Kreuz J. A Bube serves its printed suit here. |
| `side-card.intro` | 507 | Declarer / Herz / V | Leading own Kreuz A allows possible Abstich by M and H. Own hand contains all seven non-Bube Kreuz, so no unseen Kreuz; unseen Trumpf can be assigned to either later defender, but need not be. |
| `side-card.changed` | 508 | Defender / Grand / M | After V's Pik 7, own Pik A allows possible Abstich by declarer H. All six other non-Bube Pik are own; three unseen Buben need not belong to H. |
| `side-card.boundary` | 509 | Declarer / Null / V | Leading Kreuz A has no Abstich: Null has no Trumpf. This does not assert that the Ass is a good Null move. |
| `partner-overtaking.intro` | 510 | Defender / Herz / M | After declarer V's Kreuz 9, own Kreuz 7 leaves possible overtaking by H: unseen Kreuz A is higher, or H can hold no Kreuz and a Trumpf. Not certain. |
| `partner-overtaking.changed` | 511 | Defender / Grand / V | H wins the first Buben trick and leads Pik 10. Own Pik 7 leaves partner M a possible Pik A or Abstich with the unseen Karo J. Not certain. |
| `partner-overtaking.boundary` | 512 | Defender / Karo / M | Declarer V leads Kreuz J; own Pik J cannot enable overtaking. No card beats the highest Trumpf. A partner void alone would not change that. |

Each family has an introductory worked candidate, a changed surface/seat
candidate and a boundary/counterexample. These are candidate roles only.
Every previously shown or retried case is practice. Sessions determine unseen
eligibility before persisting the current presentation, including an unanswered
presentation. Only changed cases in a complete, available history are eligible. Catalogue membership proves no
transfer, retention, learner efficacy or strategic certification.

## Answers, alternatives and information boundary

Counting accepts the exact integer. Mechanism questions ask about a named
hypothetical legal card, not which card is best. Feedback uses shared
`counting::derive` or `mechanism::mechanisms` and exposes their structured
provenance/conditions, with every legal action still available. No legal
alternative is discredited. In particular, all five legal Pik cards below
the led Pik 10 in `partner-overtaking.changed` leave the same overtaking
possibility; all legal Trumpf in its boundary fail to enable overtaking.
An empty mechanism result means the narrow mechanism is absent, not that a
move is safe or optimal. No reliable model comparison is requested or
fabricated; using these deductions in a live strategic lesson still requires
the existing reliable-comparison/fallback contract.

`position()` exposes the task and permitted view; `feedback()` is a separate,
explicit answer operation. The UI does not reveal feedback, counters or
preferred plans ahead of an unaided attempt. IDs, case roles and seeds are
catalogue metadata, not prompt copy. They must not be used as evidence about
hidden cards. Changing a deal, prefix, question or answer semantics requires
raising the revision. The four old `TrainingFixture` identities and two `TrainingTopic` values remain
unchanged. Format 5 of the same learning history retains typed catalogue events.

Tests replay prefixes and all legal alternatives through core, compare the
hand-written reference counts and mechanism conditions, reconstruct alternate
hidden deals with identical views, and check seed/revision reproduction.
Positive feasibility has concrete core-legal witness completions; negative
witnesses establish that possible does not mean certain. These checks certify
only the bounded claims above. Session, assistance/exposure and bilingual widget tests extend these unchanged
oracles. Actual learner observations remain #50/#54 work.

## Session behaviour and conservative exposure

The existing training entry offers all four families plus the original fixed
counting training. Each catalogue family presents intro, changed and boundary
in order; no preview shows a changed case. Tasks contain only the acting
`PlayerView` and question. Answers, explicit help and skip reveal feedback from
`PracticePosition::feedback`; no preferred move or strategic optimality is scored.
Counts require an exact integer. Mechanism responses distinguish possible,
certain and absent; an empty supported boundary result never means safe/optimal.
Unsupported counts have no score. Unknown task IDs/revisions are rejected, not
silently upgraded. Help and visible support are sticky for the opportunity.
Retry requires settlement and repeats as practice; next cycles three cases.
Topic choice and exit retain unanswered exposure and leave the live session,
recording and series untouched.

The one `LearningHistory` writes format 5. Its `focus` transitions carry an
allocated attempt, immutable task ID/version, question, selected support,
sticky assistance, pre-presentation eligibility and presented/help/answer/skip.
Answer and skip transitions retain shared-analysis provenance; answers also
retain the submitted and expected typed responses. Mechanisms never enter
numeric deviation categories. The overview lists these observed responses
separately within the same history window. Bounds, atomic replacement,
recovery, deduplication and the attempt counter are shared with the old events.
Formats 1/2/3/4 keep all supported attempts/events/counters in order and acquire no
fabricated prior exposure; their next save writes 5. Older builds preserve the
unsupported format instead of dropping catalogue events.

Eligibility is evaluated before the current exposure is recorded and saved.
If an atomic write fails, the UI withholds any new position until persistence
works; a restart cannot then mistake an unrecorded presentation for unseen.
Only a changed task with no earlier presentation of that ID is eligible, and
only with complete exposure and available persistence. A missing file, failed
save, memory-only fallback, corrupt/unsupported history, migration or confirmed
reset disables novelty; practice remains available. Reset clears the deliberately
reset history and keeps the attempt counter, but cannot prove never-seen learning.
The app cannot know exposure in another file, an older installation, screenshots
or outside the app. Concurrent app writers are not coordinated (last save wins).
These local observations certify neither efficacy nor human transfer.

The separate [reserved validation](validation54.md) uses eight additional
identities outside `PRACTICE_CASES`. They never enter these public sequences.
Format 5 retains their explicit protocol/build/exposure/timestamp and verbatim
response metadata without granting novelty to migrated history.
