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
