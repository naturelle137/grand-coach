# Decisions

Each entry states the decision as it holds now. To change a decision, edit
its entry in place and set "Changed" to the date of the edit; history stays
in git, not here. New fundamental decisions take the next free number;
numbers are never reused. Routine implementation details belong in code,
PRs and the focused references (architecture, analysis, AI, coach, testing).

## Product and rules

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-02 | 2026-10-29 | Rules: ISkO, all ISkO game types. Only exception: Kontra/Re (D-03). | One unambiguous reference; a complete engine is a prerequisite for coach and AI. |
| D-03 | 2026-11-01 | Kontra/Re as a switchable option, following the common German pub Skat variant (esp. the rules of the Online-Skatclub). [Details](#d-03-kontrare). | Not standard in ISkO, but relevant for training; the pub variant is the widespread one. |
| D-04 | 2026-10-29 | No Ramsch, no Bock. If all pass, the hand is thrown in. | Matches the ISkO and keeps the scope small. |
| D-05 | 2026-10-29 | Scoring Seeger-Fabian. Series with fixed length and free play. | Tournament standard, comparable performance. |
| D-06 | 2026-10-29 | Replay with coach comments and "replay the same deal". Games are deterministic via seed. | Learning by comparison; determinism also helps testing. |
| D-26 | 2026-11-01 | Kontra eligibility and timing as encoded in the engine. [Details](#d-26-kontra-eligibility-and-timing). | D-03 must be precise enough to implement and test. |
| D-28 | 2026-11-01 | Rule interpretations encoded in `skat-core`, with the ISkO sources used. [Details](#d-28-rule-interpretations-in-skat-core). | Ambiguities are resolved and documented, not invented silently. |
| D-31 | 2026-11-01 | Series semantics in `skat-app`: player 0 is the human, players 1 (left) and 2 (right) the opponents; player 2 deals first, then the deal passes to the left. A thrown-in deal (Einpassen) counts as a game of the series. [Details](#d-31-series-semantics). | Counting thrown-in deals follows tournament lists and keeps the deal rotation fixed. |

## Architecture

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-11 | 2026-11-01 | Tech stack: Rust (Cargo workspace) + egui; long term Godot via godot-rust (gdext) as a second frontend. Neural networks use [Burn](https://github.com/tracel-ai/burn) 0.22.1 (pinned, MSRV 1.95 as the workspace): `skat-nn` holds the model definition and CPU inference on Burn's pure-Rust backend Flex, `tools/nn-train` trains it outside the app on the GPU via wgpu/Vulkan or on the CPU, no ROCm. The app needs no Python, ONNX runtime, model server or download. The smoke test of [#69](https://github.com/naturelle137/grand-coach/issues/69) passed on CPU and on the RX 6900 XT ([ai.md](ai.md#learned-engine-runtime)); should Burn become unusable, this entry changes to PyTorch training with ONNX export and [tract](https://github.com/sonos/tract) inference. | Fast enough for Monte Carlo on all cores; one binary per platform; egui suits the plain style. One language for simulator, self-play, training and inference avoids a Python bridge and export parity checks; wgpu trains on the AMD GPU without ROCm; Flex needs no native libraries on Linux or Windows. |
| D-12 | 2026-10-29 | Frontends talk only to `skat-app` (commands, events, view models as plain data types). No frontend code in engine, analysis, AI or coach. | Godot must be attachable without touching the core logic. |
| D-20 | 2026-10-29 | Cards as index `0..32`, card sets as `u32` bitset (`CardSet`). See [architecture.md](architecture.md). | Fast set operations and cheap state clones for Monte Carlo and double-dummy search. |
| D-21 | 2026-10-29 | `skat-core` is one state machine (`Game` with a `Phase` enum) offering `legal_actions` and `apply`; `apply` validates before mutating, so rejected actions leave the state unchanged. Errors are structured enums without texts. | One type for every position suits simulators and search; no partial updates. |
| D-22 | 2026-10-29 | A game is described by seed, rule config and action sequence. Dealing is the only randomness in the core and uses `rand_chacha::ChaCha8Rng` (D-28.8). | Reproducible replays and tests; ChaCha8 output stays stable across `rand` versions, unlike `StdRng`. |
| D-23 | 2026-11-01 | AI and coach receive a `PlayerView` per seat. Full knowledge only through an explicitly named `OmniscientView` (open mode, post-game analysis, God Mode's table view and coach, D-62) or, for Mephisto only, the whole `Game` (D-41). | Makes the information-set rule visible and checkable in the type system. |
| D-24 | 2026-10-29 | Game value, Überreizen and Seeger-Fabian are pure functions over a finished game; series bookkeeping lives outside `skat-core`. | Scoring is testable in isolation from play. |
| D-25 | 2026-10-29 | Public core data types derive `serde` traits behind the cargo feature `serde`. | Replays and `skat-app` can serialise them; the core stays lean without it. |
| D-34 | 2026-11-01 | Minimum Rust version 1.95, required by egui/eframe 0.37. | The frontend needs a current egui; Rust 1.95 is widely available. |

## Analysis and opponents

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-07 | 2026-11-01 | Opponent strength in levels (D-40) plus table modes (D-41). Opponents see only their seat's `PlayerView`, Mephisto excepted (D-41). The opponent engine (Classic, or an offline learned or hybrid engine) is a choice separate from the level. Classic is the default, and every learned path falls back to Classic when a model is missing, broken or does not cover a phase. Learned engines use only the acting seat's `PlayerView` and share their computation with the coach through `skat-analysis`. They may ship labelled "experimental"; a level loses the label once the calibration protocol in [ai.md](ai.md) orders it against its neighbour. | Fair training, an understandable choice. Implementing learned engines is a maintainer goal in itself; with Classic as default and reference, a learned engine need not beat it to be offered, and measured strength is reported instead of gating the work. |
| D-35 | 2026-11-01 | The frontend paces opponent moves with the command `AdvanceOpponent`. The rules-of-thumb opponent `BaselineOpponent` receives only its seat's `PlayerView` and picks a listed legal action using the shared rules in `skat-analysis::policy`. | Legal, fair, reproducible opponents; pacing in the frontend keeps `skat-app` free of timers. |
| D-37 | 2026-10-29 | Analysis model: `analyze` takes one `PlayerView`; hidden cards are sampled uniformly within hard constraints (exactly counted, enumerated when few) and weighted by bidding evidence; each legal option is played out per sample with the rules of thumb (`skat-analysis::policy`) as continuation policy and double-dummy card play; `skat-core` settles. Estimates are from the deciding seat's view: win chance, expected Seeger-Fabian points, expected declarer game points. Details in [analysis.md](analysis.md). | One shared computation for AI and coach with defined semantics; perfect-information Monte Carlo is the established approach for Skat. |
| D-38 | 2026-10-29 | Analysis budgets count search nodes per task (one distribution, or one Drücken option), never time; each task uses a fresh solver and tasks run in parallel, so results do not depend on scheduling. Time limits come from `Cancel::with_deadline`. Latency target and reference hardware in [analysis.md](analysis.md#latency-target-and-benchmark). Drücken screens all 66 options on 3 samples and evaluates the best 6 on all. | Reproducible results (D-06) with bounded interactive latency; Drücken would otherwise cost 66 full searches per sample. |
| D-39 | 2026-11-01 | Estimates before card play are coarse (standard error 17–20 points at the interactive budget); improving them is an open point. | All measured decisions meet the latency target; better pre-play estimates need more than the interactive budget. |
| D-40 | 2026-11-01 | Strength levels: Rookie plays the rules of thumb with seeded mistakes; Easy the rules of thumb without search; Medium and Strong let `skat-analysis` decide card play and Kontra/Re (6 and 24 samples) and use the rules of thumb for bidding, Hand or pickup, Drücken and announcement. They pick the option with the highest utility (own expected Seeger-Fabian points; a defender also counts half of the declarer's game points); ties go to the rules-of-thumb action. Opponent seeds come from a separate stream of the series seed, never from a deal seed; a decision's seed depends only on that seed and the view. Without an estimate the rules of thumb decide. Calibration in [ai.md](ai.md). | Analysed bidding was too cautious (double-dummy defenders are perfect) and seeded errors only weakened Medium/Strong; card-play analysis gave clear, low-variance steps. |
| D-41 | 2026-11-01 | Table modes (maintainer decision): Goldfish (one opponent drawn per series plays Strong but forgets cards of older tricks, the other Medium) and Mephisto (decides with every card known through `World::from_omniscient` and `analyze_known`: both opponents before card play; in card play the computer declarer while its partner plays Strong, and both defenders against the human declarer). `Mephisto::decide` takes the whole `Game`. Mephisto has its own card faces (Jacks with index M and a devil, Kreuz with an inverted cross), back (flames in the gate to hell) and red table, shown only while it plays. Details in [ai.md](ai.md). | An unbeatable opponent to beat together with a strong partner. |
| D-62 | 2026-11-01 | God Mode (maintainer decision, issue #28; name "God Mode" in German and English): the table mode `Opponents::GodMode` plays against Mephisto exactly as D-41, but the human sees every card, the Skat included, through `TableView::open_cards` (an `OpenCards` built from the `OmniscientView`, only in this mode); the GUI can switch the open cards off. The coach evaluates the options in the real deal (`skat_coach::advise_known`, `analyze_known`) and keeps every other rule, above all no telling ahead (D-49); its inferences stay those of the human's own information set, and basis and assumptions say the deal is known. Own look, only in this mode: the Jacks carry the index G and an angel; the suits carry neutral light symbols instead of religious ones (upright Latin cross for Kreuz/Eichel, crescent moon for Pik/Grün, radiant heart for Herz/Rot, star of light for Karo/Schellen); a bright back with the all-seeing eye; a blue table. | A worthy opponent for Mephisto: human with divine support against the machine. Exact estimates make the coach's figures and debriefing reliable, while hints without the answer keep it a human game. |

## Coach

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-08 | 2026-11-01 | Coach levels: off / on request / important decisions / every move. Text plus figures that are off by default (D-52, D-60). | The learner controls how much help they get. |
| D-09 | 2026-10-29 | Feedback: post-game analysis and error statistics over time. Warning before a weak move is an optional setting, off by default. | Think for yourself first, evaluate afterwards; the warning is there for those who want it. |
| D-10 | 2026-10-29 | Card-counting quiz during play. A dedicated training mode is a later stage. | Card counting is the user's biggest weakness. |
| D-13 | 2026-10-29 | The coach produces structured explanations (text keys + parameters); translation happens only in the presentation layer. | "The coach only verbalises what was computed"; works with both frontends and both languages. |
| D-48 | 2026-11-01 | Coach policies: explanations come only from `skat-analysis` (observations, opponent-model inferences, estimates ranked by `utility`). An option is weak at ≥ 10 Seeger-Fabian points below the best and beyond 2 combined standard errors; equivalent within 1 point or that margin. A decision is important when the analysis is complete and a natural option (not a jump bid, not a Schneider/Schwarz announcement) is weak. Warnings need a complete analysis and a weak chosen action; the action is held, never replaced, played once on "proceed", and not warned twice at the same position. Level default "on request", warnings off; coach seeds come from their own stream. Details in [coach.md](coach.md#policies-d-48). | Tying "weak" to the measured standard errors keeps warnings from firing on noise, especially before card play (D-39). |
| D-49 | 2026-11-01 | No telling ahead: before the move the coach never names, ranks or labels the best option (stage 3 figures excepted, D-60). Each help request reveals one stage: 1. objective, sure facts, likely inferences; 2. the options in neutral legal order plus a qualitative hint (one stands out / several about equal / forced / unclear) and the important mark; 3. every option's figures, unjudged (only with "show figures"). After each own move a short note judges it (good, about as good, somewhat or clearly weaker); the debriefing explains selected moments (D-58). The warning names no alternative. With the coach on, every own decision is analysed, if needed after the move. `skat-coach::Hints` carries only the revealed stages. Details in [coach.md](coach.md#no-telling-ahead-d-49). | Players learn nothing by playing what the computer says; hints without the answer, evaluation afterwards. |
| D-50 | 2026-11-01 | Didactic coach direction: concise review with estimates hidden by default, evidence-backed conditional lessons from shared `skat-analysis`, no preferred live action or plan, and assistance-aware independent practice. Packages and status in [coach-didactics.md](coach-didactics.md) and [coach-work-packages.md](coach-work-packages.md). | Score comparisons do not explain strategic causes; separate verified facts, model preferences, teachable mechanisms and observed understanding. |
| D-52 | 2026-11-01 | Readable debriefing: "show figures" is off for new coach settings; an explicit choice is kept. A moment shows only decision, chosen action and the model's qualified judgement, for a good moment the trap the model rates clearly lower, and the fallback "no strategic explanation yet"; comparisons per labelled metric, model ratings, figures (with "show figures"), considerations and assumptions are collapsed details. A forced last move gets no note after the game. Details in [coach.md](coach.md). | Score differences are no strategic cause, so no causal sentence is made of them. |
| D-53 | 2026-11-01 | Evidence contract for lessons: `skat-analysis::lesson` provides `Evidence` (observations as exact facts, inferences as sample summaries, each with provenance in the view; a decision `Snapshot`; no action), `Comparison` (after the move: chosen versus best option, reliable only for a complete analysis with both options evaluated on every distribution) and versioned `Lesson` definitions (applicability, permitted evidence classes, conditions, exclusions, supported or unsupported). Conditions on facts match only exact items, never samples. Fact lessons stay valid with an incomplete search; mechanisms need the move and a reliable comparison. `skat-coach::teach` selects the first applicable lesson, otherwise `Teaching::Fallback`; hints carry evidence and fact lessons only. Details in [analysis.md](analysis.md#evidence-for-lessons). | Separates a fact, a model comparison and a strategic mechanism in the types, so cancelled or screened results cannot turn into reliable claims and live hints cannot name a preferred action. |
| D-55 | 2026-11-01 | Exact lessons `proven-void` (Farbspiel, Grand, Null) and `remaining-trumpf` (Farbspiel, Grand): fact lessons on exact evidence during card play for every role, only when the fact rests on a card of the current or last completed trick (`Condition::Recent`), void first. Unseen Trumpf are never assigned to a seat; a void teaches an exclusion, not a winning plan. Details in [coach.md](coach.md#exact-lessons-d-55). | Exact knowledge needs no search and no expert; limiting lessons to new facts avoids repeating them at every card. |
| D-56 | 2026-11-01 | Card-play mechanisms `trumpf-vulnerability` (a later opponent can trump the seat's winning side Ass or Zehn) and `partner-overtake` (a defender playing second to the declarer's lead leaves the trick to a partner who can beat it), Farbspiel and Grand. `skat-analysis::mechanism` decides each condition exactly over the admissible hidden distributions and reports `Possible` or `Certain`. Mechanism lessons apply after the move, with a reliable comparison, only when the mechanism holds, and come first in the catalogue. Texts never say which card was better. Details in [coach.md](coach.md#card-play-mechanisms-d-56). | Joint conditions from the view are exact; keeping the mechanism apart from the model rating avoids turning a score difference into a cause. |
| D-57 | 2026-11-01 | Pre-play lessons `hand-structure` (own Buben, Trumpf and Spitzen per Farbspiel and Grand; first bidding call and discarding) and `game-value` (each contract's value range against the value the game must reach; later calls, Hand or pickup, announcement), for bidder and declarer, derived exactly by `skat-analysis::preplay` (an unknown Skat counts for the Spitzen). Texts name every candidate game in fixed order and never a preferred bid, game or discard; the move's rating is labelled a coarse pre-play estimate. Details in [coach.md](coach.md#pre-play-lessons-d-57). | Structure and legal values are exact rule facts; a safe bid limit is not, and pre-play estimates stay coarse (D-39). |
| D-58 | 2026-11-01 | Focus and teachable moments: `CoachSettings::focus` is one optional `Concept`, chosen by the player only and kept until changed. Pedagogical eligibility (`Explanation::teachable`) is separate from the important mark and from warnings. Level "important" shows the situation at important *and* teachable decisions, "every move" the first stage where a lesson applies and `Advice::NoLesson` elsewhere; asking always works. Stage 2 adds one neutral comparison question, identical for every option. `Command::SkipAdvice` hides the advice until the next decision. Debriefing: zero to three primary moments, each a non-forced decision with a supported lesson, at most one per concept, chosen by focus, clear complete model rating (good: trap avoided; bad: loss ≥ 5), mechanism before fact, loss, game order; shown in game order. A provisional analysis is never rated. Details in [coach.md](coach.md#focus-and-teachable-moments-d-58). | A large score gap is no learning value; choosing by lesson, focus and concept keeps moments explainable and varied. |
| D-59 | 2026-11-01 | Decision snapshots: when the human acts, `skat-app` keeps a plain serde `DecisionContext` built only from the human's `PlayerView` before the action; it travels with the queued decision and the stored review, and `DebriefingMoment` pairs each debriefing moment with its snapshot. No full `Game`, hidden hand or later fact is stored. Details in [coach.md](coach.md#decision-snapshots-d-59). | A learner cannot understand an earlier choice from the final table; the same information set as the analysis keeps the snapshot permitted by construction. |
| D-60 | 2026-11-01 | Figures exception to D-49 (maintainer decision): stages 1 and 2, and every path with "show figures" off (default), never name, rank, label or single out an option before the move. Only with "show figures" on *and* a request does stage 3 show every option's model estimate in neutral legal order without labels; a player may infer the model's preference from it. Switching figures on reveals nothing by itself; off hides them at once. Exact live facts stay visible either way. | Comparing figures is a deliberate, opt-in learning aid; stating the exception keeps D-49 and its tests honest. |

## Frontend and assets

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-14 | 2026-10-29 | Switchable card faces: French tournament deck with four colours (default), classic two-colour French deck, German deck (Altenburger Bild). All cards generated by script as SVG. | Legibility, choice for either deck, no licensing risk. |
| D-15 | 2026-10-29 | UI bilingual (German/English). | Accessibility. |
| D-32 | 2026-10-29 | i18n with Fluent (`fluent-bundle`): one `.ftl` file per language under `assets/i18n/`, embedded into the frontend. `skat-app` returns plain data and structured errors; the frontend maps them to message ids. Tests check that both languages define the same ids and that every id the code uses exists. | Fluent handles parameters and plurals; the files stay usable by Godot. |
| D-33 | 2026-10-29 | Card faces are generated SVGs (`tools/cardgen`), committed under `assets/cards/` and embedded at build time; `egui_extras` renders them with `resvg`. Indices are stroked paths, so no font is needed. A test fails if the committed files differ from the generator output. German faces use the indices U, O, K, A. | Sharp at every size, no font or licence risk. |
| D-45 | 2026-10-30 | Screenshots are rendered headlessly: `gui-egui/examples/screenshots.rs` drives named scenarios with `egui_kittest` and renders them with wgpu (software rasterisers lavapipe/WARP). CI job `screenshots` renders all scenarios on Linux and Windows when `crates/gui-egui/` or `assets/` changed; it is not part of `ci-ok`. Every frontend PR shows screenshots. PNGs live on the orphan branch `screenshots` (`issue-<n>/`, `pr-<n>/`), never on `main`, embedded via raw URLs. | Reviewers validate from a picture faster; driving a real desktop window fails in CI and under KDE/Wayland; binaries stay out of `main`. |
| D-46 | 2026-10-30 | The series seed is a developer option: "Settings → Show seed", off by default, shows it in the status line and as an input in "New series …"; when off, a new series continues with the previous seed plus one. The input accepts only whole numbers from 0 to `u64::MAX`. `GRAND_COACH_SEED` turns the option on. Settings are not persisted. | The unexplained number confused players; a checkbox keeps replays available in release builds. |
| D-47 | 2026-10-30 | The window opens with the "New series" dialog, filled with the settings and seed of the series already dealt, so "Start" unchanged and "Cancel" both play that series. While the dialog is open, opponents do not move. | Players could not tell which settings the first series used; opponents should not bid behind an unanswered dialog. |

## Process, verification and licensing

| No | Changed | Decision | Rationale |
|----|---------|----------|-----------|
| D-01 | 2026-11-01 | Licence for code: EUPL-1.2; content (`assets/`, `docs/`): CC BY-SA 4.0. SPDX header in every source file. | Set by the project owner. |
| D-16 | 2026-11-01 | Repository language is English: code, comments, commits, docs, PR texts. Skat terms stay German ([glossary.md](glossary.md)). README is primarily German with an English section. | Open to international contributors; Skat terms are established. |
| D-17 | 2026-10-29 | Branching: `main` plus feature branches; changes reach `main` only via pull request. Commit messages follow Conventional Commits. | Simple, sufficient for a small project; readable history. |
| D-18 | 2026-10-29 | Code of conduct: Contributor Covenant 2.1. | Widely used standard. |
| D-19 | 2026-10-29 | `AGENTS.md` is the single source of agent instructions; `CLAUDE.md` only imports it (`@AGENTS.md`). | Works for every agent; no two copies that drift apart. |
| D-27 | 2026-10-29 | Claude may implement issues through the manually triggered workflow `implement-issue.yml`. The maintainer picks the issue; the result is a regular PR to `main` that a human reviews. No scheduled runs, no automatic issue selection, no automated review bots. Auth via the subscription OAuth token. | Offloads well-specified issues while keeping human review and D-17. |
| D-29 | 2026-10-29 | Issue implementation follows the project skill `implement-issue`, locally and in CI (the workflow only adds CI notes). Agents tick an issue's checklist items only when done and verified; in CI the workflow applies only `[ ]` → `[x]` changes. | One procedure for all runs; the issue shows what is done. |
| D-30 | 2026-11-01 | Verification gate: `scripts/check.sh` (tiers `lint`, `test`, `quick`, `push`, `full`, `deep`) is the single definition of the checks; linters are rustfmt, clippy, markdownlint, shellcheck and actionlint. Git hooks (`.githooks/`, pre-commit `quick`, pre-push `push`) give local feedback; Claude Code hooks activate them and block bypassing. The only hard gate is the CI check `ci-ok` (always `lint`; `test` on Linux and Windows plus MSRV when code changed), required on `main`; `implement-issue.yml` dispatches CI itself and opens a draft PR on a red gate. Details in [testing.md](testing.md). | Hooks can be skipped and do not exist in GitHub Actions; only a server-side required check enforces "no failing tests on `main`". One script avoids drift. |
| D-36 | 2026-11-01 | Windows 11 is not verified interactively; Windows is covered by the CI build and headless GUI tests on Windows Server. The gap is stated in [testing.md](testing.md#platform-verification-m2). | No Windows 11 machine is available; the gap is stated rather than blocking work. |
| D-42 | 2026-10-30 | Third-party licences are allowed by an explicit SPDX list in [CONTRIBUTING.md](../CONTRIBUTING.md#allowed-third-party-licences): permissive code licences and MPL-2.0; OFL-1.1, Bitstream-Vera and Ubuntu-font-1.0 for fonts; public domain, CC0, CC BY and CC BY-SA 4.0 for content; any OSI licence for tools that are only run. Not allowed: GPL/AGPL/LGPL-only, source-available, NC/ND, unclear origin. Everything else needs the maintainer. | A fixed list keeps EUPL-1.2 and CC BY-SA 4.0 open and lets agents decide without legal judgement; strong copyleft would force a licence change, LGPL static linking is unclear in Rust. |
| D-43 | 2026-10-30 | `THIRD-PARTY-NOTICES` in the repository root holds the licence texts and copyright notices of every crate in `Cargo.lock` (all platforms) and of the egui fonts. `tools/notices` generates it; where a crate ships no licence file, the standard text of its licence applies with its authors as copyright holders. A test fails when it misses a crate. Every binary or package handed out ships it next to the program. | The permissive licences require the notices with every binary copy; generated from `Cargo.lock`, the list cannot fall behind. |
| D-44 | 2026-11-01 | Fast gate: pre-push runs `push` (lint, clippy, fast tests of the packages affected since the pushed base); tests taking 2 s or more carry `#[ignore = "slow: CI only (docs/testing.md)"]` and run in CI and `full` only. `test` runs the workspace tests once with every feature except GPU backends (which need an adapter and stay manual) and checks the feature-free build per package with clippy. The broad check that Mephisto never loses a declared game runs with 100 games in `deep`. | Hooks should answer in seconds while CI keeps full coverage; Mephisto's choices are exact, so any loss is a bug. |
| D-51 | 2026-11-01 | Coach validation needs no human Skat expert: correctness comes from independent evidence and fixtures; clarity and usability from the maintainer as learner in release validation (D-61); the unaided-task protocol runs with [#54](https://github.com/naturelle137/grand-coach/issues/54). | No expert is available. Learner feedback and AI agreement cannot certify strategy; unsupported explanations are narrowed or omitted. |
| D-54 | 2026-11-01 | [coach-validation.md](coach-validation.md) defines (A) a per-lesson evidence checklist (permitted view; exact fact versus deduction versus model estimate; conditions and exclusions; positive, boundary and counterexample fixtures with expected results written by hand or from an independent exhaustive reference; hidden-deal substitution; live-leak and fallback checks), applied in each lesson PR; (B) a learner UAT script needing no Skat expertise; (C) an unaided-task protocol fixed before any result. Agents never record learner UAT. | Re-running the same solver or asking an AI is no independent oracle; fixing measures before results keeps assisted success from being read as transfer. |
| D-61 | 2026-11-01 | Manual validation is a release 1.0 task, performed by the maintainer after the release scope is implemented and before publication. No per-feature or per-milestone validation records or handoffs. Automated verification remains part of implementation. See [testing.md](testing.md#release-10-validation). | Keeps development and release testing separate. |

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
