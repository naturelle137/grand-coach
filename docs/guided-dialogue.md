# Guided Farbspiel, Reizen, Skataufnahme, Drücken, Ansage and Kontra/Re pilot (#131, #148, #151, #153)

This first slice implements deliberate entry, orientation, checking the next
seat, a candidate attempt, the actual Easy rule answer, real observation and
return to free play. It does not complete #131's later phases, other contracts,
training, persistent history or release. Learner UAT and independent transfer
remain unmeasured.

## Rule and evidence definition

`guided-under-partner@1` and `guided-win-last@1` are bounded heuristic dialogues,
not new solver-approved mechanisms in the six-lesson catalogue. Their source
is the existing `BaselineOpponent::choose_card` on main
`ecfb2fe1ff5a50ab2fb5f17a892dd9e5f68c6c80`, now shared in
`skat-analysis::baseline`. Easy's original legal order, min-first/max-last ties,
Kontra/Re precedence and fallback are preserved. The pre-extraction regression
hash over 200 complete legal Easy games is `12969225982751591822` (FNV-1a over
Debug action bytes, seed order 0..200, Kontra/Re enabled). This freezes behavior;
it is not a strategic oracle. Explicit candidate expectations below provide
separate factual oracles.

- **UnderPartner:** Farbspiel, defender, partner currently leads, one player
  follows, and the leading card is not a master of its follow class. With at
  least two legal cards and one losing candidate, Easy takes a loser by
  `(is_trump, Augen, power)`, first exact tie. A nonchosen losing candidate may
  serve the same immediate purpose. This does not secure the partner's trick.
- **WinLast:** Farbspiel, last seat, opposing party leads, at least two legal
  cards and a winning candidate. Easy takes a winner by `(is_trump, power)`,
  first exact tie. Other winning cards still win this trick. No claim about
  their future strategic value or the final game result follows.
- **Exclusions:** no lead, early winner, partner feeding, arbitrary fallback,
  single legal action, Grand, Null or Kontra/Re (Reizen, Skataufnahme, Drücken, Ansage and Kontra/Re: see below). If the
  partner branch has no losing card, its original lowest-card fallback is
  explicitly marked `Fallback` and excluded, rather than explained as staying
  under the partner. In
  particular, `is_master` excludes only a higher card in the same follow
  class; it does not exclude trumping. These branches report unsupported.

All predicates use `Knowledge`, `TrickState`, the public trick, own permitted
hand and legal actions. Provenance is `Source::Trick`, own hand/known Skat and
the announced game's rules. Outstanding/unseen is not a location. Memory uses
M6 `CountingAid` definitions and initially stays hidden. Optional model
comparisons have their own original-decision identity, status, budget, basis,
objective and uncertainty; agreement cannot certify a rule and disagreement
cannot rewrite it. Rule answers work even without an analysis result.

## Core-validated fixtures

The explicit deal in `crates/skat-coach/tests/support/guided.rs` is:

| Place | Cards (C=Kreuz, S=Pik, H=Herz, D=Karo) |
|-------|---------------------------------------|
| Vorhand | CK CJ C9 H7 H8 H9 HT HJ HQ HK |
| Mittelhand | C7 C8 CT S7 S8 S9 ST SJ SQ SK |
| Hinterhand | CA CQ SA HA D7 D8 D9 DT DJ DQ |
| Skat | DK DA |

Hinterhand declares Herz Hand at 18 through core-accepted bidding.
As defender in Mittelhand after CK, C7/C8 leave CK ahead and CT overtakes;
Easy selects C7 by lower power at equal zero Augen. CA can move from the
hidden declarer to the unknown Skat by swapping DK without changing the
permitted input, prompts or rule. A second legal family puts CA and CQ outside
Hinterhand so that D7 may be discarded; swapping CQ with CJ and moving CA
away also permits the Kreuz-Bube to trump. Actual public play proves a Kreuz
void in both cases, while following CA proves only the card played.

As declarer in Hinterhand after C9/C8, CQ and CA both win; the cheaper winner
is CQ. All legal alternatives replay through the core. Moving CA into the
defender's own hand makes CK a follow-class master and selects excluded
`FeedPartner`; this is not narrated as safe against Trumpf. Forced last-trick
moves and Kontra/Re are unsupported. Hand has unknown Skat even for declarer;
known own discards and Ouvert use the same existing `Knowledge` definitions.

## Reproducible ordinary-app walkthrough

Start the ordinary app with series seed 21 and two Easy opponents, leave coach
on request, pass at bidding and play the first legal card until a supported
Farbspiel decision. Enter **Think through this decision together** (German:
**Entscheidung gemeinsam durchdenken**). Unsupported branches say so; return
and continue playing. `gui-egui/tests/support/guided.rs` performs these exact
moves through real widgets, without injected sessions or mock views.

1. Identify the current winning seat, then the player who still follows (or
   nobody). A wrong answer stays on its prompt, with retry and source feedback.
2. Consider a candidate without playing it. A correct candidate or deliberate
   **reveal** / **I don't know** exposes the actual Easy move and its conditional
   ranking. Optional memory opens only now. The table's cards remain playable.
3. Play any legal card, including another choice. Wait for the actual remaining
   player. Answer whether that card failed to follow the led class, or reveal.
   The result cites the real seat/card/trick and completed winner.
4. Return to free play at any time. With figures enabled, a separate request
   after observation may expose an available model comparison of the original
   decision. Missing estimates remain unavailable, provisional work stays
   provisional, and the rule answer remains unchanged.

Switch language in settings for the same English flow. Headless widget tests
exercise wrong/retry, reveal, memory, card choice, real observation and return
in both languages. App tests additionally exercise frozen input, different
hidden deals, stale/re-entered identities, default/off/automatic settings and
incomplete optional estimates. These are automated technical checks, not
observed learner clarity or interactive Windows 11 evidence.

## Reizen threshold decision (#148)

`guided-reizen@1` is a bounded heuristic dialogue over Easy's bidding rule,
not a solver statement. `skat_analysis::policy::bid_rule` returns the actual
action plus its inputs; `BaselineOpponent` calls the same function, so the
frozen 200-game hash `12969225982751591822` is unchanged, and a 500-game test
with random legal calls checks equality with Easy, no fallback and
`stake <= ceiling` ⇔ action ≠ Pass. Steps: orientation (public current bid;
hold or name a bid?), threshold (is the stake within the ceiling?), then the
worked answer after a correct attempt or reveal. Wrong answers stay with
retry; skip/return and stale identities follow the card dialogue.

Fixtures in `crates/skat-coach/tests/guided_reizen.rs` replay only
core-accepted calls on the deal above. Vorhand's ceiling is Herz 20 (mit 1),
Mittelhand's Pik 22 (ohne 1) and Hinterhand's Karo 36 (ohne 3):

| Learner decision | Stake | Easy action |
|------------------|-------|-------------|
| MH opens at 0 | 18 | Bid 18 (below) |
| MH after VH held 20 | 22 | Bid 22 (at) |
| MH after a forced Hold at 22 | 23 | Pass (above) |
| VH answers 18 / 20 / 22 | same | Hold / Hold / Pass |
| VH after MH jumps to 35 | 35 | Pass (never labelled 22) |
| HH after the jump to 35 and VH pass | 36 | Bid 36 (at) |

Further deals name a `Weak` hand (ceiling 0, Pass at 18), a `Null` hand
(23: Hold 23, Pass 24; Vorhand alone names 18), a `Grand` hand (mit 4: 120)
and the unavailable next value at 264 (only Pass legal). Moving Mittelhand's
Pik-Bube into the hidden Skat or swapping hidden cards between opponents
leaves Hinterhand's view and rule unchanged: the ceiling counts Spitzen from
the own ten cards only. In the ordinary app with seed 21, the human's first
decision is to answer 18 with a `Weak` hand; the threshold answer is “no” and
Easy passes. The learner may still hold.

## Skataufnahme and Drücken (#131 phase 4)

`guided-preplay@1` applies the same steps to Easy's Hand-or-pickup and discard
rules: orientation asks the candidate branch (`Grand`, `Suit`, `Null`,
`Weak`), the threshold asks whether the strength reaches 100 (pickup) or for
one card Easy puts away (Drücken; ties with Easy's second card count). The
worked answer follows a correct attempt or reveal. `pickup_rule` and
`discard_rule` are the exact paths `BaselineOpponent` uses; the frozen
200-game hash is unchanged and a 300-game test checks equality with Easy, no
fallback, `strength >= threshold` ⇔ Hand and the discard within the tie set.

Fixtures in `crates/skat-coach/tests/guided_preplay.rs` replay core-accepted
actions with Hinterhand declarer at 18; expected strengths are counted by hand:

| Hinterhand's cards | Decision | Easy |
|--------------------|----------|------|
| CJ HA HT HK HQ H9 H8 H7 CA SA | Herz, strength 106 | Hand |
| same with S7 for SA (changed condition) | Herz, 96 | pickup |
| CJ SJ HJ DJ CA CT CK SA ST HA | Grand, 102 | Hand |
| CJ SJ HJ DJ HA HT HK HQ H9 CA | Grand, 86 | pickup; Drücken S7 D7 |
| CJ HA HT HK HQ H9 H8 CA CT S7 + ST D8 | Herz, 94 | pickup; Drücken D8 ST, keeps Trumpf, CA, guarded CT |
| C7 C8 C9 S7 S8 S9 H7 H8 H9 D7 + DA HK | Null, 0 | pickup; Drücken DA HK |

A different hidden Skat leaves the Hand decision unchanged. A view whose
wanted action is unavailable discloses `fallback`. In the ordinary app with
seed 21 the human reaches both decisions by holding and bidding.

## Ansage (#151)

`guided-announce@1` uses the same steps for Easy's announcement: orientation
asks the branch of the preferred game, the threshold asks whether its plain
value reaches the public bid (`Answer::Within`). `announce_rule` replaces the
inline `announce` body; `announce` and `BaselineOpponent` call it. The frozen
200-game hash is unchanged, and a second hash frozen on `16c0414` before the
extraction covers 1988 Ansage decisions reached with random legal bids,
pickups and discards (`5977450191475415708`). Over the same views a test
checks the rule against Easy: preferred if it fits, else the strongest fitting
candidate with the last-in-order tie, else Grand; ouvert exactly for Null
without an unsafe suit; no fallback. Null rarely occurs in dealt games and is
covered by fixtures. No baseline inconsistency was found; the existing quirks
stay as they are: the strength behind the preferred game counts the two
discarded cards, and the Null ouvert check uses only the ten cards.

Fixtures in `crates/skat-coach/tests/guided_announce.rs` replay core-accepted
actions with Hinterhand declarer; values are worked out by hand:

| Hinterhand's cards, Skat handling | Bid | Easy |
|-----------------------------------|-----|------|
| CJ HA HT HK HQ H9 H8 H7 CA SA, Hand | 18 | Herz: Hand mit 1 = 30 fits |
| same | 33 | Herz 30 short; Kreuz 36 and Pik 33 tie at strength 61, Grand 54: Pik |
| same | 36 | Kreuz 61 over Grand 54 |
| same | 77 | no known-card plain value fits: Grand; overbid stays open |
| HJ for CJ (changed hand), Hand | 33 | Herz ohne 2 Hand = 40 fits |
| CJ SJ HJ DJ HA HT HK HQ H9 CA, pickup S7 D7, both put away | 18 | Grand mit 4 = 120, known Skat counted |
| C7 C8 C9 S7 S8 S9 H7 H8 H9 D7, Hand | 18 | Null Hand 35, no unsafe suit: Null ouvert |
| same, pickup DA HK, put away HK C9 | 18 | Null 23, Karo unsafe: plain Null |
| same | 24 | Null short; Herz (strength 45) over Kreuz and Pik (37) |

Different hidden Skat (Hand) or swapped opponent cards leave the lesson
identical. The no-fit answer therefore claims no overbid: with hidden Skat
SJ DQ at bid 77 the lesson is the same, yet the core settles Grand Hand mit 2
at 61 Augen as 96, not überreizt; earned levels can raise the value as well. Restricted views disclose the plain-Null fallback and the
first-legal fallback separately; a no-fit Grand is a supported rule fallback,
whereas Spiel has no Ansage rule. App tests cover wrong/retry, reveal, a correct
attempt, skip/re-entry, every legal announcement, stale identities, automatic
levels, coach off and the next game; the app views of different hidden deals
are identical. In the ordinary app with seed 7 the human bids to 36 and plays
Hand: the branch is Suit (Herz), Herz Hand 30 falls short, Kreuz 36 and Grand
72 fit, and Easy announces Kreuz Hand; the learner may still announce Grand.

## Kontra and immediate Re (#153)

`guided-kontra@1` covers two core-offered decisions in a non-Ouvert
Farbspiel: a defender's card turn with `Kontra` offered (declining means
playing a card) and the declarer's immediate `Re`/`KeinRe` reply. The source
is `skat_analysis::baseline::kontra_rule`, which `baseline::play` (Easy) now
calls: call and role, contract, public level, Trumpf count (Kontra) or strength
of the own hand plus the known own Skat (Re; none in a Hand game), threshold
(5 Trumpf, never in Null; strength 90), the exact card rule Easy plays when it
declines, the legal actions and the fallback flag. Order, thresholds and
`policy::wants_kontra`/`wants_re` are unchanged; the Ouvert round still
returns `KeinKontra` first and has no rule. Before the extraction a hash over
3605 Kontra, Re and Ouvert `KeinKontra` decisions reached with random legal
actions was frozen on `4d55973` (`13667852288273955558`); it and the 200-game
hash are unchanged, and a test checks every such view against Easy.

Steps: the public contract and level; orientation asks the factor on the game
points after the call (Kontra ×2, Re ×4, `Answer::Factor`); then, with factor
and threshold named, whether the own count reaches Easy's condition
(`Answer::Within`). A correct attempt or deliberate reveal shows Easy's action,
the counted Skat (Re), the comparison, Easy's declining card without
explaining its tactic, and two facts: Kontra/Re multiply the written game
points and do not change whether the bid is reached (Re: the strength count),
and the condition is a rule of thumb, not proof of a winning defence or a
probability; the outcome stays unknown. Memory stays unavailable, estimates
stay separately requested after the observation. The learner's own action
fixes the observed transition: after Kontra the declarer's actual Re/KeinRe
and the resulting factor; after Re/KeinRe the actual next card and factor;
after a declining card that card and trick (Kontra is no longer offered after
the own first card). Automated turns are followed; the learner's next own
action, a new game, coach off or replay ends the dialogue.

Grand, Null, an Ouvert contract, the Ouvert round before the reveal (no
declarer card is shown) and restricted or inconsistent views (absent contract,
seat or level not matching the call, Easy's choice not legal) report their own
unsupported text; all calls and cards stay free.

Fixtures in `crates/skat-coach/tests/guided_kontra.rs` replay core-accepted
actions: MH bids 18, VH holds, MH passes, HH bids 20, VH passes; HH declares.

| Position | Count, threshold | Easy |
|----------|------------------|------|
| VH CJ HA H7 H8 H9 SA ST S9 DA D7 vs Herz Hand | 5 Trumpf ≥ 5 | Kontra |
| same with S8 for H9 | 4 < 5 | DA (`LeadMaster`, last of SA/DA) |
| VH CJ SJ C7 C8 C9 SA ST S9 DA D7 vs Kreuz / Herz | 5 / 2 | Kontra / card |
| HH CJ SJ HJ HA HT H9 SA CT D7 D8, Herz Hand, after VH's Kontra | strength 90 ≥ 90 | Re |
| same with C7 for CT | 89 < 90 | KeinRe |
| HH pickup, holds the 89 hand, put away CT D9 / C8 D9 | 90 / 89 with the known Skat | Re / KeinRe |

Swapping unseen cards (opponents, unknown Hand Skat) leaves lesson and app
dialogue identical. Further legal positions: ineligible bidding (MH passes at
once, VH passes: no offer), the setting off, VH after its first card (no
offer, while MH may still say Kontra), MH after Kontra and Re/KeinRe, Grand
and Null (offer with exclusion, also for Re), Null ouvert round and the
following Re (exclusions). Labelled synthetic views test restricted legal
lists, an absent contract and a declarer offered Kontra. App tests cover
wrong/retry/reveal, every legal action with the actual observation, the
exact declining card, Re/KeinRe with the resumed play, stale and replaced
identities at the same seat and phase, automatic levels, coach off and the
next game. In the ordinary app with Kontra/Re on, seed 18 brings the human
(holding/bidding 18 only) to a Pik defence with five Trumpf where Easy says
Kontra, seed 0 one where Easy declines; with seed 1 the human declares the
suit it holds fewest of and answers a Kontra (Easy: no Re, strength 47); seed
8 reaches a Grand Kontra offer with the exclusion text. Easy's Kontra/Re
choices are reproduced, not improved, and no human learning follows from them.

## Verification and UI evidence

Use the existing `scripts/check.sh push --base origin/main` in `skat-dev`, plus
relevant ignored app/GUI integration suites. The existing screenshots example
adds `guided-orient`, `guided-act`, `guided-memory`, `guided-observe`,
`guided-complete`, English orientation/action/observation/completion, separately requested
`guided-figures` / `guided-figures-en`, and
`guided-orient-narrow`, and the Kontra/Re scenarios listed in
[testing](testing.md#screenshots-of-the-frontend). Each reaches the real app by widget clicks. Rendered
PNG hashes and individual visual inspections belong to the PR evidence.
