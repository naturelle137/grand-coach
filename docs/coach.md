# Live coach

The coach translates shared analysis into structured observations, hints and
reviews. The GUI renders Fluent text in the player's language. The coach
evaluates each human decision from the view available when it was made.
God Mode also supplies the known deal for option estimates.

## Exact lessons (D-55)

`proven-void` cites the play that proved a seat could not follow the led
class. `remaining-trumpf` counts Trumpf outside the player's known cards
in Farbspiel and Grand. An unknown Skat can contain outstanding Trumpf.
Neither lesson locates an unseen card without public evidence.

## Card-play mechanisms (D-56)

After a move, `trumpf-vulnerability` checks whether a later opponent can
trump a winning side Ass or Zehn. `partner-overtake` checks whether the
partner in last seat can overtake the declarer's lead. The shared analysis
tests all relevant hidden-card constraints jointly. A mechanism can be
possible or certain. The legal opportunity alone does not establish the
player's intention or the cause of a score difference.

## Pre-play lessons (D-57)

`hand-structure` describes own Trumpf, Buben and Spitzen at the first
own call or at Drücken. `game-value` compares the bid commitment with
legal values at later calls, pickup and announcement. Unknown Skat cards
can change Spitzen even in Hand games. Game value and the chance of
winning are separate quantities.

## Counting aid and quiz (D-64)

Two independent settings, both off by default and separate from the coach
level (menu "Assistance"). Changing them or the language never changes the
game. Everything is derived from the human's permitted view
(`skat-analysis::counting`); hidden cards, an unseen Skat and the opponents'
hands cannot enter. God Mode's open table does not change this: the aid
still counts only what the permitted view proves.

**Counting aid.** Exact facts, no estimates. Counts include the trick on
the table.

- *Played*: cards laid in tricks.
- *Trumpf outside*: Trumpf neither in your hand, nor played, nor in your own
  discard. Revealed Ouvert Trumpf count as outside (they are still to be
  played); an unknown Skat can hold some, which the aid says. Null shows no
  Trumpf, Grand counts the four Buben.
- *Augen*: won in completed tricks by the declarer and by the two defenders
  (Skat not included); outstanding in cards neither in your hand, nor played,
  nor in your own discard; the discard's Augen when known.
- *Voids*: a Trumpf or side-suit class in which an opponent failed to follow.

**Quiz.** After the third and the sixth completed trick, at the human's next
card-play decision, one question is asked. Kinds: Trumpf outside, Augen won
by the declarer or by the defenders, or cards of one class played (Trumpf or
a suit; no Trumpf and no Augen question in Null). The kind is chosen from the
applicable ones with a seed of the series (own stream), game number and trick
count, so the same series asks the same question regardless of aid or coach
settings. The question stores the knowledge it was asked from; the expected
answer and the derivation shown afterwards come from that snapshot only, so
unknowable hidden cards are never graded.

While a question is pending the decision is held: game actions and coach
requests are rejected (`QuestionPending`), the aid and the whole coach view
are withheld (no hint, note or debriefing can reveal the answer). The exact
aid can be restored explicitly as described below; this marks the opportunity
as assisted and keeps the decision held. The player
answers (a number), skips (the exact count and derivation are shown) or
switches the quiz off; none of these is a game action or costs the turn.
Switching it off drops the question and does not ask again in that trick
count. The result stays visible until the next action. Outside pending
questions, the aid and the coach work independently of each other. No claim
is made about learning efficacy; practice sessions and history are later
work (M8).

## Focused retrieval (#48, extension to #11)

Enable counting questions, then optionally select **Focused retrieval** in
Assistance. It reuses the M6 schedule (at most two opportunities per game),
frozen permitted snapshot and answer/skip lifecycle. No extra quiz runs
alongside M6. The two exact lesson families are `remaining-trumpf@1` and
`proven-void@1`: ask for Trumpf outside, or the number of **other seats**
publicly proven void in one follow class. The latter is offered only with
at least one public proof. It does not ask where an unseen card is or what
to play. Farbspiel and Grand support both families; Null supports suit
voids only and asks nothing if none is proven. Hand, known discards and
Ouvert follow the same exact-count definitions above. Model estimates
never select an expected answer.

Three explicit support levels apply to each opportunity:

- **Counting aid visible** shows the frozen exact aid with the question.
- **Count mentally, reveal on request** invites mental recall and lets the
  player reveal the frozen aid or open **Enter a count** to submit a number.
- **Answer before reveal** invites a numeric answer before feedback.
  The same explicit **Show help** escape remains available.

The two hidden levels withhold every coach panel and the counting aid
until answer/skip or an explicit help request. Restoring help leaves the
question pending and marks its support as assisted, even if settings are
changed afterwards. Changing to visible aid is also an explicit help
transition. Coach preferences are never changed; ordinary hints, warning
checks and historical reviews resume outside the question hold. Earlier
exposure is not tracked: an unassisted response here means no exact aid
was exposed **during this opportunity**, not a new unseen task or proof
of retention. Nothing fades automatically or infers mastery.

`AppEvent::Retrieval` is the plain-data progress boundary: `Opportunity`,
`Help`, `Answer { given, correct }` and `Skip { disabled }`. Each carries
`QuestionId` (independent quiz-stream identity, game number, trick count),
the frozen question, selected assistance level and a sticky `assisted`
flag. Consumers deduplicate by `(QuestionId, action variant tag)`; repeat
help and settings commands emit no duplicate transitions, and a settled
question rejects another answer. Disabling a pending quiz emits one skip
without exposing its expected answer. All events support serde; they need
no storage, replay service or model. Existing `AppEvent::Quiz` feedback is
retained for callers. No efficacy or mastery claim follows from these events.

The legal replay fixtures in `skat-analysis/tests/retrieval.rs` cover both
roles and Farbspiel/Grand/Null. `counting_fixtures.rs` additionally covers
known discards, Hand and Ouvert. `skat-app/tests/assist.rs` checks frozen
help, delayed responses, settings and late coach results, stable event IDs,
serde, repeated commands and unchanged complete-series scores. The GUI's
`focused_retrieval_help_and_levels_work_in_both_languages` test clicks
settings, help and answer controls in de/en. Headless screenshot scenarios
`retrieval-question`, `retrieval-request`, `retrieval-help`,
`retrieval-visible` and `retrieval-void` show the actual flow, with question,
help and void English variants and a narrow question view. These checks
support technical behaviour; learner UAT and retention remain unmeasured.

## Policies (D-48)

- **Equivalent**: within 1 point (`TIE_POINTS`) or within the noise margin,
  2 combined standard errors of the expected score (`MARGIN_SE`).
- **Weak**: at least 10 Seeger-Fabian points (`WEAK_POINTS`) below the best
  option *and* beyond the noise margin.
- Drücken options dropped after screening are `Screened`, never weak.
- **Important decision**: the analysis is complete, at least two options
  are compared, and a *natural* option is weak. Natural: every option
  except jump bids above the next bid value and Schneider/Schwarz
  announcements. An engine judgement only: automatic teaching also needs
  pedagogical eligibility (D-58).
- **Warning**: only with a complete analysis and a weak chosen action;
  independent of lessons and focus.

## No telling ahead (D-49)

A request first reveals the goal, facts and inferences. The second stage
shows legal options in neutral order and a qualitative hint. With figures
explicitly enabled, a further request reveals estimates for every option.
This can disclose the model's preference (D-60). Earlier stages keep
option-specific judgements for the review after the player acts.

## Focus and teachable moments (D-58)

The player can choose one learning focus or skip an automatic hint.
Automatic teaching requires an applicable lesson. After the game, up to
three moments show the chosen move, the model's judgement and a supported
lesson when available. Selection considers focus, relevance and variety.
Details expose comparisons and assumptions. A fallback reports the model
comparison without inventing a strategic explanation.

## Decision snapshots (D-59)

The session captures a permitted `DecisionContext` for each reviewed move.
The debriefing uses its hand, current trick, role, bid and visible history.
Reconstructing the display from the final deal would leak later information.

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
- God Mode (D-62): `coach_job` also carries the real deal
  (`CoachJob::sees_all_cards`; queued reviews keep the deal of their
  decision) and runs `skat_coach::advise_known`. Stages, notes, warnings,
  debriefing and no telling ahead are unchanged; the basis names the
  known deal.
- The coach seed comes from stream 2 of the series seed, mixed with the
  game number and the view; opponents and deals are untouched. Same view,
  seed and budget give the same explanation.

## Replay comments (D-67)

In the recording window the learner selects one recorded own decision and
asks for a comment. It is the same `review` as the debriefing (chosen
option, compared legal alternative, lesson or fallback, uncertainty, basis)
over the table as the player saw it before the move. The default uses only
that view. Hindsight is a separate control and stays labelled as a model
comparison with cards unknown then, not proof of a mistake. Lesson evidence
still comes from the permitted view; in hindsight only the option comparison
uses the real deal, as in God Mode (D-62). No lesson or claim was added, so
the [evidence checklist](coach-validation.md) is unchanged. Nothing is computed
automatically; whole-recording ranking is not offered.

## Latency

The coach runs one `analyze` with `Budget::INTERACTIVE` per human
decision; `explain` is a single pass over the options. Its latency is the
analysis latency, with the interactive target and benchmark procedure
in [analysis.md](analysis.md#latency-target-and-benchmark).
`skat-app/examples/coach_latency.rs` measures it as the frontend sees it:
a session with the coach on "every move", time from `coach_job` to an
accepted `finish_coach`, the human following the coach's best option,
Medium opponents, Kontra/Re on, series seeds 1, 2, …, 10 positions per
kind:

```sh
cargo run --locked --release -p skat-app --example coach_latency
```

## Experimental live preview

With the Experimental opponents selected, card play and Kontra/Re use a
200 ms preview deadline. The live coach reports completed worlds and the
planned count in the currently permitted hint stage. When no world has
finished yet, it shows the known facts and updates again as soon as one
world finishes. Faster worlds can be overrepresented in a preview. Each
world contributes only after all its options have been evaluated. The
search continues with its original seed, budget and workers.

Only the final result enters the warning check, notes and debriefing. A
preview for a different decision or budget is discarded. The final estimate
combines worlds in index order. Timing and thread scheduling therefore
affect the preview while the final result remains reproducible.
