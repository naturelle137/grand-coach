# Coach validation: evidence checklist and learner UAT

Status: protocol prepared for M5 (issue
[#51](https://github.com/naturelle137/grand-coach/issues/51), D-54),
2026-10-31. Shipped lessons: `proven-void` and `remaining-trumpf` (#43,
D-55), `trumpf-vulnerability` and `partner-overtake` (#44, D-56),
`hand-structure` and `game-value` (#45, D-57); focus and
moment selection without new lessons (#46, D-58); historical decision
snapshots in the debriefing (#47, D-59); opt-in figures exception to live
non-disclosure (#65, D-60); no learner session has been run. M8 execution: issue
[#54](https://github.com/naturelle137/grand-coach/issues/54). Concept:
[coach-didactics.md](coach-didactics.md); contract:
[analysis.md](analysis.md#evidence-for-lessons).

D-61: prepare evidence and scripts during development; actual M5 learner UAT
and #54 initial/delayed sessions are deferred until before public 1.0.
[Release register #81](https://github.com/naturelle137/grand-coach/issues/81)
owns the handoff from every technical closure; #54 owns its learner protocol.
No human check is fulfilled by this transfer. Technical milestone completion
and later development do not wait for these sessions.

## Scope and limits

Three different questions, answered by different means:

| Question | Means | Who | Establishes |
|----------|-------|-----|-------------|
| Is each stated fact or condition true for the permitted view? | Fixtures with independently specified expected results (part A) | agent, `scripts/check.sh` | correctness of the tested claims only |
| Can a learner find, read and use the coach? | Learner UAT script (part B) | maintainer as learner | clarity and usability for this learner |
| Does the learner later solve new tasks without help? | Unaided-task protocol (part C) | maintainer as learner, M8 (#54) | a single-learner observation |

- **No human Skat expert is available, and none is a prerequisite.** No PR,
  lesson or milestone waits for expert sign-off, and none claims it.
- The maintainer takes part **as a learner**: they judge comprehensibility,
  context and flow, not advanced strategy. A missing answer or “unclear” is
  a valid result.
- Automated checks support exactly the claims they test. They do not
  establish that a lesson is strategically valid, the best thing to teach,
  or effective for learning.
- Agreement with the same solver, another AI or the learner is not an
  independent oracle.
- A mechanism that cannot be substantiated by part A is **narrowed or
  omitted** (the honest `Teaching::Fallback`), never shipped on trust.
- Agents prepare scripts and templates. They never fill in learner
  results, impersonate the learner, or tick a human check because an
  automated scenario passed.

## A. Evidence checklist per lesson

Each lesson PR (#43–#47 and later) copies this list into its PR body,
answers every line with the evidence (test name, file, rule source) or
“n/a: reason”, and records the lesson ID and version.

### A1. Definition (written before the implementation)

- [ ] Lesson key, version, kind (`Fact` or `Mechanism`) and support status.
- [ ] Applicability: phases, roles (Bidder/Declarer/Defender), game kinds
  (Suit/Grand/Null); Hand, Ouvert, known and unknown Skat stated explicitly.
- [ ] Permitted player view: which `Source`s may be cited (own hand, own
  Skat, revealed Ouvert cards, played cards, trick card, bidding call,
  rules). Concealed cards and the deal seed are never sources.
- [ ] Each claim classified: **exact fact** (follows from the view and the
  rules), **deduction** (exact, derived from several facts, derivation
  stated) or **model estimate** (sample, comparison, bidding evidence).
  Only exact items satisfy fact conditions (D-53).
- [ ] Required conditions and exclusions as explicit predicates, with seat
  order and following obligations where relevant.
- [ ] What the lesson does **not** claim, e.g. “a proven void does not mean
  the partner wins the trick”, “a remaining Trumpf is not assigned to a
  seat”, “a risk is not proof that Trumpfziehen is best”.
- [ ] Known acceptable alternatives: other legal actions the lesson must
  not discredit; the text says “check”, not “play”.
- [ ] Missing-evidence fallback: which `Unmet` cases end in
  `Teaching::Fallback`, including `Incomplete`, `Screened`, `NotEvaluated`
  and cancelled analyses.
- [ ] Rule assertions cite their source (ISkO section, D-03/D-26/D-28);
  strategy statements are conditional and cite no rule as obligation.

### A2. Fixtures and independent oracles

- [ ] **Positive**, **boundary** and **counterexample** positions per
  family, each a legal replay (seed or explicit deal plus action sequence)
  accepted by `skat-core`. Edited positions stay legal and keep the
  intended information boundary.
- [ ] The expected result of each fixture (sets, counts, applies or
  `Unmet` reason) is written in the test **by hand**, derived from the
  position, before or independently of running the code under test.
  Snapshotting the implementation's output is not an oracle.
- [ ] Where feasible, a small exhaustive reference check: enumerate all
  hidden distributions consistent with the view (or all legal cards) in
  the test with plain code that does not call the predicate under test,
  and compare. Mechanism claims of “possible” need at least one witness
  distribution; claims of “certain” need all.
- [ ] A comment states how each expected value was derived (e.g. “Trumpf
  in Kreuz: 11; own 4, played 3, own Skat 1 → 3 outside these sets”).
- [ ] Rerunning the same solver, another seed of it or another AI is not
  accepted as the oracle; it may be an extra consistency test.

### A3. Information boundary and live leaks

- [ ] Hidden-deal substitution: the same `PlayerView` with other concealed
  cards gives identical evidence, lesson result and Fluent arguments.
- [ ] Historical view: debriefing uses the snapshot at the decision, never
  later facts or the final hand.
- [ ] D-49, explicit leaks: before the move no option is named, ranked or
  labelled better. Stage 3 with "show figures" on is the opt-in exception
  (D-60): it may show per-option estimates in neutral order without
  labels; do not claim that they hide the model's preference.
- [ ] D-49, implicit leaks: the question is not loaded towards one action,
  only one action group is not shown, and lesson or plan IDs visible
  before the move do not imply the preferred action. Checked with a test
  on hints where the preferred option differs between two fixtures with
  the same lesson.
- [ ] Mechanisms only after the move and only with a `Reliable` comparison;
  a higher utility alone triggers no causal text (test).

### A4. Determinism, copy and gate

- [ ] Same view, seed and budget give the same lesson and text keys.
- [ ] de/en Fluent messages exist, carry the same arguments and say
  “possible” versus “certain” as the evidence class does.
- [ ] Texts distinguish known fact, deduction and model assumption.
- [ ] `scripts/check.sh` passes; the result is recorded in the PR.
- [ ] Docs (`coach.md`, `analysis.md`) describe the lesson and its limits.

## B. M5 learner UAT

Run by the maintainer as a learner in the real application. No Skat
expertise is needed: every question asks what the learner saw and
understood, not whether the move was best.

### B1. Preparation (by the PR author)

Each M5 feature PR adds a short **applicable script** to its body: build
(commit), series seed, the steps that reach the lesson moment (or a D-45
screenshot scenario plus how to reach it live), coach settings, and which
of the items B2.1–B2.8 apply. Record learner UAT as **pending** in the PR;
an agent never marks it performed.

### B2. Script

Settings: coach “on request” unless the script says otherwise, figures
off, warning off. For each moment:

1. **Locate.** Can you find the moment (trick, decision) without help?
2. **Context.** Can you tell your role, the game, your hand and the
   current trick at that moment?
3. **Language.** Is the text understandable on first reading? Note every
   unclear word.
4. **Relevant fact.** Which fact does the coach rely on, and where did it
   come from (which card, trick or bid)?
5. **Certainty.** Is it presented as sure or as an estimate? Did that
   match your reading?
6. **Checking question.** Repeat the takeaway question in your own words.
7. **Details.** Open and close the analysis details. Was anything missing
   or overwhelming?
8. **Play on.** Did the coach interrupt or slow down play? Could you
   continue, change the level or turn the coach off?

Also once per session: ask for help several times before a move and note
whether any wording seemed to say which card or bid to choose.

### B3. Result template (blank)

```text
UAT record – M5
Date:                    Build (commit):         Language: de / en
Feature / issue:         Lesson IDs + versions:
Series seed / scenario:  Coach settings:
Prior exposure to this deal or lesson: none / seen before (describe)

Item | Result (yes / partly / no / unclear / n/a) | Note
B2.1 Locate          |  |
B2.2 Context         |  |
B2.3 Language        |  |
B2.4 Relevant fact   |  |
B2.5 Certainty       |  |
B2.6 Own words       |  |   (verbatim paraphrase)
B2.7 Details         |  |
B2.8 Play on         |  |
Implied move advice seen? yes / no – where:

Findings (each: severity, issue link):
Performed by: maintainer (learner)   Status: performed / pending
```

Store filled records under `docs/uat/` (one file per session, created only
when a session was actually run) or in the PR or issue comment; link them
from the PR. Findings become issues; a fix is retested with the affected
items. The combined M5 run after #47 uses the same template across all
shipped lessons before public 1.0, tracked in #81 (D-61). It is not a
prerequisite for technical #10 closure or subsequent milestones.

## C. Initial and delayed unaided-task protocol

Defined now, before any learning outcome is collected. M8 issue #54
concretises the task set for the delivered #48–#50 flows and executes it.

### C1. Fixed before collecting results

- Task set, each with ID, version, lesson ID, a legal position (permitted
  view only), the question and the **expected answer derived by part A**
  (exact set/count, “certain/possible/unknown”, or a checking question
  matched to the lesson). Answers stay out of the learner script.
- Scoring per task: correct, partly correct (defined per task), wrong,
  no answer. Strategy is not scored unless the expected answer is backed
  by part A.
- Assistance rules: unaided means no coach help, no counting aid, no
  details, and the position not seen before in practice.
- Planned interval for the delayed follow-up (e.g. one to two weeks) and
  a stop rule: the learner may skip or end at any time.

Changes after the first result get a new task version; earlier results
keep their version.

### C2. What is measured

| Measure | Counts as |
|---------|-----------|
| Initial unaided answer on a **new** position | initial unaided performance |
| Delayed unaided answer on another **new** position of the same lesson | delayed unaided performance |
| Answer with help, details or the counting aid | assisted, never unaided |
| Same deal or position again | repeated practice, never transfer |
| Subjective clarity (part B) | comprehension, not performance |

Record per attempt: task ID and version, build, date, assistance used,
prior exposure, answer verbatim, score and the actual interval since the
initial session. Results are descriptive counts. No mastery percentage,
no general efficacy or causal claim; one learner is a formative case.

### C3. Result template (blank)

```text
Unaided-task record
Session: initial / delayed   Date:        Build (commit):
Task set version:            Actual interval since initial (days):

Task ID@version | Lesson | New position? | Assistance | Prior exposure | Answer (verbatim) | Score
                |        | yes / no      | none / ... |                |                   |

Counts: unaided correct __ / __, assisted __, repeated __, skipped __
Performed by: maintainer (learner)   Status: performed / pending
```

## Status record

| Item | Status |
|------|--------|
| Evidence checklist (part A) | prepared; applied per lesson PR |
| M5 learner UAT per feature (#43–#47) | pending in #81; PRs supply scenarios |
| Combined M5 learner UAT after #47 | pending in #81 before public 1.0 |
| Unaided-task set and follow-up (#54) | pending; actual sessions before public 1.0 |
