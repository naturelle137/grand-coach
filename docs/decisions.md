# Decisions

Each entry states the decision as it holds now. To change a decision, edit
its entry in place and set "Changed" to the date of the edit; history stays
in git, not here. New fundamental decisions take the next free number;
numbers are never reused. Routine implementation details belong in code,
PRs and the focused references (architecture, analysis, AI, coach, testing).

## Product and rules

| No | Changed | Current contract |
|----|---------|------------------|
| D-02 | 2026-10-29 | Rules: ISkO, all ISkO game types. Only exception: Kontra/Re (D-03). |
| D-03 | 2026-11-01 | Kontra/Re as a switchable option, following the common German pub Skat variant (esp. the rules of the Online-Skatclub). [Details](#d-03-kontrare). |
| D-04 | 2026-10-29 | No Ramsch, no Bock. If all pass, the hand is thrown in. |
| D-05 | 2026-10-29 | Scoring Seeger-Fabian. Series with fixed length and free play. |
| D-06 | 2026-10-29 | Use deterministic seeds to support same-deal attempts and planned replay with coach comments. |
| D-26 | 2026-11-01 | Kontra eligibility and timing as encoded in the engine. [Details](#d-26-kontra-eligibility-and-timing). |
| D-28 | 2026-11-01 | Rule interpretations encoded in `skat-core`, with the ISkO sources used. [Details](#d-28-rule-interpretations-in-skat-core). |
| D-31 | 2026-11-01 | The series maps seats to players, rotates the dealer and counts thrown-in deals. [Series rules](#d-31-series-semantics). |

## Architecture

| No | Changed | Current contract |
|----|---------|------------------|
| D-11 | 2026-11-01 | Rust workspace, egui desktop frontend and Burn 0.22.1. Use Flex for offline CPU inference and train outside the app on CPU or Vulkan. Godot/gdext is a planned second frontend. |
| D-12 | 2026-10-29 | Frontends talk only to `skat-app` (commands, events, view models as plain data types). No frontend code in engine, analysis, AI or coach. |
| D-20 | 2026-10-29 | Cards as index `0..32`, card sets as `u32` bitset (`CardSet`). See [architecture.md](architecture.md). |
| D-21 | 2026-10-29 | `skat-core` is one state machine (`Game` with a `Phase` enum) offering `legal_actions` and `apply`; `apply` validates before mutating, so rejected actions leave the state unchanged. Errors are structured enums without texts. |
| D-22 | 2026-10-29 | A game is described by seed, rule config and action sequence. Dealing is the only randomness in the core and uses `rand_chacha::ChaCha8Rng` (D-28.8). |
| D-23 | 2026-11-01 | Ordinary AI and coach receive a PlayerView. Mephisto explicitly takes the full Game. God Mode uses an OmniscientView for open cards and known-deal analysis. |
| D-24 | 2026-10-29 | Game value, Überreizen and Seeger-Fabian are pure functions over a finished game; series bookkeeping lives outside `skat-core`. |
| D-25 | 2026-10-29 | Public core data types derive `serde` traits behind the cargo feature `serde`. |
| D-34 | 2026-11-01 | Minimum Rust version 1.95, required by egui/eframe 0.37. |

## Analysis and opponents

| No | Changed | Current contract |
|----|---------|------------------|
| D-07 | 2026-11-01 | Classic is the default. Learned engines use permitted views and fall back to Classic. Calibrate engine strength by level and label uncalibrated options experimental. |
| D-35 | 2026-11-01 | The frontend paces opponents through AdvanceOpponent. The baseline uses the shared rules of thumb and plays legal actions. |
| D-37 | 2026-10-29 | Sample deals consistent with public constraints, weight bidding evidence, evaluate continuations and settle through skat-core. [Analysis](analysis.md). |
| D-38 | 2026-10-29 | Bound each search task by nodes and combine parallel results in deterministic order. Deadlines are an explicit cancellation mechanism. [Budgets](analysis.md#budgets-and-determinism). |
| D-39 | 2026-11-01 | Interactive pre-play estimates are coarse. Account for uncertainty when comparing options. |
| D-40 | 2026-11-10 | Classic levels use rules of thumb before card play and for Kontra. Perfect double-dummy defenders led to excessive passing and costly Kontras when search controlled those decisions. Keep Re and card play searchable. [Profiles and calibration](ai.md). |
| D-41 | 2026-11-01 | Goldfish forgets older cards. Mephisto sees the full deal, with privileged access explicit in its API. Use the special deck and table only in that mode. [Table behaviour](ai.md#mephisto). |
| D-62 | 2026-11-01 | God Mode plays against Mephisto, reveals the deal to the human and coach, and allows hiding the open cards. Use the dedicated blue table, faces and back. Localise suit names to match visible symbols and keep the usual suit name in brackets for replacements. |
| D-63 | 2026-11-12 | Keep offline model weights in git under assets/models/ with a training manifest, SHA-256 and model card (CC BY-SA 4.0). Keep training data outside git. Phase 2 ends after reinforcement learning and selection of one or two models. Each model file must stay below 10 MB. One active optional Neural file uses the [reversible operational v2 selection](neural-operational-selection.md), independent of the unchanged historical Phase 2 no-go; Classic stays default. [Runtime and selection](learned-models.md). |
| D-64 | 2026-11-12 | Offer the counting aid and in-game counting questions as two independent, default-off settings, derived only from the human's permitted view in `skat-analysis`. While a question is pending, the decision and coach are held back. Aid is hidden unless explicitly restored or visible assistance was selected; support is recorded. Focused retrieval (#48) reuses that snapshot and lifecycle with explicit support levels and plain in-memory events. [Counting aid and quiz](coach.md#counting-aid-and-quiz-d-64). |
| D-65 | 2026-11-12 | M7 recordings are explicit privileged plain data in `skat-app`, captured after successful shared core transitions and reconstructed in isolation through the core. Version format and replay semantics separately, validate original deal/seed and completion, retain completed games apart from scores. Bounded format-1 local files use exclusive creation with no overwrite, full semantic validation and an isolated read-only recording window. No cross-version seed stability or session resume. [Recording contract](architecture.md#privileged-recordings-in-skat-app-d-65). |
| D-66 | 2026-11-12 | Same-deal practice is a separate one-game `Session` built by `Session::start_practice` from a fully validated recording before bidding, with its exact cards, rules, game number, dealer, seats, table mode and opponent seeds; recorded actions are never replayed. It is never scored, recorded or exported, cannot deal another game, and rejects other sessions' results by lineage. The frontend parks the live table with its pending work and restores it unchanged. Coach samples as the original series; the quiz uses its own practice stream. [Practice contract](architecture.md#same-deal-practice-d-66). |
| D-67 | 2026-11-13 | Coaching a recorded decision is on demand, one human decision and one analysis at a time, through the shared `skat_coach::advise`/`review`. The default perspective is the human's historical permitted view before the action, sampled with a fixed seed derived only from that view. Hindsight (`advise_known` in the recorded deal) is a separate, deliberately requested and persistently labelled model comparison, never proof of a mistake. Replay jobs and results are their own types bound to the opened recording, request generation, decision, perspective, budget and seed; they never enter live coach, opponents or practice. Revealing cards for display never changes the perspective. [Replay comments](architecture.md#historical-and-hindsight-replay-comments-d-67). |
| D-68 | 2026-11-13 | Open display is an independent series choice, `SessionConfig::display` (`TableDisplay::Closed` by default; serde default, a closed table is not serialised so format-1 recordings keep their bytes and older files read as closed). It only adds `TableView::open_cards`; opponents, seeds, scores and the live coach's analysis input are unchanged, and `TableView::coach_basis` stays `OwnView`. Only God Mode gives the coach the real deal (`RealDeal`) and always shows every card. Display data (`OpenCards`) and the coach's known deal come from separate session paths. Every session has a unique lineage, so results of replaced sessions, including God Mode, are stale. Practice keeps the recorded display. [Open display](architecture.md#open-display-d-68). |
| D-69 | 2026-11-13 | Reviewing a whole recording is an explicit request for a completed recording only (in-progress files: unavailable). Each recorded human decision with a choice is analysed once, in game order and one at a time, as the historical D-67 job (permitted view before the action, view-derived seed, never hindsight); the unchanged shared `skat_coach::moments` (D-58) then selects zero to three moments with the learner's focus, which is no analysis input. Review jobs/results are their own types bound to opened recording, review generation, decision, budget and seed; results are cached per opened recording until the budget changes and reused only for that identity. Work is bounded by the coach budget per decision, 20 s per decision and 120 s in total; cancel and time limit keep the analysed decisions, and moments of a partial review are labelled as not covering the whole game. While it runs, no single comment starts. [Recording review](architecture.md#bounded-recording-review-d-69). |
| D-70 | 2026-11-13 | One local versioned learning history in `skat-app` (`LearningHistory`, format 2) stores the session's and the D-71 training's `RetrievalEvent`s per store-assigned attempt (series, practice, practice retry, training, training repeat); attempt IDs come from a persisted counter that survives reset, never from session lineages or seeds. Events are deduplicated by attempt, opportunity ID and action tag, and an opportunity must fit its attempt kind; no cards, deals or recordings are stored. The GUI keeps it in `learning-history.json` in the platform data directory, saves atomically after new events, and offers descriptive counts per topic and context with sample-size notes, no mastery or diagnosis. A damaged or unsupported file is kept unchanged and only renamed aside on the learner's choice; reset needs confirmation. Format 1 files are migrated losslessly in memory and written as format 2 on the next save; format 1 with training content is invalid. [Learning history](architecture.md#learning-history-d-70). |
| D-71 | 2026-11-13 | Focused counting training is plain state in `skat-app` (`TrainingSession`), outside any `Session`: the two focused-retrieval topics, each with versioned fixed fixtures (seeded `Game::new` deal plus a recorded legal action prefix, replayed through `Game::apply`) for declarer and defender. Only the intended seat's `PlayerView` is kept; question, expected answer, help and feedback come from its `Knowledge` through `skat_analysis::counting` as in the quiz, with the existing support levels and sticky assistance. Each presentation is one learning-history attempt (`AttemptKind::Training`), its events identified by `OpportunityId::Training` (fixture and version); retries and repeats are labelled, and no training row is new transfer or mastery. The GUI replaces the table with a training screen; the live table is neither drawn, driven nor changed until return. [Focused counting training](architecture.md#focused-counting-training-d-71). |

## Coach

| No | Changed | Current contract |
|----|---------|------------------|
| D-08 | 2026-11-01 | Coach levels: off / on request / important decisions / every move. Text plus figures that are off by default (D-52, D-60). |
| D-09 | 2026-11-13 | Review decisions after play. A warning before a weak move is optional and off by default. Persistent strategic error statistics are planned and will extend the D-70 learning history. |
| D-10 | 2026-10-29 | Plan an optional counting quiz and dedicated practice outside a live game. |
| D-13 | 2026-10-29 | The coach produces structured explanations (text keys + parameters); translation happens only in the presentation layer. |
| D-48 | 2026-11-01 | Use shared utility and uncertainty for weak moves, equivalence and warnings. Hold the selected action for confirmation and execute it at most once. [Policies](coach.md#policies-d-48). |
| D-49 | 2026-11-13 | Reveal help in stages. Early, default, automatic hints and unaided tasks conceal recommendations. Only an explicitly entered [guided dialogue](coach.md#guided-farbspiel-dialogue-131) may reveal the actual shared Easy rule action after an attempt or deliberate reveal. All legal actions remain playable; no auto-play. D-60 governs separate requested figures. Review ordinary moves after play. |
| D-50 | 2026-11-01 | Use concise, conditional lessons from shared evidence. A score difference alone cannot explain a strategic mechanism. Future practice distinguishes assisted from unaided attempts. |
| D-52 | 2026-11-01 | Figures default off. Put comparisons and assumptions behind debriefing details. When evidence is lacking, show an honest fallback. |
| D-53 | 2026-11-01 | Lesson evidence distinguishes exact observations from samples and tracks sources. Only reliable comparisons support mechanism lessons. [Evidence contract](analysis.md#evidence-for-lessons). |
| D-55 | 2026-11-01 | Teach remaining Trumpf and proven voids from exact evidence, respecting the contract and unknown Skat. |
| D-56 | 2026-11-01 | Teach vulnerability to Abstich and the opportunity for partner overtaking only when jointly feasible under the hidden-card constraints. |
| D-57 | 2026-11-01 | Teach own hand structure and attainable legal values before play. Keep bid coverage distinct from winning chances. |
| D-58 | 2026-11-01 | One optional learner-selected focus. Automatic hints require a teachable situation. Select up to three debriefing moments using lessons, focus and concept diversity. |
| D-59 | 2026-11-01 | Capture a permitted decision snapshot when the human acts. Reviews use the cards and history visible at that point. |
| D-60 | 2026-11-13 | Stage 3 requires both enabled figures and a request. Show estimates in neutral legal order. They can reveal model preference. Ordinary stages 1 and 2 keep live options unjudged. Guided rule answers (D-49) need no estimates; their optional separate model comparison retains original decision, objective, budget, uncertainty and basis and never rewrites the heuristic. |

## Frontend and assets

| No | Changed | Current contract |
|----|---------|------------------|
| D-14 | 2026-10-29 | Switchable card faces: French tournament deck with four colours (default), classic two-colour French deck, German deck (Altenburger Bild). All cards generated by script as SVG. |
| D-15 | 2026-10-29 | UI bilingual (German/English). |
| D-32 | 2026-10-29 | i18n with Fluent (`fluent-bundle`): one `.ftl` file per language under `assets/i18n/`, embedded into the frontend. `skat-app` returns plain data and structured errors; the frontend maps them to message ids. Tests check that both languages define the same ids and that every id the code uses exists. |
| D-33 | 2026-10-29 | Card faces are generated SVGs (`tools/cardgen`), committed under `assets/cards/` and embedded at build time; `egui_extras` renders them with `resvg`. Indices are stroked paths, so no font is needed. A test fails if the committed files differ from the generator output. German faces use the indices U, O, K, A. |
| D-45 | 2026-10-30 | Include headless screenshots from named scenarios in every frontend PR. [Workflow](testing.md#screenshots-of-the-frontend). |
| D-46 | 2026-10-30 | Show the series seed and its new-series input only when the developer seed option is enabled. |
| D-47 | 2026-10-30 | Open the new-series dialog on launch and delay opponent turns until the player starts or dismisses it. |

## Process, verification and licensing

| No | Changed | Current contract |
|----|---------|------------------|
| D-01 | 2026-11-01 | Licence for code: EUPL-1.2; content (`assets/`, `docs/`): CC BY-SA 4.0. SPDX header in every source file. |
| D-16 | 2026-11-01 | Repository language is English: code, comments, commits, docs, PR texts. Skat terms stay German ([glossary.md](glossary.md)). README is primarily German with an English section. |
| D-17 | 2026-10-29 | Branching: `main` plus feature branches; changes reach `main` only via pull request. Commit messages follow Conventional Commits. |
| D-18 | 2026-10-29 | Code of conduct: Contributor Covenant 2.1. |
| D-19 | 2026-10-29 | `AGENTS.md` is the single source of agent instructions; `CLAUDE.md` only imports it (`@AGENTS.md`). |
| D-27 | 2026-11-13 | Agents own delivery and may dispatch the issue implementation workflow. One independent fresh-session review (any provider) reviews each PR; named blocking findings get exactly one fix round, after which the CTO agent verifies those findings and the final required CI and merges without a second full review. The CEO does not review PRs. Repository protection changes need a separate CEO decision. |
| D-29 | 2026-10-29 | Use the implement-issue skill locally and in CI. Tick checklist items only after verifying them. |
| D-30 | 2026-11-12 | scripts/check.sh defines shared PR/native/complete tiers: Ubuntu FAST and deterministic integration, Windows compile/IO/model/GUI/render per code PR; broader Ubuntu on main; complete suites on both platforms nightly/manual/RC; weekly/manual deep. Conservative routing and strict ci-ok aggregation are required for merge. [Verification](testing.md). |
| D-36 | 2026-11-01 | Windows CI covers compilation and headless tests. Verify interactive Windows 11 play separately. |
| D-42 | 2026-10-30 | Third-party licences are allowed by an explicit SPDX list in [CONTRIBUTING.md](../CONTRIBUTING.md#allowed-third-party-licences): permissive code licences and MPL-2.0; OFL-1.1, Bitstream-Vera and Ubuntu-font-1.0 for fonts; public domain, CC0, CC BY and CC BY-SA 4.0 for content; any OSI licence for tools that are only run. Not allowed: GPL/AGPL/LGPL-only, source-available, NC/ND, unclear origin. Everything else needs the maintainer. |
| D-43 | 2026-10-30 | `THIRD-PARTY-NOTICES` in the repository root holds the licence texts and copyright notices of every crate in `Cargo.lock` (all platforms) and of the egui fonts. `tools/notices` generates it; where a crate ships no licence file, the standard text of its licence applies with its authors as copyright holders. A test fails when it misses a crate. Every binary or package handed out ships it next to the program. |
| D-44 | 2026-11-12 | Fast hooks conservatively check the workspace and skip slow tests. PRs retain deterministic integration; main/nightly/manual complete tiers own slow regressions (D-30). Documentation changes run lint. |
| D-51 | 2026-11-11 | Agents verify lesson claims with independent evidence and fixtures; independent PR review is required by D-27. The maintainer participates only as a learner in comprehension/transfer sessions, not as a technical reviewer. Unperformed learner checks remain pending. Expert certification is outside this protocol. |
| D-54 | 2026-11-01 | Use the [lesson checklist and learner protocols](coach-validation.md). Fix unaided tasks and measures before collecting results. |
| D-61 | 2026-11-11 | Agents own technical release validation on target platforms, fixes and retests before publishing 1.0. Actual learner sessions remain a distinct release check under D-51; missing platform or learner evidence stays pending. Automated verification remains part of each change. |

## Rule details

### D-03 Kontra/Re

1. **Who may say Kontra:** an eligible defender (D-26.1).
2. **When:** before that defender plays their first card (D-26.2), in
   Ouvert games before the reveal (D-26.4). Only one Kontra per game.
3. **Re:** only the declarer, and only immediately in reply to Kontra. No
   further escalation.
4. **Scoring:** Kontra doubles and Kontra plus Re quadruples the game
   points, including the doubled loss points. The bid value and the
   Überreizen check are unaffected. With Seeger-Fabian the extra +50/−50
   and +40 points are not multiplied.
5. **Game types:** Kontra and Re are allowed in Farbspiel, Grand and Null
   games, Hand games included. Unlike some pub rule profiles, Re is also
   allowed in Null and Null Hand.

Code: `RuleConfig::kontra_re` (off by default), `scoring::settle`,
`GameResult::seeger_fabian`. Tests: `crates/skat-core/tests/kontra.rs`.

### D-26 Kontra eligibility and timing

1. **Eligibility:** a defender may say Kontra if they bid or held a value.
   Hinterhand may also say Kontra if, at its first turn in the bidding, the
   standing bid was already above 18 and it passed. After Vorhand plays
   alone at 18 no defender is eligible.
2. **Timing:** an eligible defender may say Kontra instead of playing a
   card while it is their turn and they have not played a card yet.
3. **Re:** immediately after Kontra the declarer decides Re or no Re before
   anything else happens; then play continues with the same defender.
4. **Ouvert:** before the declarer's cards are revealed, each eligible
   defender decides once whether to say Kontra, in seat order; a Kontra
   ends the round with the declarer's Re decision. No Kontra is possible
   later in Ouvert play.

Code: `Bidding::kontra_eligible`, `skat_core::kontra_eligible` (from the
public bidding history), `Game::kontra_allowed_in_play`. Tests:
`crates/skat-core/tests/kontra.rs`.

### D-28 Rule interpretations in `skat-core`

Sources: the Internationale Skatordnung (ISkO), Altenburg 2002 edition
([PDF](https://www.spielkarten.com/wp-content/uploads/2019/07/0_Skat_InternationaleSkatordnung.pdf)),
read through its [English translation](https://www.weddslist.com/skat/ordnung/skatordnung3.html).
Section numbers refer to the ISkO.

1. **Bidding:** bids are values from the Reizwert table (5.6.1); jumps are
   allowed. If Mittelhand and Hinterhand pass without a bid, Vorhand
   either plays at 18 (`Bid(18)`, no other value) or passes (3.3.6); three
   passes throw the hand in (3.3.7, D-04).
2. **Null below the bid:** a Null game whose fixed value is below the bid
   cannot be announced (3.4.4); the engine rejects it with
   `NullValueBelowBid`.
3. **Lost and overbid games:** every lost game counts double (5.3.4). An
   overbid game is lost with the smallest multiple of its base value that
   reaches the bid, then doubled (5.4.1, 5.4.2).
4. **Levels:** announced Schneider/Schwarz always count their levels.
   Schneider (either party at 30 Augen or fewer) and Schwarz (either party
   without a trick) count when reached. A declarer who announced Schneider
   and is played Schneider gets no extra level (5.2.5).
5. **End of play:** Farbspiel and Grand are played to the tenth trick; Null
   ends as soon as the declarer takes a trick. Conceding and shortening
   (4.3) are not modelled.
6. **Ouvert Kontra round:** see D-26.4; seat order is Vorhand, Mittelhand,
   Hinterhand.
7. **Seats and scoring:** `skat-core` uses per-deal seats (Vorhand,
   Mittelhand, Hinterhand = dealer). Seeger-Fabian contributions are
   returned per seat; mapping seats to players belongs to the series layer.
8. **Dealing:** Fisher–Yates shuffle with rejection sampling on the
   `ChaCha8Rng` stream (`seed_from_u64`), dealt 3 – Skat 2 – 4 – 3 from
   Vorhand, so a seed always gives the same deal (`Deal::from_seed`).
9. **Fixed-deal tests** with hand-computed results in
   `crates/skat-core/tests/`: `variants.rs` (Grand, Kreuz, Pik, Herz and
   Karo with pickup, Hand, Ouvert, announced Schneider/Schwarz made and
   missed, Überreizen; Null Hand, Ouvert and Ouvert Hand, won and lost),
   `play.rs` (full Kreuz and Karo games, Null won and ending early, Null
   values against high bids), `kontra.rs` (Kontra/Re).

### D-31 Series semantics

1. **Players and seats:** in each deal the dealer is Hinterhand, the player
   to the dealer's left Vorhand. Player 2 deals the first game, so the
   human starts as Vorhand.
2. **Rotation:** after every deal, also a thrown-in one, the deal passes to
   the left.
3. **Einpassen:** a thrown-in deal (D-04) counts as a game of the series, a
   line on the score sheet with no points.
4. **Scoring:** each line holds the Seeger-Fabian points per player
   (`GameResult::seeger_fabian`, mapped from seats to players) and the
   running totals. The series result lists totals, games won and lost as
   declarer, and the ranking by total.
5. **Length:** a fixed series ends after its n-th game (default 36). In
   free play the human ends the series between games.
6. **Seeds:** the series seed seeds three `ChaCha8Rng` streams: stream 0
   draws one seed per deal, stream 1 the Goldfish draw and the opponents'
   seeds (D-40), stream 2 the coach's seeds. The same series seed and the same human commands repeat the
   series.

Code: `crates/skat-app/src/session.rs`, `series.rs`. Tests:
`crates/skat-app/tests/series.rs`.
