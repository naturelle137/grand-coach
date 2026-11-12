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
  opponents and the closed-play coach receive a permitted view.

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
authentication against deliberate metadata rewriting. Future file IO must
also bound input bytes before parsing. No disk IO, replay UI, editable history,
same-deal attempts or training history is implemented here.
