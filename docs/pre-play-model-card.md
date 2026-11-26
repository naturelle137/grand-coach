<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Model card: learned pre-play imitation (#74)

**Status: experimental opt-in engine; model `1bc4f58c…79f5` ships as `assets/models/pre-play.gcnn`.** The 2026-11-19 runs
(below) play complete games; the first was clearly weaker than Classic Easy and
Strong, the improved recipe (W = 4) is indistinguishable from Easy and still
weaker than Strong. This card fixes what a model must cover and records label evidence
and measured results. Classic stays the default. The new
[opt-in engine](ai.md#opt-in-learned-pre-play-engine-74) lets this model decide every phase it covers.

| Item | Value |
|------|-------|
| Intended phases | Reizen, Hand/pickup, Drücken (joint pair, 66 legal pairs of 496 indices), Ansage; Kontra/Re only as analysed teacher decisions |
| Encoder / head | shared v2 encoder (634 permitted-view features), 621-action policy head; `train --all-phases` ([tooling](learned-models.md#all-phase-imitation-preparation-74)) |
| Teacher | Strong profile of #70 records; pre-play labels are the shared Classic rules of `skat-analysis` |
| Input | the acting seat's `PlayerView` only |
| Weights / hashes | experimental, not in the repo: SHA-256 `6ed45d3400b71b42adf4dc7333c14a59a613056623914c3c49b628fae5ad9aa8` (4×384, 5,621,708 bytes, trained on `T-2-10k`, `--epochs 4`) |

## Label validation

`nn-train validate-labels DIR` first runs `verify` (full replay, checksums,
score check), then recomputes each Reizen, pickup, Drücken and Ansage rule
from the stored view alone and compares it with the label. It also counts
views carrying a result and discards that are not a pair of the twelve cards.

Measured once on a fresh Strong teacher-only run (seed 0, 40 deals, 10 per
shard, engine commit of this change): Reizen 286/286, Hand/pickup 40/40,
Drücken 39/39 and Ansage 40/40 labels equal the rule; no leaks, no illegal
pairs. The Strong profile has `search_bidding`, `search_declaring` and
`search_kontra` off, so agreement is expected by construction; it confirms
the pipeline stores the rule's choice, unleaked and legal. It is a 40-deal sample: it says nothing about strategic
quality, rare contracts, Kontra/Re labels or double-dummy bias (D-40), and a
production dataset must be validated again.

A further read-only validation of the retained Strong `T-2-10k` dataset
(seed 73, engine `99c91300dee7`, 10,000 deals) found 82,866/82,866
Reizen, 9,931/9,931 Hand/pickup, 9,908/9,908 Drücken and 9,931/9,931
Ansage labels equal the current rules, with zero view leaks or illegal
pairs. `nn-train validate-labels` also verified the shards and replayed the
records. This checks storage and rule agreement, not strategic quality or
Kontra/Re label quality; the original manifest predates the explicit
`search_kontra` profile field and must not be treated as proof of that
phase's teacher configuration on its own. The recorded engine commit
`99c91300dee7` predates the Kontra rule change: its Strong profile searches
all Spiel decisions, including Kontra/Re, with the recorded 24-sample budget;
`teach` stores that decision and its analysis. This identifies the intended
teacher from the source, subject to the manifest accurately identifying the
generating binary.

A read-only scan of every retained shard found 6,891 views offering Kontra:
6,854 labels said Kontra and 37 played a card (99.46% Kontra). All were legal,
all had recorded analysis options, none recorded a fallback, and 14 recorded
`BudgetExhausted` rather than `Complete`. The resulting 6,854 Re views had
2,550 Re and 4,304 KeinRe labels; all were legal, had options and no recorded
fallback, with one `BudgetExhausted`. Counts describe recorded teacher output,
not independent reproduction of each choice or proof of its strategic quality.
The extreme Kontra rate agrees with the known D-40 concern and does not justify
shipping a model trained on these labels without separate assessment.

`validate-labels` reports Kontra and Re positive and negative labels, incomplete
analyses, fallbacks and missing option lists separately for each deal split.
It explicitly flags a split missing either class, including zero opportunities.
These diagnostics expose coverage and teacher failures; they do not establish
strategic quality or make the retained labels suitable for training. A representative
independent assessment, including Re, remains necessary.

## First training run

Details and tables in [ai.md](ai.md#first-all-phase-imitation-run-74). The
model covers all five phases (Kontra/Re stays with the rules in the opponent).
Pooled teacher agreement 71.59 % on 42,322 test rows; in 1,200 complete games
it scored -34.60 [-40.27, -28.94] against Easy and -43.39 [-49.13, -37.65]
against Strong per seat-game. It passes too rarely (14.9 % of 6,883 Reizen
actions, Strong 23.5 %) and never declares Null, Hand or announcements
(0 of 774 declarer games). It is a weak first result.

## Improved recipe

[Details](ai.md#improved-imitation-recipe-74): three times the data and
`--preplay-weight 4`. Test agreement by phase (rows): Reizen 96.41 % (23,034),
Hand/pickup 99.68 % (2,827), Drücken 79.29 % (2,815), Ansage 91.44 % (2,827),
Kontra/Re 95.03 % (3,967), card play 73.28 % (84,803); model SHA-256
`c7927678a16dd6e9151b4bb1c895617405bb39305fb76bc5c45493f596b58e1a`. In 1,200
complete games: -2.10 [-6.40, +2.21] against Easy, -14.99 [-18.92, -11.07]
against Strong; Pass 23.6 % of 5,099 Reizen actions (Strong 23.9 %), but Hand
and announcements 0 of 599 declarer games and a 70.3 % declarer win rate
against Strong's 86.6 %.

## Open before a production claim

Rare classes (Hand, announcements), declarer strength, repeat seeds, validated
Kontra/Re integration (#108), and weights licence and provenance in
[third-party](third-party.md) remain open. The Drücken outcome-label pilot and
its limitations are recorded in [ai.md](ai.md#rare-pre-play-classes-and-the-learned-kontrare-head-74);
no model has been trained on those labels.

Phase-masked check ([ai.md](ai.md#phase-masked-learned-pre-play-74)): with
Classic deciding Hand/pickup and Ansage, the W = 4 model scores -15.10
[-18.97, -11.23] against Strong, i.e. no improvement.

## Lower rare-class weight experiment

The W = 4, R = 2 model covers the same all-phase action head, including
Kontra/Re, and remains local and unshipped (SHA-256
`acaf10a4c3e117ea09be03282339c5319aa1ffb9cefdea8a5c93e18dc8bf1d88`).
Its held-out Drücken agreement is 81.03 %, but complete games against Strong
scored -16.99 points per seat-game with the existing Kontra rule and -22.09
with learned Kontra/Re. See [the experiment](ai.md#lower-rare-class-weight-experiment-74)
for denominators, behavioural rates and limitations. The source Kontra labels
remain extreme, and neither model is established as suitable for integration.

## Drücken-specific loss weight

With `--discard-weight 4` on top of W = 4, R = 2 the held-out Drücken
agreement is 86.68 % (2,815 test views; 81.03 % without). Complete games
against Strong scored -18.02 points per seat-game (95 % interval
[-22.29, -13.75]), not distinguishable from the W = 4, R = 2 model. Model
SHA-256 `b1a2c6365e37c5e5f9de5198e54d7ff970554e3d0bc8c10dae540ed33b33af54`,
local and unshipped; see [the measurement](ai.md#drücken-specific-loss-weight-74).

### Synthetic announcement positions

`cargo run --locked -p nn-train --example rare_positions -- 7 100` emits
JSONL diagnostic positions, not training shards or teacher labels. Easy plays
bidding, Hand is forced, and every legal non-Ouvert Schneider or Schwarz
announcement is emitted as a synthetic candidate. The prefix and run seed
allow replay; only the `view` is a permitted model input. Candidates are not
quality judgements and must not be used as imitation targets without an
independently evaluated teacher. The deal-derived split groups all candidates
for one deal.

For seed 7 and 100 attempted deals, all 100 reached announcement. Each of
Schneider (without Schwarz) and Schwarz had 385 train, 50 validation and 65
test candidates. These count candidate actions, not distinct deals or good
announcements. The example test reconstructs each view and checks legality
and split assignment; it does not validate playing strength.

On these same 100 distinct positions, the output also records the current
Strong decision from the acting view. Strong chose no Schneider or Schwarz
announcement (0 of 1,000 candidates matched); the positions comprise 77
training, 10 validation and 13 test deals. Strong uses its declaration rule
rather than search. This demonstrates missing positive labels for these
candidates, not the quality of the choices or an evaluation of another teacher.
The recorded `strong_action` is diagnostic and is not a training label.

An experimental Strong profile with declaration search was assessed on the
same 100 positions using one sampled world and 10,000,000 nodes per task:
`cargo run --locked --release -p nn-train --example rare_positions -- 7 100 |
cargo run --locked --release -p nn-train --example assess_declarations`.
It completed 98 decisions, exhausted its budget and fell back twice, and
selected an extended announcement twice. The deal splits were 77 train, 10
validation and 13 test. One sampled world and this small, synthetic sample do
not establish label quality or playing strength. These decisions are not
training labels; independent evaluation remains necessary.

The independent `evaluate_declarations` diagnostic compares the Classic and
searched declaration choices on the test split only. It samples fresh worlds
from each acting view, replays the public bidding and Hand choice, and plays
both choices and each legal synthetic Schneider/Schwarz candidate against the
same frozen Easy players. It reports per-deal scores, paired candidate differences
and their standard errors across worlds, the searched-versus-Classic difference
and its standard error across deals, and declaration and continuation fallbacks. The sampled worlds satisfy hard constraints;
they do not condition on the opponents' bidding strategy. These are diagnostic
outcomes, not training labels or evidence of general teacher quality. Reproduce
with a separate sampling seed, for example:

```sh
cargo run --locked -p nn-train --example rare_positions -- 7 100 | cargo run --locked -p nn-train --example evaluate_declarations -- 8 4639
```

The command above reused 13 previously inspected test views with eight fresh
paired worlds each and assessed 130 candidate actions: searched
minus Classic was -5.64 Seeger-Fabian points per declarer game, with a
17.17-point standard error across deals. Search fell back once; the 208
continuation games for those choices and 1,040 candidate games had no fallbacks. This small conditional sample cannot
establish an ordering or select a teacher.

## Longer training and wider network

A 4×512 model (D = 4, 12 epochs, SHA-256
`8f0dc3872f7534ccdc01c23c00b2020846bec7ebe403e5e92bb53b94f65705a5`, local and
unshipped) reaches 74.40 % card-play and 87.53 % Drücken test agreement and
-17.09 points per seat-game against Strong (95 % interval [-21.28, -12.90]),
not distinguishable from 4×384; see
[the measurement](ai.md#longer-training-and-wider-network-74).

## Higher Drücken weight

The unshipped 4×384 D = 8 model reached 83.91 % joint-pair agreement on
2,815 test views, below 86.68 % for D = 4. It selected Kontra on all 1,987
test opportunities; the teacher selected it on 1,980. See
[the measurement](ai.md#higher-drücken-weight-74) for the other phases and
the limits of this single-seed comparison. Against Strong in 1,200 complete
games with the existing Kontra rule, it scored -17.02 points per seat-game
(95 % interval [-21.50, -12.54]); no Null, Hand or extended announcement was
chosen in 594 declarer games. It is not a production candidate.
