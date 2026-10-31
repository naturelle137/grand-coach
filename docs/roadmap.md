# Roadmap

**Current status:** M4 done (opponent levels, issue #9, [ai.md](ai.md),
D-40, D-41): the maintainer closes it with the merge, although the
interactive smoke test of the levels is still open. The interactive smoke
test on Windows 11 is still open (D-36,
[testing.md](testing.md#platform-verification-m2)). M5 in progress
(issue #10, [coach.md](coach.md), D-48): `skat-coach`, coach levels, warning and
the egui coach panel are implemented; the coach latency is recorded on a CI
runner and window responsiveness is tested headlessly
([coach.md](coach.md#latency)); interactive smoke tests in the playable
application are still open.

A planned didactic follow-up to the implemented coach is specified in
[coach-didactics.md](coach-didactics.md), with bounded assignments in
[coach-work-packages.md](coach-work-packages.md) and tracking issue
[#39](https://github.com/naturelle137/grand-coach/issues/39). Readable review
comes first, verified lessons next, practice/progress afterwards. The work
refines M5–M8 and does not close their existing acceptance gates. Package A
(#41, D-52: compact debriefing, figures off by default) and package B (#42,
D-53: evidence contract) are implemented. Package K
(#51, D-54) prepared the evidence checklist and learner UAT protocol in
[coach-validation.md](coach-validation.md). Package C (#43, D-55) ships the
first two exact lessons, remaining Trumpf and proven voids
([coach.md](coach.md#exact-lessons-d-55)). Package D (#44, D-56) adds two
narrow card-play mechanisms, a possible or certain Abstich and partner
overtaking in the current trick
([coach.md](coach.md#card-play-mechanisms-d-56)). Package E (#45, D-57)
adds the pre-play lessons on own hand structure and legal game values
([coach.md](coach.md#pre-play-lessons-d-57)); the pre-play estimates stay
coarse. Package F (#46, D-58) adds one optional learning focus, lesson-backed
automatic hints with a neutral comparison question, skipping, and debriefing
moments chosen by lesson, focus and concept
([coach.md](coach.md#focus-and-teachable-moments-d-58)). Package G (#47,
D-59) shows each debriefing moment with the hand, trick, bidding and
history the player saw at that decision, from a permitted snapshot
([coach.md](coach.md#decision-snapshots-d-59)). No learner UAT has been
run yet.

The milestones build on each other (see the architecture in
[vision.md](vision.md)). A milestone is done when its "Done when …" holds.
No individual tasks here.

- [x] **M0 – Project foundation**
  Done when vision, decision log, CLAUDE.md, licences and contributor docs are
  in the repository.

- [x] **M1 – Rule engine (`skat-core`)**
  Done when the engine fully covers all ISkO games (bidding, Hand, Skat,
  discarding, suit, Grand, Null and Ouvert, Schneider/Schwarz, game value
  including overbidding, Seeger-Fabian) plus Kontra/Re per D-03, and thousands
  of random games (seeded) run through without rule violations.

- [x] **M2 – First playable version**
  Done when a series against two simple opponents can be played on Linux and
  Windows. Includes the application interface (`skat-app`), the egui
  frontend, self-made cards with switchable card faces, and a German/English
  UI.

- [x] **M3 – Analysis layer (`skat-analysis`)**
  Done when card counter, opponent model (distributions from bidding,
  following suit and discards) and option evaluation (Monte Carlo and
  double-dummy) deliver winning chance and expected points for every
  decision, in playable time, backed by tests against known positions.

- [x] **M4 – Opponent AI with strength levels**
  Done when 3–4 levels play measurably differently strong (AI-vs-AI series)
  and no level sees other players' cards.

- [ ] **M5 – Live coach**
  Done when the coach shows the line of thought (considerations, options,
  evaluation with reasoning) for bidding, discarding, game choice and play,
  supports the levels off / on request / important / every move plus the
  optional pre-move warning, works in German and English, and every
  statement comes from the analysis layer. Without telling the move ahead
  (D-49): staged hints before the move, a note after it and a debriefing
  of the good and bad moments after the game.

- [ ] **M6 – Card counting**
  Done when the optional counting aid (cards played, trumps out, card points,
  known voids) and the in-game quiz work.

- [ ] **M7 – Open mode, replay, post-game analysis**
  Done when open mode can be played and the coach shows how the opponents'
  hands could have been deduced. Also: games can be replayed trick by trick
  and replayed with the same deal. The debriefing of a game's good and bad
  moments already comes with M5 (D-49).

- [ ] **M8 – Training and release 1.0**
  Done when error statistics over time and the training mode work and there
  are installable packages for Linux and Windows 11 that ship
  `THIRD-PARTY-NOTICES` (D-43).

- [ ] **M9 – Godot frontend** (long term)
  Done when a Godot frontend (gdext) offers the same functionality via
  `skat-app`, without changes to engine, analysis, AI or coach.
