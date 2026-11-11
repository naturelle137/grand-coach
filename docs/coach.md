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
