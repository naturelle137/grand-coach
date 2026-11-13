# Architecture: `skat-core`

The [workspace guide](../AGENTS.md#architecture-crates) maps crate dependencies.
This page fixes the decisions inside `skat-core` that analysis, AI, coach,
replay and frontends build on (D-20 to D-26, rule interpretations in D-28).
Keep it short; details belong in the code (`crates/skat-core`).

## Cards

- `Card` is an index `0..32` (`u8`); `Suit` and `Rank` are enums.
- `CardSet` is a `u32` bitset with set operations (union, intersection,
  count, iteration).
- Rationale: Monte Carlo and the double-dummy solver need cheap set
  operations and cheap cloning of whole game states.

## Game state and actions

- One state machine `Game` with a `Phase` enum, not one type per phase, so a
  simulator or search can hold any position in one type.
- Phases: `Reizen`, `SkatAufnahme` (Hand or pick up), `Druecken`, `Ansage`,
  `Spiel`, `Beendet`, `Eingepasst`. Kontra/Re is not a phase of its own (see
  below).
- API shape:

  ```rust
  fn current_seat(&self) -> Option<Seat>;
  fn legal_actions(&self) -> Vec<Action>;
  fn apply(&mut self, action: Action) -> Result<Vec<Event>, RuleError>;
  ```

- `apply` validates first and mutates only on success: a rejected action
  leaves the state unchanged. `validate` performs the same check without
  applying.
- `Action` and `Event` are plain enums. `RuleError` is a structured enum with
  no texts (D-13).
- Actions: `Bid(value)`, `Hold`, `Pass`, `Hand`, `TakeSkat`,
  `Discard(CardSet)`, `Announce(Announcement)`, `Play(Card)`, `Kontra`,
  `KeinKontra`, `Re`, `KeinRe`. They always act for `current_seat()`.
- Events carry only public information (no Skat, discard or hand cards,
  except the Ouvert reveal), so a frontend may show them to every seat.
- Rule helpers usable by analysis: `legal_cards`, `trick_winner`,
  `GameType::trumps`, `spitzen`.

## Kontra/Re in the flow (D-26)

- An eligible defender may choose `Kontra` instead of a card while it is their
  turn and they have not played a card yet. Only one Kontra per game.
- `Kontra` puts the game into a pending state in which the declarer's only
  legal actions are `Re` or `KeinRe`. Then play continues with the same
  defender.
- Ouvert: before the declarer's cards are revealed, each eligible defender
  decides once (`Kontra` or `KeinKontra`), followed by the declarer's Re
  decision if needed. Only then the cards are revealed and play starts.

## Determinism and replay

- A game is fully described by `(seed, RuleConfig, actions)` under matching
  core/dealing semantics. Dealing is the
  only source of randomness in the core.
- RNG: `rand_chacha::ChaCha8Rng`, seeded explicitly. Not `StdRng`, whose
  algorithm may change between `rand` versions and would break old replays.

## Information sets

- `Game` holds the full state and is available to Mephisto through its explicit privileged API.
- `Game::view(seat) -> PlayerView` contains only what that seat may know:
  own hand, bidding history, tricks played, the own discarded Skat (declarer
  only), and the declarer's cards in Ouvert games once revealed.
- God Mode uses `OmniscientView` for its table and known-deal coach. Ordinary
  opponents and the closed-play coach receive a permitted view, also with an
  [open display](#open-display-d-68), which changes only the human's table.

## Rules and scoring

- `RuleConfig { kontra_re: bool }`; further options only when decided.
- Game value, Überreizen and Seeger-Fabian are pure functions over a finished
  game: `settle(&GameOutcome) -> GameResult` and
  `GameResult::seeger_fabian()`, testable without playing a game.
- Seeger-Fabian returns the contribution of each seat for one game; series
  bookkeeping is not part of `skat-core`.

## Serialisation

- Public data types derive `serde` traits behind the cargo feature `serde`,
  so `skat-app` and replays can store them without forcing `serde` on every
  user of the core.

## Privileged recordings in `skat-app` (D-65)

`Session::export_current_recording_privileged` returns a detached
`GameRecording`; `completed_recordings_privileged` retains each completed
game across `NextGame`, separately from score entries. Capture happens once,
after a successful core transition in the common human/opponent apply path.
Rejected, quiz/coach/settings and held commands are not game actions; a held
choice is captured only when actually applied. A zero-length series has none.

Format 1 stores the actual game seed, original deal, rules, ordered core
actions and explicit in-progress/all-pass/finished result metadata. Context
contains producer version, original session configuration, game number,
dealer, actual opponent seeds and the selected Goldfish identity. This is
provenance, not an AI reproducibility promise: replay uses recorded choices,
never current opponents or models. Coach/retrieval events and running series
totals are not game history and are not reconstructed.

`GameRecording::reconstruct_privileged(prefix)` validates the entire record
and returns plain `PrivilegedRecordingPosition` data at an action boundary,
including public core events and the historical permitted views inside its
explicit omniscient state. It uses only core legal transitions in isolation;
active games, RNG streams and score sheets are inaccessible. Later Skat or
Ouvert knowledge is never injected into earlier permitted views. Neither
records nor replay states enter `TableView`, ordinary opponent jobs or live
coach/background analysis. Export explicitly discloses concealed cards.

Only format 1 and `RECORDING_ENGINE` = `grand-coach-recording-1` are supported.
Bump the engine discriminator for incompatible rule/deal/replay semantics;
producer package version is descriptive. No cross-version seed stability is
promised. The stored deal must match the seed under the supported engine,
so dealing drift is rejected rather than silently replayed. Invalid context,
rules, actions, prefixes, completion or settlement yield `RecordingError`.
Serde handles malformed wire syntax and rejects unknown recording/context
fields; semantic validation occurs on reconstruction. Action deserialisation
and reconstruction are bounded to 512 actions (above the full bidding ladder,
Skat/announcement, thirty card plays and Kontra/Re). A truncated finished
record cannot pass its completion/result check, even for prefix zero.
In-progress prefixes are deliberately supported; this is validation, not
authentication against deliberate metadata rewriting.

`GameRecording::read_local` caps readers (including ones that grow) at 1 MiB
plus one sentinel byte, rejects oversized input before JSON parsing and bounds
actions before typed decoding. `open_local` accepts regular local files;
`save_local_new` validates the full recording and serialized byte size before
exclusive creation (`create_new`). Existing files and symlinks are never
truncated. A write/sync failure may leave a partial **new** file; recovery uses
a new name or user removal, never silent replacement. `RecordingFileError`
separates JSON/shape, unsupported schema/engine, semantic history, size,
missing path, existing target and other IO failures. No filesystem code enters
core or the frontend. Producer version remains descriptive; there is no
migration or session resume.

The Game menu saves the current or a selected completed game and opens a
local JSON file through a simple full-path dialog. Export requires explicit
concealed-information consent. A separate egui recording window starts at
the final recorded position, labels incomplete/all-pass/finished records and
shows bidding, contract/result, completed and partial tricks. Concealed hands,
Skat and original deal require deliberate reveal and carry a full-information
label. First/previous/next/last navigate initial, completed-trick and final
boundaries derived from core `TrickWon` events. Historical seat views are
reconstructed at each boundary. Coach comments exist only for a separately
selected recorded own decision (D-67).

Live controls and background-result delivery pause while a file dialog or
recording is open. Pending opponent/coach jobs are retained, not cancelled or
replaced. Closing resumes the original series; invalid input leaves the last
valid recording and live session untouched. Recording data never enters live
analysis. Editable history and training history remain unimplemented;
the whole-recording review runs only on request (D-69).

### Same-deal practice (D-66)

`Session::start_practice(&live, &recording)` first validates the whole
untrusted recording (header, seed/deal, legal history and end) and builds
nothing on failure. The new session starts **before bidding** with the
recorded cards and rules, game number, dealer and stable player coordinates
(late-series games keep their seats), the recorded series configuration
(opponent mode and levels, Kontra/Re, Goldfish identity, God Mode
visibility, open display) and opponent seeds. It does not use the recording's game seed as
a series seed and never reruns the recorded actions. Opponents and the coach
use the same shared code paths and permitted views as in a live game. From
the live session it copies only coach/assistance settings and coach budget.

RNG policy: the coach seed is stream 2 of the recorded series seed, as in the
original series, so the same position is analysed with the same samples. The
quiz uses practice stream 4, keeping opportunity IDs apart from the live
series. Deal and opponent-seed streams are never advanced.

An attempt is exactly one game: `NextGame`/`EndSeries` return
`AppError::PracticeAttempt`, the result enters no score sheet, no recording is
retained and `export_current_recording_privileged` returns `None`.
`TableView::practice` labels it. Each attempt, like every live session, has a
unique lineage in opponent positions and coach decision keys (D-68), so moves
and analyses from the live series or another attempt are rejected as stale
even at equal positions.

The recording window offers "Practise this deal". The app then parks the live
table state (session, drafts, error, pacing, outstanding opponent and coach
jobs and channels, new-series draft) without polling or cancelling it, closes
the recording windows and installs the practice table under a practice banner.
Saving, opening and new-series commands are disabled. "New attempt" (at most
50 per entry) and "Return to series" drop the practice state, cancelling its
background work and disconnecting its channels; return restores the parked
live state, whose retained results are then delivered normally. Language and
deck are shared display settings.

### Historical and hindsight replay comments (D-67)

`ReplayCoach::open(&recording, budget)` validates the whole recording and
lists every recorded action of the human (`PlayerId::HUMAN` at the recorded
dealer, so late-series seats are right) as `RecordedDecision`. Opponent
actions, the final position and an in-progress file's outstanding decision
are not selectable; a file without a recorded human action reports
`ReplayStatus::NoDecisions`. Selecting a decision builds its
`DecisionContext` from the historical permitted view **before** the action;
later tricks, Skat or Ouvert cards never enter it.

The default perspective is `Historical`: the job holds only that
`PlayerView` and runs `skat_coach::advise`, seeded by `decision_seed` of the
constant `REPLAY_COACH_SEED` and the view. No recording, series or game seed
and no concealed card is used, so a substituted hidden deal with the same
permitted view gives the same comment. `Hindsight` is requested by a separate
control; only the explicitly privileged path adds the reconstructed
`OmniscientView` and runs `advise_known` (basis `known`). The comment is
`skat_coach::review` of the recorded action plus the compared legal
alternative (best option, or the runner-up when the choice was best), with
lesson or fallback, uncertainty and basis unchanged.

Every request has a key of opened-recording source (unique per open, even
for identical files), generation, prefix, perspective, budget and seed;
`ReplayCoach::finish` accepts only the outstanding key. Selecting another
decision restores the historical perspective and computes nothing;
changing perspective, retrying, cancelling or changing budget invalidates
the previous request. `ReplayCoachJob`/`ReplayCoachResult` are distinct from
`CoachJob`/`CoachResult`, so `Session::finish_coach` cannot accept them.

The recording window runs one replay job at a time on a background thread
with the live coach budget and `REPLAY_COACH_TIME_LIMIT` (60 s); reaching it
shows as cancelled. Selection, perspective, cancel, close and practice start
drop the job, cancelling it and disconnecting its channel. A newly opened
recording starts unselected and historical. Revealing concealed cards remains
a display choice and never changes the perspective. Live opponent/coach jobs
stay parked, and session, scores, recordings and files are untouched.

### Open display (D-68)

`SessionConfig::display` is chosen per series in the new-series dialog:
`TableDisplay::Closed` (default) or `Open`. It is a display choice only.
`Session` derives two separate things: `displayed_cards` builds plain
`OpenCards` for `TableView::open_cards` when `SessionConfig::shows_all_cards`
holds (open display or God Mode); `coach_known_deal` gives the live coach an
`OmniscientView` only for `CoachBasis::RealDeal`, which is God Mode alone.
`OpenCards` is accepted by no analysis, so the display path cannot feed the
coach or an opponent. `TableView::coach_basis` tells frontends which basis
applies; the egui status bar and coach panel label "open display: all cards"
apart from "coach: your cards only" (or "all cards (God Mode)").

With equal seeds, deals and human actions a closed and an open series produce
identical opponent jobs and moves, coach jobs and results, human views apart
from `open_cards`, events and score sheets (lockstep tests over levels,
Goldfish, Mephisto and Neural, every dealer, Hand, Drücken, Ouvert, finished
and thrown-in games). An opponent's discarded Skat appears only in
`open_cards`. Mephisto keeps seeing every card as before; the open display
neither grants nor removes privilege in any mode. In God Mode the display
field is irrelevant: the table is always open and the coach uses the deal.

Pending work: the display is fixed for a series, so switching display, mode
or series means a new `Session`. Each `Session` has a unique lineage in
opponent positions and coach decision keys, so a job of a replaced session,
including a God Mode analysis, is rejected as stale even at an equal
position. The frontend also drops its in-flight opponent and coach work when
it starts a new series.

Format: the field uses a serde default and is omitted when closed, so closed
recordings keep their exact format-1 bytes and files written before D-68
read as closed; no format bump. An older build reading an open file ignores
the unknown field (the session object does not deny unknown fields) and
shows it closed. Same-deal practice keeps the recorded display and the
closed-view coach. Open display adds no hindsight pipeline: full-information
comparison stays the explicit replay hindsight request (D-67).
A recording of an open game gets the same historical comment and review jobs,
seeds and results as its closed twin (D-67, D-69).

### Bounded recording review (D-69)

`ReplayCoach::request_review` starts the review of the opened recording;
`review_status` reports `Unavailable(Incomplete)` for an in-progress file
and `Unavailable(NoDecisions)` without a recorded human action. Covered are
the recorded human decisions with more than one legal option ("eligible");
forced decisions are counted and skipped, and every index stays the index
into `ReplayCoach::decisions`. `review_job` hands out the next eligible
decision without an outcome, in game order, at most one at a time:
`ReplayReviewJob` holds only that decision's historical `PlayerView`, the
recorded action, the coach budget and `decision_seed(REPLAY_COACH_SEED,
view)`, and runs the same `advise`/`review` path as a historical single
comment, so a substituted hidden deal with the same views gives the same
review. `finish_review` accepts only the outstanding key (opened-recording
source, review generation, decision, prefix, budget, seed).

When every eligible decision has an outcome, the analysed reviews in game
order go to the unchanged `skat_coach::moments` with the learner's focus.
That is the live debriefing's D-58 policy: eligible is a decision with a
choice and a supported lesson; rated `Good`/`Bad` only with a complete
analysis (`Bad` needs a `Worse`/`Weak` judgement, i.e. beyond the noise
margin of 2 combined standard errors and more than 1 Seeger-Fabian point,
plus a loss of at least `MOMENT_POINTS` = 5 points in the seat's utility:
Seeger-Fabian points, for a defender with half the declarer's lost game
points, so Null, Schneider and Schwarz count through the score); provisional,
screened or not evaluated choices are `Neutral` at most; at most one moment
per concept, ranked focus, rated, mechanism, larger loss or avoided trap,
game order, and shown in game order. No good/bad quota and no new threshold.
`ReviewMoment` adds the decision index, the compared legal alternative and
the decision's permitted `DecisionContext`; `ReviewSummary` counts recorded,
forced, eligible, analysed, provisional and unavailable decisions, its
`ReviewEnd` (`Finished`, `Cancelled`, `TimeLimit`) and `exhaustive()`
(every eligible decision analysed).

Outcomes are cached per decision for the opened recording and its current
budget; a budget change resets review and cache, closing or reopening drops
them. Selecting a decision in the historical perspective shows its cached
comment (same identity as a single request); hindsight is never cached and
never enters the review. The focus only reselects. Cancel and time limit
keep analysed decisions; "continue" retries unavailable ones and analyses
the rest. A single comment cannot start while the review runs, nor the
review while a comment computes; selecting decisions stays possible.

The recording window runs each decision on a background thread with
`Cancel::with_deadline(min(now + REPLAY_REVIEW_DECISION_LIMIT (20 s),
start + REPLAY_REVIEW_TIME_LIMIT (120 s)))`; a decision reaching its deadline
counts as unavailable, reaching the total limit ends the review as
`TimeLimit`. Close, practice start and cancel drop the running thread.
Measured with `Budget::INTERACTIVE` on the four fixture games (release,
20 hardware threads): 4.4 s, 2.1 s, 2.3 s and 0.6 s per whole review, slowest
decision 1.5 s, all eligible decisions analysed, cancellation returns within
2 ms (`replay_review_latency` example). Live session, scores, pending jobs and
file bytes stay untouched.

## Learning history (D-70)

`skat_app::LearningHistory` is the single local store for learning
observations; later decision provenance extends it rather than adding a
second statistics system. Format 3 holds a persisted attempt counter, the
attempts (`AttemptId`, `AttemptKind::Series`, `Practice { attempt }` or
`Training { attempt }`, where attempt 2 and higher are intentional retries
or repeats) and the `RetrievalEvent`s exactly as the session or the
[training](#focused-counting-training-d-71) emitted them, each tagged with
its attempt. An event's `OpportunityId` is either `Game(QuestionId)` or
`Training(TrainingTask)` (fixture and version); it must fit its attempt
kind (`LearningError::InvalidContext`). It holds no cards, deals, seeds or
recordings.

Provenance (format 3): each new event carries `lesson`, the immutable
`LessonRevision` (`LessonFamily::CountingQuiz`, `FocusedRetrieval` or
`CountingTraining`, and a rule version, currently 1) fixed when its
opportunity is offered. The version names when questions are asked, how
the exact count is derived and how an answer is scored; it is raised when
one of them changes, so observations never merge across rule revisions.
Training events still name the fixture and its own version in their
`OpportunityId`. A new `Answer` carries `expected`, the exact count it was
scored against; it exists only on the answer itself, never on the
opportunity or help events, so nothing persisted reveals the count before
the learner answers (help shows it on screen, but its event stores none).
`record` refuses (`LearningError::InvalidProvenance`) a new event without
lesson or a new answer without `expected`, a lesson that does not fit the
opportunity or changes within it, and an `expected` contradicting
`correct`. No question snapshot, card or deal is stored for this.

Summary: besides the counts per topic and context, `summary` classifies
each settled response by `ResponseCategory` (exact, too high, too low,
mismatch with unrecorded value, skipped), split by support (without help /
with help) per row and in total. The category is only the observed answer
against the stored exact count: no reason, strategic mistake, mastery or
transfer is inferred, and strategic decision classification is not part of
the history. Legacy wrong answers have no stored count and stay
`MismatchUnrecorded`; their deviation is never reconstructed. `provenance`
counts opportunities per lesson revision, with unversioned legacy ones
listed separately. `comparisons` gives, per topic, the number of settled
responses and, from twice `COMPARISON_WINDOW` (10) on, two windows: the 10
before the last 10 and the last 10, in persisted event order, each with its
positions, lowest and highest attempt ID, categories by support and
responses per context. Skips stay skips; the windows are descriptions, not
trends.

Identity: the frontend asks `begin_attempt` for a fresh ID on the first
retrieval event of each series and each practice attempt. IDs come from the
persisted counter and are never reused, not even after a reset; session
lineages and quiz seeds are not used because they repeat across restarts
and practice attempts of one deal. Each training presentation gets its own
attempt. An event's key is (attempt, `OpportunityId`, action tag), with answer and skip sharing the one settlement tag; `record`
skips stored keys, so re-sent batches, reloads and replays never count
twice. Browsing recordings and replay comments emit no retrieval events.
Original recordings, score sheets and live jobs are not touched.

File: `learning-history.json` in the data directory: `GRAND_COACH_DATA` if
set, on Windows `%APPDATA%\Grand-Coach`, otherwise
`$XDG_DATA_HOME/grand-coach` or `~/.local/share/grand-coach`; without one
the history stays in memory and the window says it is not saved. The GUI
saves after every batch with new events and after a reset: written
completely to `learning-history.json.tmp`, then renamed over the file.
Retention: until the learner resets it in the window (confirmation
required); uninstalling the app does not remove it. Bounds: 16 MiB,
50,000 events, 20,000 attempts; a full history refuses new events, never
drops old ones, and the window reports it.

Reading checks the size, then the `format` field before decoding, then
attempt IDs (non-zero, unique, below the counter), event attempts and
duplicate keys. Failures are structured (`LearningFileError`: invalid,
unsupported format, too large, I/O). A damaged or unsupported file is kept
byte for byte and never saved over: the window reports it and new events
of this run stay in memory until the learner chooses **Rename damaged file
and start a new history**, which renames it to the first free
`learning-history.damaged-<n>.json` and saves.

Migration: this build reads formats 1, 2 and 3 (`LEARNING_FORMATS_READ`)
and writes 3. Format 1 (PR #133) is decoded unchanged: the untagged
`OpportunityId::Game` serialises exactly like the former `QuestionId`.
Formats 1 and 2 (PR #137) are migrated in memory, keeping counter,
attempts, events and their order, so counts and deduplication are
identical; their events stay unversioned (`lesson: None`) and their answers
have no `expected`: no current lesson revision or count is assigned
retrospectively. Both fields are omitted when absent, so legacy events are
written back unchanged in shape. Reading never rewrites the file, and the
next save (after a new event or a reset) writes format 3. A format 1 file
containing training attempts or tasks, and a format 1 or 2 file containing
provenance fields, is invalid. Any other version (for example 4 from a
newer build) is reported as unsupported. Older builds report a format 3
file as unsupported and keep it, so they never drop its provenance. Two app instances writing the same file are not coordinated; the
last save wins.

## Focused counting training (D-71)

`skat_app::TrainingSession` is a small plain state machine outside any
`Session`; it owns no game, score sheet, recording or file. Topics are the
two focused-retrieval families (`TrainingTopic::TrumpfOutside`,
`ProvenVoids`). Each `TrainingFixture` is a fixed position: a seed for
`Game::new` with `RuleConfig::default()`, a recorded action prefix
(bidding, Skat, announcement, card play), the intended seat (whose turn it
is) and one question. `TrainingFixture::view` replays the prefix through
`Game::apply`, rejects an illegal action, a position that is not the seat's
card-play turn or a question outside `retrieval_questions`, and returns only
that seat's `PlayerView`; the full game is dropped. Question, help,
expected answer and feedback come from the view's `Knowledge` via
`skat_analysis::counting::derive`, as in the quiz, so hidden-card
substitutions with identical views give identical tasks.

Initial set (version 1 each; a changed position gets a new version):

| Fixture | Seed | Seat, role | Game | Question |
| --- | --- | --- | --- | --- |
| `TrumpfDeclarerFarbspiel` | 1 | Hinterhand, declarer (own discard known) | Karo | Trumpf outside: 2 |
| `TrumpfDefenderGrand` | 19 | Vorhand, defender (Skat unknown) | Grand | Trumpf outside: 1 |
| `VoidsDeclarerGrand` | 0 | Vorhand, declarer | Grand | Proven Trumpf voids: 2 |
| `VoidsDefenderFarbspiel` | 3 | Hinterhand, defender | Kreuz | Proven Herz voids: 1 (0 one card earlier) |

Null has no Trumpf, so the Trumpf topic has no Null fixture. These four are a
baseline, not #50's varied transfer set.

Commands: `Topic`, `Assistance` (existing support levels), `Help`,
`Answer`, `Skip`, then `Retry` (same task) or `Next` (next fixture of the
topic, cycling); a refused command changes nothing. Help is recorded once
and stays assisted for that presentation; `VisibleAid` starts assisted.
Every presentation emits an `Opportunity` and is numbered per fixture within
one entered training (`presentation`); the frontend records each in its own
`AttemptKind::Training { attempt: presentation }`. A topic switch or leaving
with an open task records no answer or skip.

GUI: **Game → Counting training …** replaces the whole window content with
the training screen (banner with **Back to the game** and the learning
history). While it is open, the live session is not drawn, its opponents and
coach are not driven and no command reaches it; pending background results
wait in their channels. The screen shows contract, bid, roles, completed
tricks with seats, the trick on the table, the hand, and the own discard or
that the Skat is unknown.
