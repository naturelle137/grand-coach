# Coach didactics: from evaluation to learning

Product direction first reviewed on 2026-10-31. Packages A–G and K are now
implemented; H–J and L remain open. Current behavior is in [coach.md](coach.md).
The baseline review below is historical, not today's feature inventory.
Replaces the implementation assumptions of the supplied German concept
`skat_trainer_coach_didaktisches_konzept.md`; that input remains unchanged.
The implementation plan is in [coach-work-packages.md](coach-work-packages.md).

## Historical evidence and baseline (2026-10-31)

Inspected `skat-analysis`, `skat-coach`, `skat-app`, the egui coach renderer,
translations, project decisions and the supplied debriefing screenshot.
Baseline: local `feat/coach-no-tell`, commit `9eda0d9`, including D-49;
`origin/main` at inspection was `edc0db0`, without that last commit.
[PR #38](https://github.com/naturelle137/grand-coach/pull/38) carries D-49
and merged during preparation. The documentation branch was then rebased
onto main `f733d70`. The staged-hint prerequisite is now available;
re-read the merged APIs before implementing an issue.
This review inspected code; it did not reproduce the pictured deal or run
a new interactive playtest. The screenshot alone cannot establish why
Karo-Acht, Kreuz-Bube or passing was strategically preferable.

| Evidence at that baseline | Assessment | Consequence |
|------------------|------------|-------------|
| `skat-coach::Reason` compares chances, utility and game points | These describe an evaluation difference, not its strategic cause | Add tested evidence in shared analysis before writing causal explanations |
| `considerations` forwards phase-filtered observations and all inferences | Correct information can still be irrelevant to this decision | Select only facts supporting the current teaching point; keep the rest in details |
| `moments` selects up to two good and three bad moments by score gaps or avoided traps | A large gap is not sufficient learning value | Select explainable, varied moments; no quota of praise or criticism |
| `CoachSettings::default` enables figures | The screenshot's primary view is overloaded | Default estimates off; preserve exact card counts when relevant |
| egui flattens debriefing into styled text lines | Poor hierarchy and no visual reconstruction of the decision | Add compact summaries, expandable evidence and historical context |
| `MoveReview` stores evaluations and considerations, not the original hand/trick | The player cannot reconstruct the moment from these data alone | Retain a permitted decision snapshot through `skat-app` |
| Shared sampling, deterministic seeds, visibility boundaries and asynchronous jobs exist | Sound foundation; no rewrite needed | Extend existing paths; preserve cancellation and stale-result handling |

Hiding figures already removes estimated numbers, but leaves comparisons
such as “wins less often” and the long fact list. This alone does not
produce a human explanation. `Reason::Points` uses ranking utility; for a
defender it is not simply that defender's Seeger-Fabian list entry.

## Purpose and learning loop

Teach a repeatable thought process: orient yourself, identify the relevant
unknown, compare feasible options, act, and check what changed. The aim is
better independent play at a real table. Counting cards and understanding
opponents remain the first learning domains. Human expertise is a design
model, not a universal cognitive law. No supporting human Skat expert is
available; this is an explicit validation limitation, not a release blocker.

One optional focus per learning game. A teaching interaction has:

1. A concrete question about the current position.
2. One or two relevant facts, with their source.
3. A conditional explanation of a mechanism, if validated.
4. After the move, a fair comparison with uncertainty.
5. One question to carry into a different position later.

The coach must distinguish a move's estimated quality, the actual game
result, and evidence of understanding. A good move does not prove a good
thought process; a bad move does not diagnose a misconception. Praise the
observable choice. Claim independent recognition only after an unaided
answer or new task supports it. Never infer “you considered this trap”
from an unchosen alternative.

## Corrections to the supplied concept

| Original proposal | Revised commitment |
|-------------------|--------------------|
| Separate strategic explanation engine | Strategic features, mechanisms and any search stay in `skat-analysis`; `skat-coach` selects and verbalises, `skat-app` coordinates |
| Help levels culminating in an advised plan and exact move | Retain D-49: no preferred action or ranked plan before a live move, even on repeated requests. Demonstrations belong to separate exercises after an answer. Exception (D-60): the opt-in figures stage shows unlabelled per-option estimates from which a preference can be inferred |
| A neutral question can name the right plan | Audit implicit leaks too: do not reveal “draw Trumpf first” by loading the question or showing only the preferred action group |
| MVP 1 rules, MVP 2 simulation | Sampling and double-dummy analysis already exist; improve teaching incrementally on that foundation |
| 15–25 rules and coverage of every phase in the first increment | Begin with two exact-information lessons, then two narrow card-play mechanisms; unsupported positions receive an honest fallback |
| Safe tricks, reachable Augen distributions, information gain and robust plans | Not outputs of today's solver. Do not promise or display them without new semantics, evidence and tests |
| Counterfactual state edits validate causes | Edited positions must be legal and preserve the intended information boundary. Matched examples and explicit rule predicates can be the first validation method; correlation is not proof of causation |
| Robust difference implies realistic human win probability | Perfect-information continuations and heuristic bidding weights introduce model bias; sampling error does not measure it |
| Three successes establish competence | No mastery claim from a fixed streak. Store opportunities, difficulty, support and independent answers; start with descriptive progress |
| Adaptive misconceptions and spaced repetition in the MVP | First collect interpretable local observations and build varied exercises. Automatic diagnosis and adaptive scheduling are deferred |
| “Inferred” groups proven voids and fallible bidding signals | A void proven by not following is exact knowledge; bidding and unvalidated play signals remain model assumptions |
| General partner signals | AI partners do not have a validated signalling convention. Explain proven voids and legal possibilities, not intended secret messages |
| Generic Augen goal and counted Trumpf | Null has no Trumpf or Augen-winning target. Grand has only Buben as Trumpf. Known Skat, unknown Skat and other hands must remain distinct |
| Learning-value multiplication formula | Use explicit eligibility and deterministic ordering first; no invented precision or opaque learned weights |

Kontra/Re follows the existing configured D-03/D-26 variant; it is not a
new rule-design task. Rules of strategy remain conditional, never ISkO
obligations. No new game variants, online services or generative text are
needed. Optional learned-backend epic #67 supersedes #34/#35; it is not a
prerequisite for this work and cannot supply strategic causes by itself.

## Player experience

### Before the move

Keep off / on request / important / every move and the independent warning
setting. On request remains the default, warnings remain off. Existing
explicit settings keep their meaning. Estimates default off for new settings.

On request, gradually reveal attention, relevant known information and a
neutral comparison question. Reuse/adapt D-49 stages without exposing a
ranked `Explanation`. Stage count and exact Rust names are implementation
choices; semantic boundaries are mandatory. Exact answers to a counting
question appear only at the requested help level or after submission.

Examples of permissible attention prompts: “Welche Trümpfe sind schon
gefallen?” and “Wer konnte die ausgespielte Farbe nicht bedienen?” A
prompt must match an actual supported learning opportunity. Generic
questions must not pretend to explain why an option wins.

Automatic teaching requires a supported lesson, not only a large utility
gap. In every-move mode show a compact status when no lesson applies,
without fabricating a question. Normal play never waits for a quiz.
Preserve the existing optional warning flow, including play without check,
reconsider, and executing the original action exactly once.

### After a move and after a game

The short note may say the choice was supported or roughly equivalent,
without claiming it was uniquely right. Omit repetitive forced-move notes
from the game-end summary. Show **zero to three** primary moments, initially
at most one per teaching concept. These are product defaults to test,
not research-established optimums. No minimum number of good/bad moments.

A moment contains:

- **Context:** role, game type, phase/Stich, own hand and current trick at
  the decision, chosen action. Show public history on expansion.
- **Question:** one thing worth checking at that point.
- **Evidence:** at most two immediately relevant observations; distinguish
  known facts, deductions and model assumptions.
- **Explanation:** a supported mechanism and its condition, or an explicit
  statement that only a numerical comparison is available.
- **Takeaway:** a conditional checking question, not an unconditional command.

A possible exact-information lesson is: “Links hat nicht Trumpf bedient.
Damit weißt du: Links hat keinen Trumpf mehr. Prüfe bei der nächsten
Trumpffrage zuerst, was durch das Bedienen schon feststeht.” This does not
assert which card is best or that the other player still has Trumpf.

A loss alone cannot establish “bad luck” either: without a supported
ex-ante comparison and separate outcome evidence, simply acknowledge that
a sound estimated choice may still lose.

For the supplied screenshot a truthful interim summary is: “Stich 5:
Deine Herz-Sieben wird im Modell schwächer bewertet als Kreuz-Bube. Eine
strategische Begründung liegt noch nicht vor.” Do not replace that with
an invented assertion that Kreuz-Bube protects an Ass or secures victory.

Collapsed **Analysis details** retain legal alternative comparisons,
rounded chances, metric definitions, samples/effective samples, available
standard errors, status and model assumptions. Distinguish Augen, actual
Seeger-Fabian score, ranking utility and declarer game points. Never derive
confidence intervals for unavailable quantities (including ranking utility
from only a score standard error) or interpret sampled
min/max Trumpf counts as proven bounds. Exhaustive enumeration is exact
under the continuation model, not proof of realistic human outcomes.

### Selective counting and practice

Use the same exact knowledge as the M6 aid/quiz (#11). Clarify whether a
count means outside the player's hand, in unknown places, or in other
players' hands; a known discarded Trumpf is not a remaining opponent
Trumpf. Derive the answer at the question's decision snapshot.

Assistance starts visible or on request, then permits an optional answer
before reveal. Later practice presents a new legal position without help.
The player can skip, restore assistance or leave training at any time.
Do not penalise using help or silently change a user's coach settings.

Dedicated exercises build on M7 reconstruction (#12) and M8 training
(#13), separate from series scores. The first set contains counting,
proven voids and the validated card-play mechanisms, with changed suits,
seats and counterexamples. Retrying the same deal is practice, not
independent transfer evidence. Demonstrations and feedback follow the
answer; live play still follows D-49.

## Evidence architecture and limits

`PlayerView` → shared analysis facts/mechanisms → coach lesson selection
and structured keys/parameters → `skat-app` snapshots/commands/view models
→ egui plus Fluent de/en. Frontends only depend on `skat-app`.

Extend the existing types instead of introducing parallel `BeliefState`,
`CandidatePlan` or game-state hierarchies. Each lesson needs a stable ID,
version, phase/role/game applicability, explicit predicates, evidence
references, exclusions, attention prompt, retrospective explanation and
transfer question. Code and docs stay English; product copy lives in
`assets/i18n/`. No texts in logic crates.

Separate **a fact useful for learning** from **evidence that one action is
better**. The first can be taught without a causal claim about the second.
A proven void alone does not prove a partner can win a later trick.

For the first strategic increment, support only narrowly specified cases
of side-card vulnerability to Trumpf and partner overtaking in the
current trick. Include seat order, following obligations, card strength
and role; reject cases where the evidence is insufficient. A possible
Abstich is not a certain one and does not by itself justify Trumpfziehen.
Do not assert a multi-trick winning plan from a one-trick predicate.

Before-card-play lessons come separately: explain own Buben/Trumpf
structure for candidate contracts and legal value constraints using core
helpers. Do not promise a profitable bid limit from those features.
Coarse pre-play estimates and the baseline continuation bias remain
visible limitations, especially at the interactive 24-sample budget.

Reuse analysis results and seeds. Exact feature extraction should not
launch additional searches; expensive counterfactual experiments run
offline or in explicitly requested analysis. Preserve bounded work,
cancellation, decision-key matching and asynchronous UI responsiveness.
Retain per-decision permitted snapshots, never a hidden-hand copy or a
seed from which the GUI can reconstruct hidden hands. Analysis seed,
budget and versions support reproducibility but are not learning evidence.

Debriefing continues to use historical ex-ante views even after the game.
Future omniscient replay comparisons must use separate types, cache keys
and visible labels. Open display and Mephisto do not make the normal
human coach omniscient.

## Progress and validation

Record local opportunities, lesson/task version, permitted-context ID,
answer/action, support used and outcome of an explicitly scored question.
Keep normal games, repeated-deal practice, assisted exercises and new
unaided tasks separate. Deduplicate by attempt and decision/question ID.
No inference about thoughts from engine regret alone. Initially report
counts and support use, with “insufficient evidence” where appropriate.

Persistent history is an M8 extension: versioned local data, restart,
reset, corrupt/unsupported-data handling and no silent loss. No accounts,
cloud telemetry, automatic misconception diagnosis or mastery percentage.

Each lesson's engineering gate requires independently specified positive,
boundary and counterexample positions, evidence provenance, hidden-deal
substitution tests, deterministic output, missing-evidence fallback and
bilingual UI coverage. Legitimately known Skat and Ouvert are separate cases.
A rule engine test proves legality, not pedagogical effectiveness.

No human Skat expert is available. The maintainer participates as a learner
in user acceptance tests (UAT), assessing clarity, relevant context and
usable flows, not certifying advanced strategy. Support factual/strategic
claims with reproducible permitted-view evidence and independently stated
expected results. Agreement with the same solver, another AI or a learner
is not independent strategic certification. Narrow or omit unsupported
claims; correctness and information-boundary gates remain binding.

[#51](https://github.com/naturelle137/grand-coach/issues/51) prepared the M5
evidence checklist, UAT scripts and initial/delayed evaluation protocol
after #42 and before #43: [coach-validation.md](coach-validation.md)
(D-54). Lesson PRs apply the evidence checklist (part A).
This supersedes any earlier expert-review or expert sign-off step in the
original concept and plan, and the earlier combined #51 placement that
also covered the M8 evaluation (now #54).

[#54](https://github.com/naturelle137/grand-coach/issues/54) executes the M8
learner evaluation after #50, including new unaided tasks and a later
follow-up. Freeze tasks, scoring and assistance rules before collecting
results; record the real interval and previous exposure. Actual human-run
items stay pending until performed. Agents must not impersonate the
learner. A single learner's observations do not establish general learning
effectiveness or expert strategic validity.

Research supports investigating self-explanation, fading and retrieval;
it does not validate this product's frequencies, task set or threshold:

- [Aleven and Koedinger (2002)](https://doi.org/10.1207/s15516709cog2602_1)
  studied self-explanation with a Cognitive Tutor, not Skat coaching.
- [Salden et al. (2009)](https://doi.org/10.1111/j.1756-8765.2008.01011.x)
  studied worked examples and fading in tutored problem solving.
- [Carpenter, Pan and Butler (2022)](https://doi.org/10.1038/s44159-022-00089-1)
  review spacing and retrieval practice; no specific Skat schedule follows.
- [Buro et al. (2009)](https://ijcai.org/Proceedings/09/Papers/236.pdf)
  describe Skat inference and search, not the effectiveness of this teaching UI.

## Delivery boundaries

First ship readable debriefing without inventing new reasons. Next ship
evidence contracts, a tiny exact lesson set, validated strategic lessons,
focused hints and historical context. Only then connect retrieval practice,
local progress and varied transfer tasks. Pre-play explanation can follow
independently after the evidence foundation.

New issues refine the functional scope; they do
not duplicate the counting aid, replay engine, release packaging or optional
neural-network projects. Adaptive curricula, universal explanations,
information-gain estimation, free-form AI dialogue and a new solver are
explicitly deferred.
