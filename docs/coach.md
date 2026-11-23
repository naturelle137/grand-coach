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
winning are separate quantities. The post-move bidding lesson explicitly labels
its minimum commitment as belonging to the position before the decision;
a higher jump commits to its actual bid, not that historical minimum.
The lesson reuses the original shared analysis without another evaluation.

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

### Reserved validation (D-75)

From the training chooser, **Prepare learner validation** leads to a separate
confirmation screen for Initial or Delayed. Each presents four reserved legal
positions, one per family; ordinary family practice and galleries never show
them. Feedback stays hidden until a bounded answer or requested help/skip.
External prior exposure is explicitly none/seen/unknown; own words and clarity
are optional verbatim fields. Unclear, skip and exit remain distinct. Help and
retries are sticky practice; unexpected further aid records assistance and
stops. Failed durable saving blocks further feedback/tasks. A separate private
build-linked kit supplies German instructions, hashes and a blank local record.
The [preregistration](validation54.md) and independent agent key precede human
results. Neither technical tests nor local exposure prove learning efficacy.

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
power, retaining the first exact tie. Non-Ouvert Grand supports `WinLast`
(#157), for a defender in the middle seat `UnderPartner` (#159), and
`WinEarly` in Mittelhand (#161), and last-seat `FeedPartner` (#163). The
revealed Grand Ouvert declarer also supports [second-seat `WinEarly`](guided-dialogue.md#grand-ouvert-declarer-winearly-311).
The revealed Farbspiel Ouvert declarer's lead also permits the second-seat
defender's [guarded `WinEarly` dialogue](guided-dialogue.md#ouvert-farbspiel-declarer-acting-second-317).
[Second-seat Grand `FeedPartner`](guided-dialogue.md#grand-second-seat-feedpartner-131)
also admits the acting defender after a partner follow-class master. Empty-trick Grand and non-Ouvert Farbspiel `LeadMaster` (#171, #182)
also support both roles where Easy selects a nontrump master: own/played
higher cards do not block it, known discarded higher cards still do.
It takes maximum Augen, last exact legal-order tie, with no trick guarantee.
`Source::Lead` has no current winning card; observation retains both actual
replies and the completed individual winner. The declarer's own empty-trick
`DrawTrumpf` lead (#175) is also admitted, see
[its scope](guided-dialogue.md#grand-declarer-drawtrumpf-lead-175).
Farbspiel also admits last-seat defender `FeedPartner` (#185), including a
validated [Ouvert view](guided-dialogue.md#farbspiel-ouvert-last-seat-feedpartner-131),
with the partner currently ahead, exact ordered pure core-legal actions and
consistent turn/leader and Hand/pickup information. This family uses the
permitted view, including original-decision estimates, even in God Mode.
FeedPartner considers all legal cards, including overtakes, maximises
`(!is_trump, Augen)` and keeps the last exact legal-order tie without power. WinEarly retains all current winners as
candidates and explains only the selected card: Trumpf or a follow-class
master. Farbspiel `WinEarly` (#178) uses the same rule with strict
non-Ouvert current-second-seat admission, one contiguous public card and an
exact ordered core-legal list. It checks consistent Hand/pickup information.
All Buben and the announced suit are Trumpf, regardless of printed suit.
A different own master does not qualify a cheaper non-master winner.
A side master can be trumped; a Bube can be overtrumped. Grand has
only four Buben as Trumpf. Non-Ouvert Null (#165/#169) supports the human
declarer second or last to the current trick, with two or more pure card actions and a
present loser. The opening lead (#176, below) uses a separate admission and
minimum-cost task. In nonempty tricks Easy maximises Null power among current losers and retains the
last exact tie in legal order. Buben follow their printed suit, with ranks
7/8/9/10/Bube/Dame/König/Ass; off-suit cards cannot win the led suit.
A current loser in second seat cannot win this trick for the declarer; the
remaining defender can change which defender wins. The actual third card
and individual winner are observed, rather than predicted. Winning alternatives
are named after disclosure and stay playable. No
optimality, later lead or hidden holder is claimed. Other unsupported Grand/Null branches return to free play; the supported
Ouvert branches are listed in the walkthrough. Kontra/Re has its own
dialogue below. In particular, a side master is
not safe against trumping: second-seat Grand `FeedPartner` explicitly teaches
this limit and observes the remaining declarer. [Second-seat Farbspiel `FeedPartner`](guided-dialogue.md#farbspiel-second-seat-feedpartner-131) uses the same admission, where the declarer may trump with any Trumpf.

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
`UnderPartner`, `WinEarly`, early Grand `FeedPartner` and second-seat Null, the later player's actual card is the observation; for
`WinLast`, last-seat Null and last-seat `FeedPartner`, the human's actual completing card is. Evidence carries trick index,
seat, led card, played card and the actual completed winner, even if the human
chose a different card. Failure to follow establishes a void at that play;
a Bube follows Trumpf in Farbspiel/Grand, its printed suit in Null. Following accounts
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

### Guided Reizen threshold (#148)

The same entry also supports every learner Reizen decision. Its evidence is
`skat_analysis::policy::bid_rule`, the exact path Easy now uses: actual
Hold/Bid/Pass, public current bid, whether the seat holds, the value at stake
(current bid when holding, otherwise the next standard value or none), the
heuristic ceiling, candidate game and `GameBranch` (`Suit`, `Grand`, `Null`,
`Weak`). The coach only checks answers against it and depends on neither
`skat-ai` nor search. One prompt at a time: first the public current bid
(never a pre-jump value) and whether this seat holds or names a bid; then
whether the value at stake is within Easy's ceiling for the own ten cards.
Prompts reveal neither ceiling, game nor action. A correct threshold attempt
or deliberate reveal shows Easy's action, the candidate game and ceiling, the
branch formula, the stake comparison and a caveat: the ceiling is a
conditional, approximate rule of thumb from the own hand, not a guaranteed
contract value or winning promise, and the hidden Skat cannot supply missing
Spitzen. Null and Grand are named as such; their card-play scope is defined
by the guided card-play section above.
Memory and estimates are unavailable here. All legal calls stay on the table;
any call, a new game, coach off or replay ends the dialogue, and old
responses are rejected.

### Guided Skataufnahme and Drücken (#131 phase 4)

The same entry supports the declarer's Hand-or-pickup and discard decisions.
Evidence is `skat_analysis::policy::pickup_rule` (actual Hand/TakeSkat,
candidate game and `GameBranch`, strength for that game, Hand threshold 100)
and `policy::discard_rule` (actual two cards, candidate game for the twelve
cards, a `Keep` reason per discarded card, every card tied with Easy's second
discard, fallback flag); Easy calls both. One prompt at a time: first which
branch builds the candidate (Grand, suit game, Null, none); then, at pickup,
whether the strength reaches the Hand threshold, or, at Drücken, one card Easy
puts away first (a card tied with Easy's discard also counts). Prompts show the
candidate game only after orientation and never the strength, the discard or
the action. A correct attempt or deliberate reveal shows Easy's action, branch,
strength comparison and count (pickup) or the keep order and per-card reason
(Drücken), the disclosed fallback if Easy's action was not legal, and a caveat:
a conditional rule of thumb from the own cards, the hidden Skat stays unknown,
no promise to win. Both legal pickup actions and every legal discard stay free;
any action ends the dialogue. Memory and estimates are unavailable here.

### Guided Ansage (#151)

The same entry supports the declarer's announcement. Evidence is
`skat_analysis::policy::announce_rule`, the path Easy calls: the preferred game
and `GameBranch` of `choose_game` for the ten cards plus the own known Skat
(empty for Hand; the hidden Skat is never used), its plain value against the
public bid, the suit games and Grand with value, strength and fit, the
announced game (preferred if it fits, else the strongest fitting candidate, the
later in the order Kreuz, Pik, Herz, Karo, Grand on ties, else Grand), the Null
unsafe-suit count and ouvert wish, and three disclosed fallbacks: no game fits
(Grand; the actual value and an overbid stay unknown), Null ouvert unavailable (plain Null) and the rule's
announcement not legal (first legal action). Before the reveal only the public
bid and Hand state are shown; orientation asks the branch, then, with the
preferred game named, whether its value reaches the bid. A correct attempt or
deliberate reveal shows Easy's announcement, the counted Skat, the comparison,
the candidates and tie or fallback, and a caveat (conditional rule of thumb, no
promise to win, no rating of the announcement). Every legal announcement stays
free; any action ends the dialogue. Matching Easy says nothing about strategic
quality.

### Guided Kontra and Re (#153, #155)

The same entry supports a core-offered Kontra at a non-Ouvert Farbspiel,
Grand or Null card turn and the declarer's immediate Re reply, including
Null Ouvert Re. Evidence is
`skat_analysis::baseline::kontra_rule`, the path Easy calls: the call (and so
the role), contract, public level, the own Trumpf count against 5 (never in
Null) or the strength of the own hand plus the known own Skat against 90, the
exact card Easy plays when it declines, legal actions and fallback. Only core
eligibility and public facts decide whether the lesson starts; no hidden hand
or silent bidding is inferred. Before the reveal only the contract and level
are shown; orientation asks the factor after the call (×2, ×4), then, with
factor and threshold named, whether the condition holds. A correct attempt or
deliberate reveal shows Easy's action, the comparison, the counted Skat, the
declining card without a tactic claim, and two facts: the factor multiplies
the written game points without changing whether the bid is reached, and the
condition is a heuristic, not proof of a winning defence or a probability.
Every card and call stays free; declining in card play means playing a card,
`KeinKontra` exists only in the Ouvert round. After the learner's action the
dialogue waits for the actual declarer reply, the resumed card or the
declining card itself, across automated turns. The separate Ouvert round and restricted views report their own unsupported text. Memory is unavailable;
estimates stay separately requested.

Grand uses the same count questions: only the four Buben are Trumpf, so a
defender holds at most four and Easy's five-Trumpf Kontra condition never
holds; Re compares the own hand plus known own Skat with 90 as in Farbspiel.
The worked answer calls this a limit of the rule of thumb, not a Skat rule:
Kontra stays allowed. Null has no Trumpf and Easy's strength is zero, so the
threshold step asks only whether the generic rule supplies an assessment of
a Null game (`Answer::Applies`, correct: no); no count, threshold or
strength comparison is shown, and the caveat says that Kontra and Re stay
allowed. Grand and Null card-play tactics, including the tactic of Easy's
declining card, stay outside the lesson.

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
  first, then the oldest queued one (`CoachJob::is_review`) by default.
  While the level is **off**, queued reviews are not handed out (no work for
  a hidden debriefing); they resume if the coach is turned on again and are
  still dropped by the next game. The optional learned ordering below changes
  only review priority. The
  frontend uses a current-decision lane and a review lane. The oldest review
  continues through foreground changes; a displaced foreground analysis may
  occupy the review lane only if it is that oldest review. Otherwise it is
  cancelled and retains its slot until exit. At most two live/practice coach
  computations run, including cancelled and parked work. A cancelled review stays
  queued; coach-off cancels both lanes, and the next game drops the queue. `Session::coach_result` exposes the full analysis for tools and
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

## Experimental learned debrief ordering (#78)

Settings → Coach offers **Experimental: prioritize debriefing with model
estimates**, off by default. It uses the existing bundled CPU model and
changes only which queued review runs next. `NeuralModel::review_priority`
uses the estimated own Seeger-Fabian score difference between the best legal
option and the recorded action. This number is an uncalibrated model estimate,
never a fact, mistake, human choice, Monte-Carlo sample or lesson predicate.
No learned numbers are displayed or passed to `Explanation`, lesson evidence,
`exact` or `Comparison::Reliable`; ordinary analysis supplies every shown moment.
The bilingual setting and tooltip state that boundary. Existing exact facts,
voids, lessons and D-49/D-60 staged figures remain authoritative.

No decision is filtered out, including small estimated gaps. Unsupported,
invalid and privileged views retain FIFO priority ahead of estimated cases;
God Mode never sends privileged cards to the network. Equal estimates retain
original queue order. Missing models leave Classic selected. Each captured
view/action has one cached priority under the selected model SHA-256; changing
models or disabling the option clears those priorities. Ordinary analysis jobs
and results contain no learned values and stay valid across an ordering-model
switch. Decision, lineage and budget checks, cancellations, current-decision
priority and next-game queue disposal retain D-59's lifecycle. A cancelled
review remains queued. No new training, weights, arena or compute permission
is introduced.

While either queue drains, the debriefing shows up to three **already analysed**
moments, with their original snapshots and a bilingual notice that selection
can change. These use the unchanged lesson/focus/diversity selection over the
finished reviews. The complete selection is identical to FIFO on the same
trace; an early shortlist is not the final selection. This classical incremental
presentation is the scheduling control for the learned experiment.

### Fast-play measurement

The original benchmark waits at every decision. The additional command below
plays a complete game without waiting for any coach analysis, then drains the
actual backlog. Each pair clones the same captured session; opponents are Easy,
the human passes in Reizen and otherwise takes the first legal action. It
reports peak pending decisions, one explicitly scripted cancelled review,
time from trace start to the first visible supported moment and to the complete
debriefing. No-moment traces report `none`, not zero latency. Order alternates
between modes across seeds. All decisions must finish and the complete moments,
snapshots and evidence must compare exactly equal. This maximal-backlog control
does not simulate human think time, worker overlap or a cancellation rate.

```sh
cargo run --locked --release -p skat-app --example coach_latency -- --fast-play 6 20000
```

Measured on 2026-11-18 in `skat-dev`, Linux x86-64, CPU release build, bundled
PPO v2 4×384 model SHA-256
`6eeb33593eb1e0a8fc91eb2774488a3e9729fb28a45e76b8812e37bf569def75`.
Host workload and CPU scheduling were not isolated. The explicit bounded diagnostic budget was 24 samples × 20,000 nodes/task,
not the interactive 100,000,000-node budget; budget-exhausted results retain
ordinary uncertainty. Model load was 25.682 ms, excluded from the paired times;
trace capture, priority inference, cancellation, analysis and view building are
included. Six paired seeds each queued 11 decisions and completed three moments,
with zero missed final moments and exact final-evidence equality.

| Seed | Classic first / full (ms) | Learned first / full (ms) |
|------|---------------------------|--------------------------|
| 1 | 5.415 / 27.988 | 6.548 / 28.678 |
| 2 | 4.923 / 23.260 | 6.735 / 25.822 |
| 3 | 5.036 / 28.170 | 6.835 / 29.644 |
| 4 | 5.225 / 25.936 | 6.341 / 26.650 |
| 5 | 5.348 / 25.151 | 6.828 / 26.880 |
| 6 | 4.820 / 27.644 | 6.325 / 29.416 |

Before incremental presentation, a separate original-FIFO process on seeds
1–3 measured first/full 335.364/335.367, 278.172/278.180 and
375.825/375.829 ms at the same bounded budget. That process includes different
cold-start/scheduling effects; it does not establish a speedup ratio. Within
any paired row, the former final-only presentation would first show a moment
at that row's full-debrief time. Incremental Classic therefore already supplies
the early moment without network inference. **Learned ordering was slower on
these short traces and stays opt-in and off by default.** This diagnostic
establishes the implementation and no-drop behaviour, not reference-machine
interactive latency, model calibration, learner efficacy or a general speedup.

### Classical bottleneck diagnostic (#72)

The fast-play command also reports the sum of wall time inside completed
`CoachJob::run` calls, excluding the scripted cancellation. On 2026-11-18,
at source `910971c` plus the diagnostic timers, the same six-seed,
24 × 20,000-node command ran in
`skat-dev` on an i9-10900K with an unisolated host. Classic execution and
full trace times in milliseconds were respectively: 109.826/117.160,
79.534/87.659, 103.070/110.427, 85.403/93.472, 95.049/103.660 and
91.416/97.601. All eleven reviews completed in each trace, with three final
moments and exact equality to the paired learned-order results. The execution
interval includes analysis and explanation; it does not isolate their costs.
The full interval also includes game capture, cancellation and view building.
These are wall times, not CPU time, and timer calls add measurement overhead.

The command also counts distinct review inputs (`CoachJob::input_identity`:
permitted view, God-Mode deal only when present, budget, seed and mode,
without the decision key). On the same six seeds each trace had 11 distinct
inputs of 11 reviews, so zero results were reusable. A repeated rerun of the
command in the same unisolated environment later measured Classic review
execution of 22.8–33.8 ms per trace (full 24.7–35.8 ms), so the host noise
between runs exceeds 3×; neither set of wall times supports a ratio.

A second paired, serial control can be run with `cargo run --locked --release -p
skat-app --example coach_latency -- --paced-play 6 20000`. On 2026-11-18,
for seeds 1–6, each pair used the same fixed actions and produced exactly equal
final moments and evidence. All 11 inputs per game were distinct in both
modes. The fast mode deferred all 11 reviews; the paced mode completed each
review after its action, leaving none pending at the end. Fast total times were
23.297–27.862 ms; paced times were 22.931–28.747 ms. This serial approximation
does not model a human delay, worker overlap or the GUI. A separate warm repeat
of the whole paired process under `/usr/bin/time -v` used 1.33 s user and
0.14 s system CPU and peaked at 9,488 KiB RSS; these are process totals for
both modes, not per-mode resource comparisons. The host was not isolated and
other builds were active, so no speed ratio is inferred.

**Decision (bounded no-go for identical-input reuse, #72):** The measured
trace contains no duplicate work for a cache of completed immutable analyses
to remove: every review has a different permitted history. Cache hits could
arise only when an identical position is analysed again (a replay with
identical actions), which is not a measured bottleneck and would need a key over
the full permitted history, perspective, seed, budget, versions and rules plus
stale-result and hidden-card tests that no measured gain justifies. A worker or
scheduling policy has no demonstrated target either: execution is serial and
dominates, and the first visible moment (about 5 ms) already precedes the full
debrief. Classic, moment selection and results stay unchanged. Real paced worker overlap, CPU-contention, per-mode RSS and GUI-responsiveness
comparisons were not run. This no-go covers identical-input reuse in these
traces, not other candidate optimizations; it is reopened if repeated identical
inputs appear.

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

## Null opening lead (#176)

The [bounded opening-lead dialogue](guided-dialogue.md#null-opening-lead-176)
uses the unchanged shared Null minimum-cost rule, FIRST exact tie. Only the
Vorhand declarer's own first lead is admitted, including Null Ouvert after
the hand is revealed; Augen are legacy
sorting cost, not Null scoring. All legal leads remain available. Empty-table
candidate status never predicts either defender reply or completed winner.

Default-feature `cargo test -p skat-analysis --test null_lead_policy` checks
accepted1695's complete literal inputs/all eleven fields and all 286 first
tricks (94 immediate Null losses); `skat-coach --test guided_null_lead` and
app tests matching `null_lead` cover full non-absent lessons/dialogues, both
hidden substitutions including God Mode, legal alternatives, actual two replies,
individual winner, consent, stale/reentry/replay/reset and original estimates.
Run `cargo test -p gui-egui --test null_lead -- --include-ignored --nocapture`
in skat-dev for ordinary widgets and labelled component fixtures in both
languages/sizes. `SKAT_NULL_LEAD_WIDGET_FIXTURES_OUT` intentionally regenerates
the component JSON from checked app sessions; ordinary renders never inject it.
Source-derived `null-lead-inventory.json` joins the existing both-OS gallery.
Push gate and the configured workspace non-GPU/include-ignored suite remain
required, including nn-train and vulkan-search. Terminal owns review and final
actual-main/both-OS archive/hash/visual acceptance. No learner results follow.

## Explicit live Abstich responses (#13)

A separate deliberate [live question](live-abstich.md) uses shared positive
Abstich evidence for a learner-selected hypothetical legal card. It conceals
answers until an explicit response or help, preserves the game and records
assistance-aware observations separately from counting and catalogue training.
Empty analysis is unsupported, never an absence reference or strategic diagnosis.

### Worker and contention probe

`coach_latency --worker-play DELAY_MS GAMES NODES_PER_TASK` runs fixed
Pass/first-legal actions with a real coach thread. Before each action it waits
for the chosen delay; an unfinished worker continues while the foreground runs
Easy opponents. At the next decision the probe requests cancellation, waits
for that worker and later drains the reviews. It counts actual overlaps of the
worker execution and foreground opponent intervals, cancellation **requests**
(which can race completion), and reviews drained at game end. Unlike egui it
joins a cancelled worker and does not run queued reviews between turns. Thus it
is an application-API probe, not a measurement of the GUI scheduler or actual
human think times. `COACH_TRACE_EVIDENCE=1` additionally prints the complete
final moments as JSON for equality checks; it includes the ordinary permitted
snapshots and is intended for local diagnostic data only.

On 2026-11-18, a release build in `skat-dev`, i9-10900K, seeds 1–6,
24 samples and 20,000 nodes/task ran on one CPU. A second Python busy loop
pinned to that same CPU supplied the controlled contention; other host activity
was not isolated. Each cell is one process for six games, measured by
`/usr/bin/time`; CPU is user + system, RSS is peak process RSS. First/full are
ranges in ms from trace start to first visible moment at or after game end and
complete debrief; opponent time is the sum per game, not GUI input latency.

| Delay | Extra load | Overlaps per game | Cancel requests per game | Drained per game | First / full ms | CPU s | RSS KiB | Opponent ms/game |
|---|---|---|---|---|---|---|---|---|
| 0 ms | none | 0–1 | 11 | 11 | 29.9–59.4 / 76.1–152.4 | 0.63 | 4,924 | 0.085–0.164 |
| 5 ms | none | 4–7 | 4–7 | 4–6 | 56.7–69.7 / 112.7–174.9 | 0.63 | 5,016 | 0.094–0.162 |
| 0 ms | busy loop | 0 | 11 | 11 | 66.4–106.0 / 172.1–317.0 | 0.74 | 5,056 | 0.086–0.130 |
| 5 ms | busy loop | 6–9 | 6–9 | 6–8 | 64.1–94.7 / 237.6–331.7 | 0.77 | 4,924 | 0.109–0.162 |

A separate unpinned run compared all six fast and paced final moments as JSON
and found them exactly equal. This checks these schedules only, not every
possible interruption. The short fixed pauses and pinned single-core experiment
are synthetic; a single run per cell does not support general speed ratios.
No cache or changed scheduler was measured, and CPU/RSS are not before/after
an optimization.

For the real egui coach path, the ignored headless test
`app::tests::debrief_headless_frame_timing` starts a queued debrief through the
frontend and times 100 harness steps at 1280 × 820 with the same reduced budget.
On the same unisolated host, the worker was present for 46 steps; median, p95
and maximum were 0.830, 0.972 and 2.978 ms. Worker presence does not prove it
was executing during each step. These are headless test-step times, not input,
presentation or physical-display latency. The existing
`window_stays_responsive_while_the_coach_analyses` test also exercises an action
while live analysis is active. Neither measurement establishes responsiveness
at interactive budget on a reference machine.

The separate ignored `app::tests::debrief_paced_gui_worker_probe` exercises the
real egui worker driver, with 0 or 5 ms before each fixed action and Easy
opponents without their usual delay. On the same unisolated host, each of six
seeds played 11 actions per mode; a coach worker handle was present on 8–11
of those actions. All paired final moments and evidence were exactly equal.
The largest headless step per trace was 8.0–16.1 ms; total times were
156.9–196.0 ms fast and 211.0–257.2 ms paced. The worker handle can outlive
actual execution, and this probe does not count preemptions or CPU/RSS per
mode. It does not establish physical UI latency, interactive-budget behaviour
or sustained-foreground starvation.

The completed-input cache no-go remains bounded to these distinct inputs.

### Scheduler candidate study (#72)

`coach_latency --review-play cancel|finish DELAY_MS GAMES NODES_PER_TASK` runs
the historical single-lane egui job policy: the current
decision first, otherwise the oldest queued review. The human acts after a fixed
delay. **cancel** reproduces the pre-control baseline, including its ordering: an
unfinished analysis of an earlier decision gets a cancellation request, the
current decision's analysis starts at once without joining the old worker (the
two may overlap), and the cancelled decision stays queued and restarts from
scratch. A controlled-worker test
(`current_analysis_starts_while_cancelled_worker_still_runs`, run by
`cargo test`) checks that the new analysis starts while a worker that blocks
after cancellation is still running. **finish** is the candidate: the
foreground waits until the running analysis completes. Both end with a drained
queue; with `COACH_TRACE_EVIDENCE=1` the complete final moments of all six
seeds were exactly equal between cancel and finish in every cell below.
`COACH_TRACE_RECORDING=1` also emits the privileged full game recording;
these local diagnostic traces reveal the deal and must not be shared as ordinary
player views. On 2026-11-21, release runs with six seeds, 24 samples and
20,000 nodes/task compared the complete JSON recordings and final moments
pairwise for cancel and finish at both 0 and 5 ms delay: all twelve pairs
matched. These fixed-action Easy-opponent traces do not establish equivalence
for other games or a performance improvement.
The probe now captures the first visible moment at game end or immediately
after accepting the running worker, before waiting for cancelled workers.
The historical measurements below used the later, post-settlement observation,
so their first-moment times can overstate when the moment became visible.

Measured on 2026-11-19, release build, six seeds, 24 samples × 20,000
nodes/task, one run per cell, host not isolated (load average about 10 from
concurrent work). "Contended" pins the process to one CPU shared with a Python
busy loop. Executed and wasted are sums of worker wall time per cell (six
games), wasted being the time of cancelled runs, which now overlap the
following analysis; the largest foreground wait is the longest time from
showing a decision to starting its analysis. CPU is user + system for the
whole process.

| Delay | Load | Policy | Cancelled | Wasted ms | Executed ms | Max fg wait | CPU s | RSS KiB |
|---|---|---|---|---|---|---|---|---|
| 5 ms | free | cancel | 0 | 0 | 147 | < 0.1 ms | 0.70 | 9,508 |
| 5 ms | free | finish | 0 | 0 | 149 | < 0.1 ms | 0.71 | 8,932 |
| 20 ms | free | cancel | 0 | 0 | 158 | < 0.1 ms | 0.71 | 10,432 |
| 20 ms | free | finish | 0 | 0 | 145 | < 0.1 ms | 0.74 | 10,340 |
| 50 ms | free | cancel | 0 | 0 | 148 | < 0.1 ms | 0.76 | 9,356 |
| 50 ms | free | finish | 0 | 0 | 146 | < 0.1 ms | 0.75 | 10,204 |
| 5 ms | contended | cancel | 60 | 315 | 1,119 | < 0.1 ms | 0.56 | 5,704 |
| 5 ms | contended | finish | 0 | 0 | 835 | 46.4 ms | 0.42 | 5,304 |
| 20 ms | contended | cancel | 60 | 1,054 | 1,880 | < 0.1 ms | 0.93 | 5,700 |
| 20 ms | contended | finish | 0 | 0 | 816 | 17.1 ms | 0.42 | 4,984 |

The finish rows' "< 0.1 ms" is the same quantity (the wait is only the time
to start a thread when no review runs); contended finish waits were 17.1 and
46.4 ms. These replace earlier numbers measured with a baseline that joined the
cancelled worker before starting the current analysis, which the GUI does not
do.

Findings, for this host load and these small budgets only. On a free host every
analysis finished within the think time and nothing was cancelled, so the two
policies were indistinguishable. On the contended core the baseline cancelled
60 runs per cell (about 5 ms and 18 ms of worker time each on average) and used
about 0.14–0.51 s more process CPU than the candidate, while the candidate
delayed the next decision's analysis by up to 46 ms, the baseline by under
0.1 ms. The candidate's wait is at most one review's length; these cells did
not measure it at the interactive budget (100,000,000 nodes/task).
`nodes_per_task` is a ceiling, not consumed work, so the budget ratio does not
imply a latency ratio, and no progress signal exists to bound the wait. No
first-versus-full debrief time comparison is drawn: each cell is a single run.
RSS is process-wide. Single-run cells support no ratios; the delay is synthetic,
not human think time, and no GUI, presentation, physical-display latency,
cold-versus-warm cache or interactive-budget run was included in these cells.

On 2026-11-20 the same release diagnostic was also run once per policy for
seed 1, a 5 ms delay and the interactive 24 × 100,000,000-node budget. The
host was unisolated and other builds were running. Cancel requested ten
cancellations, drained nine reviews after play, and reported a maximum
foreground start wait of 0.585 ms, first moment at 1.943 s and completion at
3.279 s. Finish cancelled none, drained none after play, and reported a maximum
foreground start wait of 1.801 s, first moment at 2.653 s and completion at
2.653 s. Both reported three moments; their evidence was not compared in this
run. Process user plus system CPU was 9.77 s versus 9.71 s and peak RSS was
343,420 versus 356,188 KiB respectively. These single, sequential runs cannot
establish a speed ratio or isolate a policy effect. They measure the diagnostic,
not GUI responsiveness, and do not establish cold-cache behaviour.

A subsequent six-seed repeat at the same budget and delay compared the
serialized complete moments and evidence exactly; all six pairs matched. Each
cancel run requested ten cancellations; finish requested none. The maximum
foreground start waits were 0.063–3.137 ms for cancel and 0.664–4.393 s for
finish. First/full times ranged from 0.088/0.931 to 2.232/3.533 s for cancel
and 1.365/1.365 to 7.387/7.387 s for finish (the extrema need not belong to
the same seed). Total process CPU was 29.49 s and 28.68 s; peak process RSS was
641,348 and 483,408 KiB. Runs were sequential on the same busy host, with
cancel first, so these observations neither establish a ratio nor separate
host load from policy effects. They also do not measure GUI presentation.

On 2026-11-21, the same interactive-budget diagnostic additionally compared
both the complete privileged recordings and final moment/evidence JSON for
six seeds at each of 0 and 5 ms delay; all 12 pairs matched. Each cell was a
single sequential process on an unisolated host while a project gate compiled.
At 0 ms, cancel/finish used 28.31/23.88 CPU seconds and peaked at
880,308/479,976 KiB RSS; their largest foreground waits were 2.883 ms/1.569 s.
At 5 ms, they used 28.87/28.07 CPU seconds and peaked at 586,444/467,168 KiB;
their largest foreground waits were 5.513 ms/1.834 s. These process-wide
observations do not isolate policy effects, establish a speed ratio, or measure
GUI responsiveness. The full recordings remain local because they expose the
deal. Cold and warm cache states were not controlled.

**Finish-first decision (#72):** the candidate saves CPU only
under contention with think times shorter than one review, and pays with a
foreground wait, including the interactive-budget observation above, conflicting
with the current-decision-first rule (D-59). Finish-first is rejected;
Classic ordering and analysis results stay unchanged. This is a bounded
no-go for finish-first and identical-input caching, not for every scheduler.
The two-lane control below addresses progress independently of analysis
progress/deadline signals.

The historical synchronous lifecycle model in `skat-app/tests/coach.rs`
proves identical final evidence after cancellation/retry, but not progress of
the real driver under continuous preemption. The old driver's finite
post-game drainage likewise did not establish starvation freedom. These limits
of the previous measurements are preserved; they are not a general no-go.

### Bounded fair review control (#72)

The egui driver now keeps one foreground slot and one FIFO review slot. A
running review is never cancelled for a newer foreground request. Only the
oldest queued decision can enter the review slot; a newer displaced foreground
worker is cancelled instead of overtaking it. Cancellation retains the slot
until the worker exits. A process-wide atomic permit bounds live/practice coach
computations to two, even across new games, series, repeated attempts and parked
sessions. Each computation retains the existing CPU-count bound on its internal
analysis threads; two computations can contend for those CPUs. Replay
comments/reviews and opponent workers are separate and outside this cap. No cache, analysis budget, seed, hidden-view input, quality or
moment-selection rule changes.

With bounded analysis work and eventual OS thread service, each finite FIFO
predecessor completes even while foreground requests continue. Coach-off,
new-game and abandoned attempts deliberately cancel work rather than promising
progress for discarded contexts. Off retains the queue for resume; completed
results from cancelled slots are ignored. Existing lineage/budget checks still
reject stale results. Practice parks and restores both live lanes.
`reviews_progress_while_foreground_keeps_preempting` holds real input jobs behind
controlled channels, repeatedly replaces the foreground decision and accepts
the oldest review **before play stops**. The overtaking regression prevents
fresh foreground work bypassing an older queued review. The lifecycle test
covers both slots, coach-off, an already-in-flight result, practice restoration
and game-reset channel isolation. Another regression keeps useful work alive
when no new foreground request exists (including game end), and a tiny real
driver comparison checks full recordings and nonempty final moments. Existing
app tests cover retries, stale
lineages, God Mode, open display and hidden-deal substitution. The new
`coach_review_job` exposes the same stored permitted snapshot independently of
the current request, suppressing work while off or inside a permitted guided
dialogue. Its input/result equality is tested against the ordinary job API.

The comparison entrypoint is now
`coach_latency --review-play cancel|finish|fair DELAY_MS GAMES NODES_PER_TASK`.
`fair` models the two slots and waits only for an already-cancelled foreground
worker to release its slot, never for a whole background review. The real GUI
performs that wait asynchronously. `COACH_TRACE_ROUNDS=2` repeats the same seeds
in one process. `COACH_TRACE_EVIDENCE=1` additionally emits every accepted full
explanation keyed by its input-derived sampling seed; `COACH_TRACE_RECORDING=1`
emits privileged game traces. Completion is timestamped before serialising the
final evidence. Both session-start totals and the remaining wait after game end
are reported. These local traces expose the deal and must remain private.

The ignored `debrief_paced_gui_worker_probe` runs the actual egui driver and a
frozen test-only copy of the old driver. `SKAT_COACH_POLICY=cancel|fair`,
`SKAT_COACH_NODES`, `SKAT_COACH_DELAY` and `SKAT_COACH_ROUNDS` select paired
manual measurements; defaults remain fair, 20,000 nodes, both 0/5 ms delays and
one round. It records first useful moment, full completion, the remaining wait
after the GUI observes game end, and headless step
p95/max including the post-action driver step. It is not physical-display or
human-response latency.

#### Interactive-budget paired measurements

Measured on 2026-11-21, Intel Core i9-10900K (20 logical CPUs), 32 GB RAM,
`skat-dev`, Rust 1.99.1, release app probe and optimized test-profile egui
harness. Both use Classic Easy opponents, six fixed seeds, 24 samples and
100,000,000 nodes/task, fixed legal actions, 0/5 ms synthetic think time and
two passes in each process. Every game starts with an empty session result
store. The repeat pass warms process/code/allocator state; it is **not** a
shared analysis-cache hit. No shared result cache was added, and OS caches
were not flushed; physical cold boot/startup is not measured. Timed session
intervals exclude process setup (and initial harness construction).

"Shared" uses all available CPUs on this unisolated host (load average
around 35 before the matrix). "Contended" pins the whole process to CPU 2,
shared with one owned Python busy loop. That also changes available analysis
parallelism equally for both policies. Project builds/tests were stopped for
the matrix; other host work was uncontrolled. CPU is process user + system
for both passes, including diagnostic collection/output; RSS is whole-process
peak across both passes, excluding the contention process. Cells are single
paired processes, not statistical speed estimates.

All **96 app traces** (48 policy pairs) match the six canonical seeded games
across policy, delay, load and pass: all eleven full accepted explanations per
game, complete final moment/evidence JSON and complete privileged recordings.
Cancelled work is excluded from accepted evidence and remains queued. No
budget, result semantics, input perspective or Classic gameplay output changed.

The following first/full intervals are the **remaining wait after game end**,
in ms, rounded to 0.1 ms. Full columns show the six-seed range separately for
first and repeat passes. Foreground is the largest request-to-thread-start wait
in the app harness; fair waits for cancellation exit synchronously here,
whereas the GUI keeps accepting input and waits asynchronously. The app probe
starts every scripted foreground request before advancing the trace, including
at zero think time; the GUI can advance while cancellation settles. Their
absolute times therefore describe different pacing controls, not interchangeable
UI latency. Both preserve the same actions and final evidence.

| Load | Think ms | Policy | First useful ms | Full first pass ms | Full repeat ms | Max foreground start ms | CPU s | RSS KiB |
|---|---|---|---|---|---|---|---|---|
| shared | 0 | cancel | 372.4–4985.1 | 1645.1–13200.9 | 976.1–3592.0 | 3.420 | 52.28 | 1,277,768 |
| shared | 0 | fair | 42.5–1422.8 | 225.2–3427.6 | 151.6–2171.0 | 132.942 | 50.00 | 820,960 |
| shared | 5 | fair | 0.0–1354.3 | 103.5–2725.4 | 141.9–1809.0 | 37.462 | 50.05 | 779,692 |
| shared | 5 | cancel | 0.0–0.1 | 350.4–2094.3 | 156.1–1197.2 | 4.411 | 48.37 | 635,216 |
| contended | 0 | cancel | 620.0–6571.2 | 1330.8–10493.8 | 1308.5–11261.4 | 2.209 | 35.28 | 216,588 |
| contended | 0 | fair | 517.6–8839.8 | 1149.4–11617.7 | 1106.6–9137.3 | 15.818 | 27.64 | 152,580 |
| contended | 5 | fair | 0.0–0.0 | 1147.5–8921.9 | 1106.8–8958.3 | 5.952 | 26.40 | 116,196 |
| contended | 5 | cancel | 0.0–0.0 | 1114.6–8687.1 | 1169.4–8751.3 | 0.042 | 26.15 | 139,952 |

The actual-driver egui harness also produced **96 matching traces**, including
exact final moments and full recordings across policy, delay, load and pass.
Its six canonical recordings/moments match the app probe's canonical games too.
In all these final GUI cells, at least one useful verified moment was already
available when the harness observed game end (first-after-finish rounds to
0.0 ms). This is not a physical presentation-time measurement.

| Load | Think ms | Policy | Full first pass ms | Full repeat ms | Largest per-game step p95 ms | Max step ms | CPU s | RSS KiB |
|---|---|---|---|---|---|---|---|---|
| shared | 0 | cancel | 601.6–3535.0 | 185.5–1296.7 | 12.904 | 155.581 | 83.33 | 598,132 |
| shared | 0 | fair | 0.0–1022.5 | 0.0–1042.7 | 18.929 | 148.249 | 63.38 | 883,744 |
| shared | 5 | fair | 0.0–950.4 | 0.0–918.6 | 15.960 | 138.877 | 63.38 | 938,560 |
| shared | 5 | cancel | 143.8–1305.6 | 157.2–1267.3 | 8.022 | 62.805 | 80.16 | 594,296 |
| contended | 0 | cancel | 1792.6–13610.8 | 1816.9–13826.5 | 2.041 | 67.199 | 52.95 | 145,064 |
| contended | 0 | fair | 1927.3–20903.3 | 1345.0–13702.2 | 8.184 | 124.148 | 60.03 | 179,184 |
| contended | 5 | fair | 1271.7–13645.5 | 1263.8–12530.4 | 2.281 | 71.376 | 48.30 | 153,800 |
| contended | 5 | cancel | 1613.0–12279.0 | 1734.2–13475.3 | 4.969 | 96.192 | 55.26 | 149,584 |

**Decision:** adopt the bounded fair control for guaranteed FIFO service under
sustained foreground changes and a bound on overlapping live/practice coach
computations, preserving quality and result semantics. The actual GUI's shared
cells used less CPU and had shorter debrief waits here, but peaked at
883,744–938,560 KiB rather than 594,296–598,132 KiB and had higher step p95.
Under pinned contention at zero think time, the candidate also used **more**
CPU (60.03 vs 52.95 s), had a 20.9 s full-wait outlier and worse step outliers.
This is a real cost, not a universal latency/CPU win. The app stress control
likewise delays some first useful moments and foreground starts (maximum
132.942 ms here); it avoids waiting for a complete background review, not all
scheduling delay. The global cap prevents unbounded cancellation cohorts;
it does not impose an RSS ceiling or reserve CPU time for the UI.

The no-new-foreground regression additionally caught an overbroad initial
candidate condition: at game end it cancelled useful work solely because a
different queued review was next. The corrected driver cancels only for a
**different foreground request** and keeps useful idle/terminal work alive.
The full GUI matrix above was rerun after that fix; its earlier matrix remains
private diagnostic evidence and is not used for the final timing comparison.

These single-process pairs, synthetic Easy traces and noisy host runs establish
fixed-input equivalence and the bounded scheduling control, not a general speed
ratio, frame deadline, reference-machine result, physical cold-start latency,
Windows performance or learner benefit. Neither foreground analysis completion
nor physical input-to-display latency is the request-to-start metric. Existing
privacy tests and the complete local platform gate remain mandatory; native
Windows CI verifies that platform separately. No neural/text-generation claim
or additional cache is introduced.
