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
an `OpportunityId`, in play `Game(QuestionId)` (independent quiz-stream
identity, game number, trick count),
the frozen question, selected assistance level and a sticky `assisted`
flag. Consumers deduplicate by `(OpportunityId, action variant tag)`; repeat
help and settings commands emit no duplicate transitions, and a settled
question rejects another answer. Disabling a pending quiz emits one skip
without exposing its expected answer. All events support serde; they need
no storage, replay service or model. Existing `AppEvent::Quiz` feedback is
retained for callers. No efficacy or mastery claim follows from these events.

### Focused counting training (D-71)

**Game → Counting training …** practises the same two families outside a
game. The learner chooses a support level and a topic, then sees one fixed
legal position from one seat: contract and bid, their seat and role, every
completed trick with seats and winner, the trick on the table, their hand
and their own discard (declarer) or that the Skat is unknown. The question,
help, expected answer and feedback are those of the in-game focused
retrieval, derived from that seat's view only. For voids, help and feedback
name only the seats with public proof (failure to follow); an unknown Skat
is stated as a possible place of outside Trumpf, unseen holders are never
named. No card recommendation, strategic advice or diagnosis is given. After
answering or skipping, **Try again** presents the same position as a new,
labelled round and **Next position** the topic's other position; the topic
can be switched at any time and **Back to the game** returns to the live
table exactly as it was. Initial fixtures: each family has one declarer and
one defender position, one Farbspiel and one Grand; the defender void
position is the moment right after the declarer failed to follow, one card
after nothing was proven. Null is excluded from the Trumpf topic because it
has no Trumpf. Fixed positions and repeats are practice, never transfer
evidence. See
[the contract](architecture.md#focused-counting-training-d-71).

### Catalogue focus sessions (D-73)

The same training entry also offers remaining Trumpf, proven voids, side-card
vulnerability and partner overtaking across the twelve accepted catalogue
positions. Each family has intro, changed and boundary cases. Learners submit
an exact count or judge a hypothetical legal card's narrow mechanism as
possible, certain or absent. Feedback uses shared analysis; question correctness
is separate from optimal card choice. Before answer or explicit help/skip, no
answer, counter or preferred plan is shown. Every legal alternative stays valid.

History records exposure before presenting the task, even if the learner exits
unanswered. Assistance is sticky. A changed case can carry app-local unseen
eligibility only before its first exposure in complete, available history;
repeats and uncertain exposure are practice. Missing, migrated, reset, corrupt
or unsupported histories cannot establish novelty. Atomic-save failure withholds
new positions. No human efficacy/transfer claim follows. Retry, next, topic choice
and return reuse the isolated training lifecycle; normal play waits unchanged.
See [session behaviour and limitations](practice-catalogue.md#session-behaviour-and-conservative-exposure).

### Learning history (D-70)

**Game → Learning history …** shows what the local
[learning history](architecture.md#learning-history-d-70) recorded from
these events: per topic (remaining Trumpf, proven voids, Augen won, cards
played) and context (series, practice, practice retry, counting training,
training repeat) the questions asked,
answers (exact), answers without help (exact), help shown and skips. Every
column counts its own event type; an answer counts as without help only if
no exact aid was available during its opportunity, so visible-aid and
restored-help answers never do. Practice attempts and intentional retries of
the same deal are listed apart from series games, and so are training
presentations and their repeats, so a repeated deal or fixed training
position is never presented as a new unaided task. Without observations the window says so;
below 20 questions it states that the sample is too small for any
conclusion. It always states that the counts are not a measure of skill or
understanding.

Below, answers are listed by observed result per topic and context: exact,
too high, too low, wrong with the value not stored (answers saved before
format 3) and skipped, each as "without help + with help". Per topic,
from 20 answers or skips on, the 10 before the last 10 are set beside the
last 10, numbered in saving order with their attempt range and places;
below that the window states how many there are and that 20 are needed.
The question rules (lesson family and version) recorded with each question
are listed, with older unversioned questions counted apart. The window says
that categories describe the answer, not its reason, and that a difference
between windows is no trend, progress or transfer. No mastery, thought,
game-loss diagnosis, recommendation or strategic error classification is
derived; card-play decisions are not classified (remaining #13/#49 scope).

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
This can disclose the model's preference (D-60). Outside the deliberately entered
guided dialogue below, earlier stages keep
option-specific judgements for the review after the player acts.

## Guided Farbspiel dialogue (#131)

**Think through this decision together** deliberately enters one decision's
orientation, check, candidate attempt, worked rule answer and actual observation.
It never starts automatically, is unavailable with coach off or a pending quiz,
and can be skipped back to free play at any point. Early prompts, automatic
hints and unaided tasks continue to conceal the preferred move. A correct
candidate attempt or deliberate reveal (including “I don't know”) reveals the
actual shared Easy card, not a search recommendation. All legal cards remain
clickable throughout; answering never plays a card. Wrong answers stay on the
current question and allow retry or reveal. No strategic optimum is claimed.

The initial supported rule families are `UnderPartner` and `WinLast` in
Farbspiel card play with at least two legal actions and a nonempty branch
candidate set. The former follows a partner who is not a follow-class master
with one seat still to play; Easy selects a losing card by nontrump, Augen,
power, retaining the first exact tie. The latter wins in last seat by nontrump,
power, retaining the first exact tie. Other branches, Grand, Null and pre-play
report unsupported and return to free play. In particular, a side master is
not safe against trumping: the early `FeedPartner` branch is excluded.

One active prompt has at most two supporting facts. Rule conditions and
candidates appear only after an attempt or deliberate reveal. Optional card
memory starts hidden, cannot open before the worked answer, and closes while
an observation question is active. It uses frozen `Knowledge` and M6 counts:
own hand, known own Skat, public played cards and unseen cards; outside Trumpf
excludes own hand, played and known own Skat. Unknown Skat stays unknown; no
unseen holder is assigned, including at an open/God table. Reveal is assistance,
not unaided learning evidence. This slice adds no persistent progress store. `AppEvent::Guided` is the plain
in-memory handoff: unique entry/decision ID, monotonic sequence, explicit
action, resulting step and sticky assisted flag. Consumers deduplicate by
`(id, sequence)`; no mastery or retention inference follows.

The app follows the originating trick across automated turns. For
`UnderPartner`, the later player's actual card is the observation; for
`WinLast`, the human's actual completing card is. Evidence carries trick index,
seat, led card, played card and the actual completed winner, even if the human
chose a different card. Failure to follow establishes a void at that play;
a Bube follows Trumpf in Farbspiel, never its printed suit. Following accounts
for that card only. Observation deductions are hidden until an answer/reveal.
A new human move, new game, changed assistance, coach off, replay entry or
practice session cancels the old dialogue. Each entry has its own identity;
responses to another entry/session are rejected.

After observation feedback, enabled figures plus a separate request may show
an available analysis of the original decision, with its objective, budget,
finished/planned distributions, uncertainty and own-view/real-deal basis.
Missing estimates remain unavailable; incomplete estimates remain provisional.
No model number is invented. Search agreement or disagreement never changes
the frozen rule explanation. God Mode's statistics explicitly retain their
privileged basis, while the rule dialogue still uses the closed permitted view.
The [walkthrough and fixture contract](guided-dialogue.md) records technical
evidence and remaining learner work.

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
- Open display (D-68): the table shows every card, but `coach_job` keeps
  only the human's view (`CoachBasis::OwnView`), exactly as at a closed
  table; the frontend labels this basis. No new lesson or claim.
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
automatically.

## Recording review (D-69)

For a completed recording the learner can ask for a review of all own
decisions. Each decision with a choice gets the historical comment above;
the debriefing's D-58 selection then shows zero to three moments in game
order, each with the table before the move, the choice, the compared legal
alternative and the lesson, and a link to that decision. Same lessons, same
thresholds, no hindsight; the focus only reselects. Partial reviews
(cancelled, time limit, unavailable decisions) say how many decisions were
evaluated and that the moments do not cover the whole game. Fewer or no
moments come with an explanation and no judgement of the game. No new lesson
or claim, so the [evidence checklist](coach-validation.md) is unchanged.
[Contract and budgets](architecture.md#bounded-recording-review-d-69).

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

## Observed mechanism responses (D-74)

The bilingual learning overview describes explicit conditional Abstich and
partner-overtaking answers from catalogue training. It separates possible from
certain, denied possibility and false presence, with assistance and app-local
exposure context. Task/version sample denominators include unanswered cases;
comparisons require twenty settlements in the exact same context. These
[response patterns](mechanism-responses.md) suggest what to practise only by
showing observed mismatches, without inferring played-card errors, thoughts,
mastery, or ordinary-game strategic weaknesses. Counting stays separate.
