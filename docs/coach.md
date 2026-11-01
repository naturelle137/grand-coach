# Live coach: `skat-coach`

How the live coach of M5 explains a decision (D-48). Details are in the
code (`crates/skat-coach`, `crates/skat-app/src/coach.rs`).

Didactic goals and the historical implementation comparison are in
[coach-didactics.md](coach-didactics.md); agent-sized follow-up issues are
listed in [coach-work-packages.md](coach-work-packages.md). A–G and K are
implemented; H–J and L remain open. The sections below describe current
lesson coverage. Each lesson is checked
against [coach-validation.md](coach-validation.md) (D-54).

## Evidence only from the analysis

`skat_coach::explain(view, analysis)` turns one `Analysis` of the human's
`PlayerView` into a structured `Explanation` (D-13); `advise` runs the
analysis and the explanation in one call. The coach ranks, groups and
labels; it computes no strategy, counts no cards and models no opponent.

| Part | Source in the analysis | Kind |
|------|------------------------|------|
| Considerations `Known` | `observations` (voids, trumps out, Augen, own Skat, bids, Kontra eligibility) | exact fact |
| Considerations `Inferred` | `inferences`: expected trumps (Jacks before the announcement) per hidden place, with min/max | estimate; `certain` only if the hard constraints prove it |
| Options, values | `options[].estimate`, ranked by `skat_analysis::utility` (the opponents' utility) | estimate with standard error |
| Reasons | differences between two estimates | estimate |
| Assumptions, basis | `assumptions`, `effort` | model |

Lessons (D-53): `Explanation::evidence` is the analysis `Evidence` with
provenance, `Explanation::comparisons` each option's `Comparison` with the
best ([analysis.md](analysis.md#evidence-for-lessons)). `teach` picks the
first lesson of the catalogue whose `Lesson::check` applies, otherwise
`Teaching::Fallback`: only the model comparison, no strategic reason.
`Hints` carry the evidence and at most a fact lesson, since mechanisms
need the move; `MoveReview` carries evidence, comparison and teaching.
The catalogue holds the two mechanisms, the two exact card-play lessons
and the two pre-play lessons below, in that order; elsewhere the teaching
is the fallback.

## Exact lessons (D-55)

Two fact lessons (`LessonKind::Fact`, version 1, supported) on exact card
knowledge during card play, for every role. They cite only exact items
(`EvidenceClass::Exact`), run no search and hold with an incomplete or
cancelled analysis. `teach` takes the first that applies; a lesson needs
a fact that rests on a card of the trick in progress or the last
completed trick (`Condition::Recent`), so older facts are not repeated.
`skat_coach::lesson_content` turns the cited items into plain data
(`LessonContent`); missing or mismatching evidence gives no lesson.

| Key | Games | Applies when | Evidence shown after the move |
|-----|-------|--------------|-------------------------------|
| `proven-void` | Farbspiel, Grand, Null | another seat did not follow the led class (Trumpf or a side suit) | trick, seat, led card and card played; that seat holds no card of the class (follow-suit rule, ISkO) |
| `remaining-trumpf` | Farbspiel, Grand | a Trumpf was played | Trumpf of the game (Grand: the four Buben), own, played, in the own discarded Skat, revealed Ouvert cards and unseen, with the cards |

Before the move the hints show only the attention question (section
"Worauf achten?"), which names no option and gives no answer (D-49).
After the move the note on the last move shows the evidence, the
exception and a transfer question, in German and English.

What they do not claim:

- A proven void excludes cards only. It does not say whether the seat
  will trump or discard, who wins a later trick, or that a partner can
  win; in Null there is no Trumpf and a void seat discards.
- Unseen Trumpf are not assigned to a seat: they lie in a concealed hand
  or, with an unknown Skat, in the Skat. A Trumpf in the own discarded
  Skat is no opponent's Trumpf.
- No lesson says which card to play.

Limitations: Null has no Trumpf lesson. "Recent" is the trick in progress
or the last completed trick, not exactly "since the seat's last card".
Debriefing moments show their lesson since D-58. The Trumpf sets do not
appear as a separate fact line in the considerations; `TrumpsOut` still
does. The fixtures are in `skat-analysis/tests/exact_lessons.rs` and
`skat-coach/tests/lessons.rs`; learner UAT is pending
([coach-validation.md](coach-validation.md)).

## Card-play mechanisms (D-56)

Two mechanism lessons (`LessonKind::Mechanism`, version 1, supported) on
the trick in progress after the chosen card, Farbspiel and Grand. They
apply only after the move, only with a `Reliable` comparison, and only
when `Comparison::mechanisms` holds the mechanism: an exact deduction
from the view (`skat_analysis::mechanism`), never the estimates. A
higher utility of the chosen card triggers nothing; an incomplete,
cancelled or screened analysis gives the fallback. The text never says
which card was better; the note on the move stays a separate model
judgement.

| Key | Roles | Position | Condition (possible / certain) |
|-----|-------|----------|--------------------------------|
| `trumpf-vulnerability` | declarer, defender | the seat leads or follows the led side suit with its Ass or Zehn, which wins the trick so far | each opponent playing later in the trick holds no card of that suit **and** at least one Trumpf in some / every hidden distribution |
| `partner-overtake` | defender | the declarer led, the seat plays second without taking the trick, the partner plays last | the partner holds a higher card of the led class or, with a led side suit, none of it and a Trumpf, in some / every hidden distribution |

Hidden distributions are those `sampling` admits: hand and Skat sizes,
seen cards (own hand, played, own Skat, Ouvert cards) and proven voids.
A known Skat (declarer after pickup) and an Ouvert declarer's open cards
narrow them; an unknown Skat can hold up to two cards. Rule source: the
follow-suit rule of the ISkO (who can follow must; who cannot may trump
or discard).

The note after the move shows, per seat, the condition, whether it is
possible or certain, the unseen cards of the suit and Trumpf (Abstich) or
the cards and ways that beat the declarer's card (overtaking), the rule,
the exception and a transfer question, in German and English.

What they do not claim:

- That the opponent or partner *will* trump or overtake, who wins a later
  trick, or any plan over several tricks.
- That drawing Trumpf (or any other card) would have been better: a
  possible Abstich is a topic to check, not a judgement of the move.
- Anything about the partner trumping the seat's own card, a partner who
  already played, or a declarer card not yet played.

Not covered: Null, a seat in third position (nobody after it), side
cards other than Ass and Zehn, the case where the partner certainly
cannot overtake. The fixtures are in `skat-analysis/tests/mechanisms.rs`
(hand-derived positions, a hidden-deal substitution and an exhaustive
reference over every consistent distribution of random positions) and
`skat-coach/tests/mechanisms.rs`; learner UAT is pending
([coach-validation.md](coach-validation.md)).

Win chances are those of the deciding seat's party; the coach labels the
party (own side, declarer, defenders). Points are Seeger-Fabian list
points and, for defenders, the declarer's game points; Augen are never
estimated. Null and Schwarz objectives come from the contract.

`skat-analysis` gained `inferences` for M5: the opponent model's
distribution of trumps over the hidden places, from the same weighted
distributions as the estimates.

## Pre-play lessons (D-57)

Two fact lessons (`LessonKind::Fact`, version 1, supported) before card
play, for the bidder and the declarer. They cite only exact items and
hold with an incomplete or cancelled analysis. `skat_analysis::preplay`
computes them from the view and the rules (ISkO: Spitzen count the
declarer's cards with the Skat, also in Hand games; game value = base
value × levels; a game worth less than the bid is lost as überreizt).

| Key | Decisions | Evidence shown after the move |
|-----|-----------|-------------------------------|
| `hand-structure` | the seat's first bidding call; Drücken | per Farbspiel and Grand: own Trumpf (Grand: Buben) with the cards, Spitzen mit/ohne, and with an unknown Skat the possible Spitzen range and most Trumpf; Trumpf in the own discarded Skat |
| `game-value` | later bidding calls; Hand or pickup; the announcement | the value to reach (bid, or lowest value of an own Bid or Hold); per game type each contract still possible (with pickup, Hand, Null Ouvert, Hand Ouvert) with its value range and whether it reaches the value, only with a suitable Skat, only with Schneider or Schwarz made in play, or never; Kontra/Re, if configured, changes no game value |

Unknown versus known Skat: while bidding, at Hand or pickup and in a Hand
game the Skat is unknown and every pair of unseen cards may lie in it, so
Spitzen and values are ranges. After pickup the twelve cards fix the
Spitzen (the discard counts too); the text says that only ten are played.
After Drücken the own Skat is known.

Before the move the hints show only the question; it names no option and
contains no figures. After the move the content does not depend on the
chosen action: every candidate game is listed in a fixed order (Kreuz,
Pik, Herz, Karo, Grand, Null). Each note adds that the rating of the move
above is a coarse model estimate before card play.

What they do not claim:

- That a game wins, which game or bid is better, or a safe or profitable
  bid limit; reaching the value only rules out Überreizen.
- Anything about the opponents' cards or the bidding evidence.
- Which cards to put away.

Limitations: values leave out Schneider and Schwarz made in play (named
as such) and announced Schneider or Schwarz without Ouvert; suit and Grand
Ouvert appear only with both announced, as the rules require. The
pre-play estimates and the continuation bias stay as in
[analysis.md](analysis.md); this package does not improve them. The
fixtures are in `skat-analysis/tests/preplay.rs` (hand-derived positions
and an independent enumeration over every possible Skat of random deals)
and `skat-coach/tests/preplay.rs`; learner UAT is pending
([coach-validation.md](coach-validation.md)).

## Policies (D-48)

- **Equivalent**: within 1 point (`TIE_POINTS`) or within the noise margin,
  2 combined standard errors of the expected score (`MARGIN_SE`).
- **Weak**: at least 10 Seeger-Fabian points (`WEAK_POINTS`) below the best
  option *and* beyond the noise margin. Card play and Kontra/Re estimates
  have standard errors of about 3–4 points, so weak means a clear loss
  there; before card play (17–20 points, `analysis.md`) only gross
  mistakes qualify, which matches what 24 samples can tell apart.
- Drücken options dropped after screening are `Screened`, never weak.
- **Important decision**: the analysis is complete, at least two options
  are compared, and a *natural* option is weak. Natural: every option
  except jump bids above the next bid value and Schneider/Schwarz
  announcements. An engine judgement only: automatic teaching also needs
  pedagogical eligibility (D-58).
- **Warning**: only with a complete analysis and a weak chosen action;
  independent of lessons and focus.

## No telling ahead (D-49)

Before the move a frontend gets `Hints` (`skat_coach::hints`), never the
ranked `Explanation`. Each `RequestAdvice` reveals one more `Stage`:

1. `Considerations`: objective, sure facts, likely inferences.
2. `Situation`: the legal options in the view's order and a `Situation`
   (`Forced`, `OneStandsOut`, `SeveralEqual`, `Unclear` when the analysis
   is incomplete) plus the important-decision mark and one neutral
   comparison question (`Hints::compare_question`, D-58), the same for
   every option and ranking. No option is named as better.
3. `Figures`: every option's estimate in the same neutral order, no
   judgement; only with "show figures" (otherwise stage 2 is the last).

**Two boundaries (D-60).** The guarantee depends on the stage:

- **Stages 1 and 2, and everything with figures off** (the default):
  before the move no option is named, ranked, labelled or singled out as a
  plan, however often the player asks. Requests stop at stage 2; no
  per-option estimate is exposed. Stage 1 and 2 hints are identical under
  opposite model preferences (same lesson, concept, situation, option
  order and question).
- **Stage 3, opt-in exception**: only when the player has switched on
  "show figures" *and* asks for stage 3, the hints list every option's
  model estimate (winning chance, expected points) for comparison. They
  stay in neutral legal order without best, better or recommended labels,
  but a player can infer the model's preference from them; that is
  intended (maintainer decision, issue
  [#65](https://github.com/naturelle137/grand-coach/issues/65)). The
  estimates are the model's assessment, not a certified best move.
  Switching figures on reveals nothing by itself; switching them off hides
  the estimates at once and caps the stage at 2. Switching them on again
  at the same decision shows the stage 3 already asked for there.

Exact live facts (Trumpf counts, voids, Augen, hand structure, game
values) are not model estimates: they show with figures on or off. Only
estimated numbers (option estimates, uncertain expected card counts) follow
the setting. #46 left its second acceptance item open in PR #63 because
of stage 3; D-60 resolves it as this explicit exception. Tests:
`skat-coach/tests/focus.rs` (opposite preferences at card-play, bidding,
pickup and fallback decisions, every focus),
`skat-app/tests/coach.rs` (`requests_reveal_one_stage_at_a_time`,
`figures_off_stops_at_the_situation_and_toggling_keeps_the_request_flow`)
and `gui-egui/src/coach.rs` (de/en rendering with and without figures).

After the move `review` judges the chosen option; `MoveReview::note` is the
short note (`Good`, `AboutEqual`, `SomewhatWeaker`, `ClearlyWeaker`,
`Forced`, `Uncertain`). At the end of the game `moments` picks zero to
three teachable moments (D-58, section below). The warning (`Warning`)
carries only the chosen option and its loss.

Each moment shows the table of its decision (D-59). Its summary names
the trick and decision, the chosen action and
the model's judgement as the model's ("the model rates it clearly lower
than …"; for an unfinished analysis that there is no rating); for a good
moment also the trap the model rates clearly lower, without claiming the
player weighed it. Then come the lesson's checking questions and the
sentence that the lesson does not explain the model's rating, so score
differences never become a cause. The lesson's evidence and exceptions are
a collapsed section of their own. The collapsed "analysis details" hold the comparison per
metric with its party (winning chance, ranking value, declarer game
points, throw-in chance), the model rating of each named option, the
figures (only with "show figures"), what was known and inferred, and the
basis and assumptions (`MoveReview::basis`, `assumptions`). The summary
never shows an estimate. After the game, a forced last move gets no note
above the debriefing.

## Focus and teachable moments (D-58)

**Focus.** `CoachSettings::focus` is one optional `Concept`, one per
catalogue lesson (`Concept::ALL`: hand structure, game value, proven void,
remaining Trumpf, Abstich, partner overtaking). Only the player chooses it
("Settings → Learning focus"); the coach diagnoses nothing. It stays for
the next game until changed and never changes the game. With a focus:

- hints (`hints_for`), the note after the move (`MoveReview::lesson_for`)
  and the debriefing show the focus lesson when it applies, otherwise the
  first lesson of the catalogue;
- automatic teaching considers only the focus lesson.

**Pedagogical eligibility.** `Explanation::teachable(focus)`: a supported
lesson (of the focus) applies before the move. Only fact lessons can; they
depend on the evidence, never on the ranking. It is separate from the
engine's important mark (D-48) and from warnings. Automatic teaching needs
it: level "important" shows stage 2 at decisions that are important *and*
teachable; "every move" shows stage 1 where a lesson applies, otherwise the
compact status `Advice::NoLesson` ("no checked lesson; hints on request")
instead of invented hints. Asking always shows the stages.

**Skip.** `Command::SkipAdvice` ("Hide") puts the advice of the current
decision away until the next decision or a new request. It changes no
game, setting, analysis or held action; a warning stays held. No question
is ever forced; normal play never waits for the coach.

**Moments.** `moments(reviews, focus)` returns zero to `PRIMARY_MOMENTS`
(3) primary moments, shown in game order. Eligible: a decision with a
choice (not forced) and a supported lesson; a score gap or an avoided
trap alone is not. Fact lessons are exact, so a provisional analysis stays
eligible, but is never rated (`MomentKind::Neutral`). Ranked, at most one
moment per concept:

1. the focus lesson;
2. a complete analysis rating the choice clearly: `Good` (important,
   best or equivalent, a costly natural option avoided) or `Bad` (worse or
   weak, losing at least `MOMENT_POINTS`, 5 points);
3. a mechanism lesson before a fact lesson;
4. the larger loss or avoided trap;
5. game order.

There is no minimum of good or bad moments; a game without a lesson shows
none and says so. The ranking is a product default to test with learners,
not a validated optimum. Changing the focus after the game re-selects from
the stored reviews without a new analysis. Fixtures:
`skat-coach/tests/focus.rs`, `skat-app/tests/coach.rs`.

Limits: concepts are the six shipped lessons; no progress or adaptation
is stored.

## Decision snapshots (D-59)

Each debriefing moment shows the table as the player saw it at that
decision (issue #47). When the human acts, `skat-app` builds a
`DecisionContext` from the human's own `PlayerView` before the action is
applied: game number and decision, dealer and the seat of each player,
phase, own hand (twelve cards while discarding), cards per player,
bidding, bid, declarer, Hand or pickup, contract, Kontra/Re, the own
discarded Skat after Drücken, an Ouvert declarer's cards once revealed,
the completed tricks, the trick in progress and the chosen action. It is
plain serde data and contains no other hand, no unrevealed Skat and
nothing that happened later; a full `Game` is never kept.

Lifecycle: the snapshot travels with the decision, in the review queue
(played before the analysis finished) and with the stored review.
`moments` reports each moment's position in the reviews (`Moment::index`),
so `Debriefing::Ready` pairs it with its own snapshot
(`DebriefingMoment`). A cancelled analysis keeps decision and snapshot
queued; a failed one keeps no review; a result for another game or budget
is rejected. The next game drops queue, reviews and snapshots; at most
one snapshot per own decision of the current game is kept.

The moment card shows decision and trick, seat, role and dealer, bid or
contract, Kontra, the own Skat and open cards if known then, the hand
with the chosen card(s) marked and the trick so far with the players'
names; "History up to this decision" (collapsed) lists the bidding and
the completed tricks. Seat names in the lesson and details follow the
snapshot. Opening or closing sections changes neither game nor score.
Out of scope (#12): saved replays, alternative continuations, open mode.
Fixtures: `skat-app/src/tests.rs` (hidden-card substitution, own Skat,
Ouvert reveal, failed reviews and cleanup), `skat-app/tests/coach.rs`
(queued, cancelled, out-of-order and stale reviews; serde),
`gui-egui/src/coach.rs` (a hand-written snapshot rendered in de/en).

## Session lifecycle (`skat-app`)

- Levels (D-08, D-49, D-58): **off** (no hints, notes or debriefing, no
  help action), **on request** (default; each `RequestAdvice` one stage),
  **important** (stage 2 on its own when important and teachable, help
  action too), **every move** (stage 1 on its own where teachable, the
  compact status elsewhere). Warning off by default (D-09),
  independent of the level; "show figures" (off by default, D-52) hides
  estimates but keeps the reasoning and exact facts. Stage 3 needs it and
  a request (D-60).
- With the level not off, every own decision is analysed for the note
  (`CoachView::last_move`) and the debriefing (`CoachView::debriefing`,
  once the game is over). A decision played before its analysis finished
  is queued with its view; `coach_job` hands out the current decision
  first, then the oldest queued one (`CoachJob::is_review`), and the
  frontend cancels a running review when the current decision needs an
  analysis. A cancelled review stays queued; the next game drops the
  queue. `Session::coach_result` exposes the full analysis for tools and
  tests only.
- `Session::coach_job` hands out the analysis of the human's view when one
  is due; the frontend runs it off the UI thread. `finish_coach` accepts it
  only for the same decision (game number, actions applied) and budget, or
  for a queued one. Cancelled or failed analyses show as
  unavailable, asking again recomputes; an analysis out of budget shows as
  provisional without recommendation, importance or warning.
- Warning flow: with warnings on, `Act` holds the action until the
  analysis is there (`Checking`; "play without check" proceeds), then plays
  it or holds it with a `Warning`. `ProceedAnyway` plays it exactly once;
  `Reconsider` withdraws it, and choosing it again at the same position is
  not warned twice. Turning warnings off withdraws a held action. Nothing
  held is ever played behind the player's back.
- The coach seed comes from stream 2 of the series seed, mixed with the
  game number and the view; opponents and deals are untouched. Same view,
  seed and budget give the same explanation.

## Latency

The coach runs one `analyze` with `Budget::INTERACTIVE` per human
decision; `explain` is a single pass over the options. Its latency is the
analysis latency, with the interactive target and reference measurement
in [analysis.md](analysis.md#latency-target-and-benchmark).
`skat-app/examples/coach_latency.rs` measures it as the frontend sees it:
a session with the coach on "every move", time from `coach_job` to an
accepted `finish_coach`, the human following the coach's best option,
Medium opponents, Kontra/Re on, series seeds 1, 2, …, 10 positions per
kind:

```sh
cargo run --locked --release -p skat-app --example coach_latency
```

Recorded on 2026-10-30 on a GitHub Actions runner (AMD EPYC 7763,
4 vCPUs, 16 GB; series seeds 1–71, about 45 minutes), not on the
reference hardware with 20 threads:

| Decision | Median | p90 | Max | Incomplete |
|----------|--------|-----|-----|------------|
| Reizen | 5.5 s | 9.9 s | 26.6 s | 1 |
| Hand/pickup | 3.3 s | 4.5 s | 5.0 s | 0 |
| Drücken | 15.9 s | 36.2 s | 42.0 s | 0 |
| Announcement | 6.8 s | 8.5 s | 12.8 s | 0 |
| Kontra/Re | 0.6 s | 0.9 s | 3.0 s | 0 |
| Card play, trick 1 | 0.7 s | 3.9 s | 14.7 s | 0 |
| Card play, trick 4 | 10 ms | 38 ms | 135 ms | 0 |
| Card play, trick 8 | 1 ms | 1 ms | 1 ms | 0 |

On this historical four-vCPU run, Kontra/Re and later card play meet the
interactive target. First-trick card play misses the p90 target (3.9 s
versus 3 s), although its median passes; pre-play decisions also exceed
their targets. These are dated measurements, not a current performance guarantee. The analysis
spreads its work over all available threads, so fewer threads mean
longer waits. The positions also differ from the analysis benchmark:
every bidding decision counts, not only opening calls, and the human
bids as the coach suggests. While the coach computes, the panel shows a translated
"thinking" status and the player can act at any time; the analysis then
continues for the note, unless the next own decision needs one first. The GUI test `window_stays_responsive_while_the_coach_analyses`
checks that frames keep returning quickly during a full interactive
analysis and that acting works meanwhile. The interactive smoke test in
the playable application on the reference hardware is still open, transferred from
issue #10 to [release register #81](https://github.com/naturelle137/grand-coach/issues/81)
before public 1.0 (D-61); automated profiling/benchmarks remain due during
development. #68 adds paced/fast-action backlog measurements.
