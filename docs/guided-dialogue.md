# Guided card play, Reizen, Skataufnahme, Drücken, Ansage and Kontra/Re pilot (#131, #148, #151, #153, #155, #157, #159, #161, #163, #165, #169, #171, #176, #178, #185)

This first slice implements deliberate entry, orientation, checking the next
seat, a candidate attempt, the actual Easy rule answer, real observation and
return to free play. It does not complete #131's later phases, other contracts,
training, persistent history or release. Learner UAT and independent transfer
remain unmeasured.

## Rule and evidence definition

The card-play scope includes `UnderPartner`, `WinLast`, non-Ouvert Grand `WinEarly` and
[non-Ouvert Farbspiel `WinEarly`](#farbspiel-winearly-178) on the human's own current second turn,
[early Grand `FeedPartner`](#grand-second-seat-feedpartner-131), last-seat Grand and [Farbspiel `FeedPartner`](#farbspiel-last-seat-feedpartner-185), [Grand `LeadMaster`](#grand-lead-with-a-follow-class-master-171),
and Null declarer play [last](#null-declarer-last-to-the-current-trick-165)
or [second](#null-declarer-acting-second-169), the [opening lead](#null-opening-lead-176), and a [defender following the declarer](#null-defender-following-the-declarer-131). These are bounded heuristic
dialogues, not new solver-approved mechanisms in the six-lesson catalogue.
The original `UnderPartner` and `WinLast` source was the existing `BaselineOpponent::choose_card` on main
`ecfb2fe1ff5a50ab2fb5f17a892dd9e5f68c6c80`, now shared in
`skat-analysis::baseline`. Easy's original legal order, min-first/max-last ties,
Kontra/Re precedence and fallback are preserved. The pre-extraction regression
hash over 200 complete legal Easy games is `12969225982751591822` (FNV-1a over
Debug action bytes, seed order 0..200, Kontra/Re enabled). This freezes behavior;
it is not a strategic oracle. Explicit candidate expectations below provide
separate factual oracles.

- **UnderPartner:** Farbspiel, Grand (#159) or the bounded Grand Ouvert
  defender view ([#315](#grand-ouvert-defender-under-partner-315)), defender,
  partner currently leads, one player
  follows, and the leading card is not a master of its follow class. With at
  least two legal cards and one losing candidate, Easy takes a loser by
  `(is_trump, Augen, power)`, first exact tie. A nonchosen losing candidate may
  serve the same immediate purpose. This does not secure the partner's trick.
- **WinLast:** Farbspiel or Grand (#157, below), including [Ouvert declarer last](#farbspiel-ouvert-declarer-playing-last-318), last seat,
  opposing party leads, at least two legal cards and a winning candidate. Easy takes a winner by `(is_trump, power)`,
  first exact tie. Other winning cards still win this trick. No claim about
  their future strategic value or the final game result follows.
- **WinEarly:** non-Ouvert Grand (#161, below) or
  [non-Ouvert Farbspiel](#farbspiel-winearly-178), opposing party ahead,
  one remaining player, at least two legal cards and current winners. The
  selected cheapest winner itself is Trumpf or a conservative follow-class master;
  another own master does not qualify it. Farbspiel requires the human's own
  current second turn, exactly one contiguous public card and the correct leader,
  at least two pure actions in exact core-legal order and consistent Hand or
  pickup information. In Farbspiel, all four Buben and the announced suit are
  Trumpf. A side master can be trumped; a lower Trumpf can be overtrumped.
  Unknown holders stay unknown.
- **FeedPartner:** non-Ouvert Grand (#163, below) or
  [Farbspiel](#farbspiel-last-seat-feedpartner-185) (#185), defender last to
  the current trick, partner ahead, at least two pure legal card actions.
- **LeadMaster:** non-Ouvert Grand (#171) and [Farbspiel](#farbspiel-lead-with-a-follow-class-master-182) (#182), own empty-trick
  lead, both roles where the actual shared branch permits it, at least two
  pure core-legal cards; no guarantee against trumping.
- **Null declarer:** Null acting [last](#null-declarer-last-to-the-current-trick-165)
  or [second](#null-declarer-acting-second-169), with a current loser; the
  [opening lead](#null-opening-lead-176) also supports Ouvert. Ouvert turns
  require the matching revealed hand. All require at least two pure legal cards;
  the linked sections define the full guards.
- **Null defender:** opening lead in Vorhand, including Ouvert after reveal; non-Ouvert second after the declarer leads, or last
  with the declarer ahead, with at least two exact legal actions and a current loser.
- **Exclusions:** other leads, Farbspiel WinEarly outside the [#178 guards](#farbspiel-winearly-178),
  early feeding or Farbspiel feeding outside the #185 guards, arbitrary fallback,
  single legal action, other Grand branches, other Grand Ouvert branches, other Null
  card play or card play when Kontra/Re takes precedence (Reizen, Skataufnahme, Drücken,
  Ansage and Kontra/Re: see below). If the
  partner branch has no losing card, its original lowest-card fallback is
  explicitly marked `Fallback` and excluded, rather than explained as staying
  under the partner. In
  particular, `is_master` excludes only a higher card in the same follow
  class; it does not exclude trumping. These branches report unsupported.

Unsupported card-play turns identify the public contract and whether the
player leads, plays second or plays last. A revealed Ouvert hand is named only
when present in the permitted table view. No rule or preferred card is exposed;
return leaves every legal card available. Kontra/Re retains its own fallback.
This descriptive fallback does not extend the rule-backed lesson scope or imply
learner efficacy.

All predicates use `Knowledge`, `TrickState`, the public trick, own permitted
hand and legal actions. Provenance is `Source::Trick` or an empty-trick `Source::Lead`, own hand/known Skat and
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

## Grand WinLast card play (#157)

`guided-win-last@1` also covers non-Ouvert Grand when the shared rule selects
`WinLast`: last seat, the opposing party (declarer for a defender) ahead, at
least two legal cards and a winning candidate. Prompts, checks, candidates,
memory and observation are unchanged; Easy's `choose_card`, branch order,
ties and the frozen hashes are untouched. In Grand only the four Buben are
Trumpf: a led Bube must be followed with a Bube, and a Bube never follows its
printed suit. A partner ahead in last seat selects the earlier `FeedPartner`
branch, now supported in last-seat Grand (#163 below; `UnderPartner` before
the last seat: see #159 below). Grand `LeadMaster` is supported within its
[own scope](#grand-lead-with-a-follow-class-master-171), as is Null declarer
play [last](#null-declarer-last-to-the-current-trick-165) or
[second](#null-declarer-acting-second-169). Non-Ouvert Farbspiel `WinEarly`
follows the [#178 guards](#farbspiel-winearly-178). Grand declarer
[`DrawTrumpf`](#grand-declarer-drawtrumpf-lead-175) is supported. Unsupported remain early Farbspiel `FeedPartner`, Farbspiel `FeedPartner` outside the
[#185 guards](#farbspiel-last-seat-feedpartner-185), fallback, single legal moves
and Ouvert Grand. Older Farbspiel families, including Ouvert, keep their
shipped scope; #178 does not add Ouvert WinEarly. A core-offered Kontra
keeps its own dialogue first. In last seat the learner's own card completes
the trick: the observation uses that card and the actual core `TrickWon`,
also for a losing alternative, without waiting for another seat.

Fixtures in `crates/skat-coach/tests/guided_grand.rs` replay core-accepted
actions on the deal above (declarer bids 18 or holds, the others pass, Hand).
Expectations follow by hand from the Grand order (side suits A T K Q 9 8 7)
and Easy's documented `(is_trump, power)` first minimum:

| Position | Legal | Winners | Easy |
|----------|-------|---------|------|
| HH Grand Hand, VH C9, MH C8 | CQ CA (DJ is Trumpf) | CQ CA | CQ |
| same, VH CK | CQ CA | CA (CQ loses) | CA |
| VH Grand Hand, VH C9, MH C7, HH defends | CQ CA | CQ CA | CQ |
| same, VH CK | CQ CA | CA | CA |
| HH also holds SJ (MH D7), VH HJ, MH C7 | SJ DJ only | SJ | SJ |
| HH holds CJ instead of D7, VH C9, MH C8 | CQ CA, not CJ | CQ CA | CQ |
| as above without plain Kreuz in HH | all ten | CJ DJ | DJ |

Swapping VH's unplayed H7 with the unknown Hand Skat's DK leaves HH's view,
lesson, memory and observations identical. `WinLast` fixture exclusions: partner ahead (MH CT
over the declarer's C9: `FeedPartner`), the declarer's lead (`DrawTrumpf`, a separate
[#175 lesson](#grand-declarer-drawtrumpf-lead-175)),
not last (MH: `Fallback`), no winner (HH H8/HA under
MH's SJ), a single winning CQ, Null Hand, Grand Ouvert and a Kontra-eligible
defender. App tests drive both roles through every legal card, wrong/retry,
correct non-Easy attempts, reveal, hidden swap and God Mode parity,
Kontra precedence, unsupported Grand branches (a partner-ahead `Fallback`
without a losing card, Ouvert Grand), automatic/off levels, stale
identities and the next game.

Ordinary app (Kontra/Re off, two Easy opponents): with series seed 19 the
human passes and plays the first legal card; in the second game it defends
last in Hinterhand against Mittelhand's Grand (MH's C9 over VH's C7; C8/CT/CA
legal, CT/CA win, Easy CT), and a later decision of that game shows the honest
fallback. With seed 2 the human holds or bids up to 30, picks up, puts away
its first two cards, announces Grand and in the first game plays last over
Hinterhand's HK (HQ/HA legal, only HA wins). The ignored `guided` widget test
drives both paths in German and English through real widgets only, plays the
losing C8 against Easy's CT and Easy's HA as declarer, and checks the actual
trick winner. The ordinary-app helper now names the target Grand branch, so
these seeds keep reaching `WinLast`. This is technical evidence, not learner
UAT.

## Grand UnderPartner card play (#159)

`guided-under-partner@1` also covers non-Ouvert Grand when the shared rule
selects `UnderPartner` (`baseline.rs` partner branch): a defender in the
middle seat, the partner ahead, the declarer still to act, a leading card that
is not a follow-class master, at least two legal cards and at least one losing
candidate. Easy takes the first minimum of `(is_trump, Augen, power)` among
the losers. The source is public: current trick, seats, announced Grand, own
cards, legal actions and frozen knowledge; an unseen card never names a
holder. Prompts, checks, memory and observation are unchanged; the rule text
states that staying under never secures the trick, because the declarer after
the learner may overtake or trump. Easy, branch order, ties and the frozen
hashes are untouched. The #159 increment excluded the partner's follow-class
master or the last seat (`FeedPartner`; a side master is no immunity to trumping), no losing card
(`Fallback`), a single legal card, the declarer role (a Grand declarer never
enters `UnderPartner`), leads, Farbspiel `WinEarly`, Null, Ouvert Grand, and a
core-offered Kontra, which keeps its own dialogue first. The observation waits
for the declarer's actual automated card and the actual core `TrickWon`.

Fixtures in `crates/skat-coach/tests/guided_grand.rs` use the deal above with
HH's Grand Hand and VH's lead; MH learns. Expectations follow by hand and are
replayed through the core, including every legal MH card and the actual HH
reply:

| Position | Legal | Losers | Easy | HH reply → winner |
|----------|-------|--------|------|-------------------|
| VH CK | C7 C8 CT | C7 C8 (CT overtakes) | C7 | after C7/C8: CA → HH, CQ → VH; after CT: CA → HH, CQ → MH |
| HH's DJ ↔ MH's C7, VH HJ | SJ DJ only | DJ (SJ overtakes) | DJ | D7 (no Trumpf, proves the void) → VH after DJ, MH after SJ |
| HH's CA/CQ ↔ VH's H7/H8, VH CK | C7 C8 CT | C7 C8 | C7 | after C7: DJ trumps → HH; H7 discards → VH; both prove a Kreuz void |

The third row leaves MH's permitted view identical to the first: staying under
does not make the partner safe. Changed condition: CA ↔ MH's C7 makes CK a
follow-class master from MH's view (all higher Kreuz own), so `FeedPartner`
applies and no lesson starts. Swapping HH's unplayed D8 with the unknown Hand
Skat's DK keeps MH's view, prompts, rule, candidates, memory and observations
identical. Further exclusions: partner ahead in last seat (`FeedPartner`), MH
with only CT/CK over VH's C9 (`Fallback`), MH's single C7, MH as declarer and a
Kontra-eligible MH. App tests drive wrong/retry, the correct non-Easy C8,
reveal, every legal card with the automated declarer's actual CA and the
completed trick, hidden swap and God Mode parity, Kontra precedence,
automatic/off levels, skip, stale re-entry, next game and estimates that
cannot rewrite the rule.

Ordinary app (Kontra/Re off, two Easy opponents, the human passes every bid
and otherwise plays its first legal card): a skat-app test searches seeds
0..128 with at most three games each and records each seed's first Grand
`UnderPartner` decision: seeds 97, 106 and 119, all in the third game. Seed 97
(VH's S7, all ten cards lose) is first; the walkthrough uses seed 119, the
first route with a losing and an overtaking card. There, in the sixth trick,
the human in Mittelhand defends against Hinterhand's Grand: partner VH leads
H8, H7 stays under, H9 overtakes, Easy H7. The ignored bilingual `guided`
widget test enters it by the real button, answers wrong and retries at each
step, attempts H7 (German) or reveals it (English), opens the memory, plays
the non-Easy H9 and observes the declarer's actual HQ, which wins. This is
technical evidence, not learner UAT.

## Grand WinEarly card play (#161)

Non-Ouvert Grand also supports the existing shared `WinEarly` branch in
Mittelhand: the opposing party leads, one player remains, at least two cards
are legal and there are current winners. Candidates are **all** current
winning legal cards. Easy's first minimum `(is_trump, power)` is unchanged.
Only the selected card must be Trumpf or a master of its follow class. The
worked answer explains that selected-card condition, never a condition on
every candidate. A side master can still be trumped; a Bube can be
overtrumped. In Grand only the four Buben are Trumpf, never following their
printed suit. Neither the trick nor strategic quality is guaranteed. Every
legal card, including a loser, stays free.

Orientation asks who currently wins, then who remains. Attempts at any current
winner, deliberate reveal and “I don't know” use the existing disclosure and
consent-gated memory lifecycle. Observation cites the actual third card and
completed winner. Early/automatic/off/unaided help does not reveal the rule.
No optional estimate selects the rule. The #161 increment added no Farbspiel WinEarly,
Null/Ouvert, early FeedPartner, DrawTrumpf, LeadMaster, Fallback or single-card lesson.
Existing supported branches keep their behaviour.

The unchanged handwritten oracle is in `tests/support/win_early.rs` under
`skat-coach`. Both MH-declarer and MH-defender roles are **fixtures**. Starting
with the complete deal above and VH's CK:

| Case | Change | Legal MH cards | Current winners; Easy | Actual reply and winner |
|------|--------|----------------|-----------------------|-------------------------|
| A | CA/C7 swapped | C8 CT CA | CT CA; CT | CQ: CT/CA win for MH, C8 for VH |
| B | A, C7/H8 and CQ/H7 swapped | C8 CT CA | CT CA; CT | DJ: HH wins, Kreuz void proven |
| C | Original, C7/D7 C8/D8 CT/D9 swapped | D7 D8 D9 S7 S8 S9 ST SJ SQ SK | SJ; SJ | CA after SJ: MH; C7 after D7: VH |
| D | C, HH's CA CQ C7 C8 CT swapped for VH's H7 H8 H9 HT HQ; HA/CJ swapped | same as C | SJ; SJ | CJ overtrumps SJ: HH, Kreuz void proven |
| E | Original | C7 C8 CT | CT; Easy C7 Fallback | CA after CT: HH; no lesson |

The core/coach tests preserve ten case/role combinations, 16 named completed
winners and 58 all-legal completion witnesses. A/B and C/D have identical
permitted input; extra unseen opponent/Skat substitutions preserve it too.
App tests add consent, attempts, all legal actions, actual replies, God Mode,
stale/re-entry and next-game boundaries. Frozen Easy checks remain unchanged.

### Frozen ordinary route

The permanent widget helper `enter_early` in `gui-egui/tests/support/guided.rs`
replays the independently frozen 43-action route: **series seed 0, fixed three
games, two Easy opponents, Kontra/Re off, closed display, Coach Off during
navigation**. Human is **MH declarer only**, Grand Hand, bid 27; actual game
seed `12896916468484187878`, game 3, trick 2. Language and coach changes use
Settings widgets, and announcements use Grand radio + Announce. The sole
harness timing change sets opponent delay to zero.

1. Game 1: hold 18,20,22,23,24,27,30; pass 33; play CT CJ CQ SJ SQ H7
   D7 D9 DK DJ; Next game.
2. Game 2: bid 24,27; Hand, Grand; play D9 H8 ST DQ SK C8 CK H9 HT DK;
   Next game.
3. Game 3: bid 18,20,22,23,24,27; Hand, Grand; after CA play C7. VH wins
   after HH's CQ, then leads CT. Enable Coach On request and enter.

Legal alternatives: S8 ST HK D7 D9 DT DJ DK DA. DJ is the sole current winner
and Easy card. The bilingual widget test exercises wrong/retry, winning-card
attempt, return/re-entry, deliberate reveal/don't-know, consent memory and
non-Easy S8, then HH's actual C8 and VH's actual win with CT, and return. Bilingual orientation,
action and observation renders also cover narrow action/observation at
960x640. No injected view/session/recording supplies this route. Defender roles,
changed conditions and the side-master explanation are fixture-only evidence.
These technical checks establish no human learning, transfer, UAT, persistence,
interactive Windows 11, packaged lifecycle or release acceptance. #131's
later-phase criterion remains open; terminal acceptance owns actual-main and
both-OS archive checks.

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

`guided-kontra@1` covers two core-offered decisions in a
Farbspiel (also Ouvert, #271), Grand (#155, also Ouvert) or Null (#155): a defender's card turn with `Kontra` offered (declining means
playing a card) and the declarer's immediate `Re`/`KeinRe` reply. The source
is `skat_analysis::baseline::kontra_rule`, which `baseline::play` (Easy) now
calls: call and role, contract, public level, Trumpf count (Kontra) or strength
of the own hand plus the known own Skat (Re; none in a Hand game), threshold
(5 Trumpf, never in Null; strength 90), the exact card rule Easy plays when it
declines, the legal actions and the fallback flag. Order, thresholds and
`policy::wants_kontra`/`wants_re` are unchanged; in the Ouvert round Easy
always selects `KeinKontra` without assessing the hand. Before the extraction a hash over
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

Ouvert Farbspiel, Grand (#271) and Null (#308) guide the defender's decision before the
reveal: Easy always chooses `KeinKontra` without assessing the hand. The hand
is hidden at that decision and becomes visible to defenders after the round.
The declarer's Re reply in Farbspiel, Grand and Null Ouvert is guided like any
Re; Null keeps its no-assessment explanation. Easy uses only the learner's own
cards, known Skat and public facts; swapping hidden cards leaves the dialogue
unchanged. Restricted or inconsistent views (absent contract,
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
offer, while MH may still say Kontra), MH after Kontra and Re/KeinRe, Null
ouvert round and the following Re (supported). Labelled synthetic views test restricted legal
lists, an absent contract and a declarer offered Kontra. App tests cover
wrong/retry/reveal, every legal action with the actual observation, the
exact declining card, Re/KeinRe with the resumed play, stale and replaced
identities at the same seat and phase, automatic levels, coach off and the
next game. In the ordinary app with Kontra/Re on, seed 18 brings the human
(holding/bidding 18 only) to a Pik defence with five Trumpf where Easy says
Kontra, seed 0 one where Easy declines; with seed 1 the human declares the
suit it holds fewest of and answers a Kontra (Easy: no Re, strength 47).
Easy's Kontra/Re choices are reproduced, not improved, and no human learning
follows from them.

### Grand and Null (#155)

The same rule, steps and observations apply; only the threshold question and
the worked answer depend on the contract. Grand: only the four Buben are
Trumpf, so the Kontra count can never reach Easy's 5 and Easy always declines
with its card; the prompt names the four-Buben limit and the caveat calls it
a limit of the rule of thumb, not a Skat rule. Grand Re compares the own hand
plus the known own Skat with 90 (Buben 14, Kreuz Bube 18, side Ace 10, side
Ten 6 with its Ace or two more cards, else 1; no empty-suit bonus in Grand).
Null: `policy::wants_kontra` excludes Null and `strength` returns 0, so Easy
never says Kontra or Re. The threshold step asks whether the generic rule
supplies an assessment of a Null game (`Answer::Applies`; only `false` is
accepted, `Within` is rejected), and no count, threshold or strength is
shown. Kontra and Re stay allowed in both; no claim about their strategic
quality, no probability and no outcome follows. Grand/Null card-play tactics,
including the tactic of Easy's declining card, remain outside.

| Position (counted by hand) | Count, threshold | Easy |
|----------------------------|------------------|------|
| VH CJ SJ HJ DJ SA ST S9 DA D7 D8 vs Grand Hand | 4 < 5 | DA (`LeadMaster`, last of SA/DA) |
| VH CA C7 C8 S7 S8 S9 H7 H8 H9 D7 vs Grand Hand | 0 < 5 | CA (`LeadMaster`) |
| VH C7 CA SA ST S9 H9 DA CJ SJ HJ vs Null Hand | rule does not apply | C7 (`Null`, lowest) |
| HH CJ DJ CA SA ST S7 HA HT DA DT, Grand Hand, after VH's Kontra | 32 + 10 + 16 + 16 + 16 = 90 ≥ 90 | Re |
| same with SJ for HA | 46 + 10 + 16 + 1 + 16 = 89 < 90 | KeinRe |
| HH pickup, holds CJ SJ CA CT HA HT H9 DA DQ S8 (74 alone), put away SA ST / HJ ST | 90 / 89 with the known Skat | Re / KeinRe |
| HH with the Grand 90 cards (Hand) or the pickup 90 cards, Null | rule does not apply | KeinRe |

Swapping unseen cards (opponents, the unknown Hand Skat with a Bube or Ace
that would count if it were known) leaves lesson and app dialogue identical.
Every core-offered call and card stays accepted, also Kontra in Null and Re
after Easy's KeinRe; labelled synthetic Grand/Null views with a restricted
legal list or a declarer offered Kontra fall back. App tests cover
wrong/retry (Null: "applies" and any count answer are wrong), every legal
action with the actual reply or declining card, Re and KeinRe with the
resumed play for Grand Hand/pickup and Null, the next-game reset with a stale
identity and God Mode parity. In the ordinary app with Kontra/Re on, seed 848
(first game) reaches a Grand defence with two Buben where the human says
Kontra and the declarer's actual reply is shown, seed 1282 (first game) a
Null defence where the human declines with a card; seed 848's second game
reaches the Null ouvert round with the same pre-reveal dialogue. No automatic opponent
says Kontra in a non-Ouvert Grand or Null, so the learner's Grand/Null Re
reply is reached only by the core-replayed app fixtures above, not in an
ordinary-app walkthrough or screenshot.

## Verification and UI evidence

Use the existing `scripts/check.sh push --base origin/main` in `skat-dev`, plus
relevant ignored app/GUI integration suites. The existing screenshots example
adds `guided-orient`, `guided-act`, `guided-memory`, `guided-observe`,
`guided-complete`, English orientation/action/observation/completion, separately requested
`guided-figures` / `guided-figures-en`, and
`guided-orient-narrow`, and the Kontra/Re scenarios listed in
[testing](testing.md#screenshots-of-the-frontend). Each reaches the real app by widget clicks. Rendered
PNG hashes and individual visual inspections belong to the PR evidence.

## Grand last-seat FeedPartner (#163)

`guided-feed-partner@1` is a bounded heuristic dialogue, not a new catalogue
mechanism. It applies only to non-Ouvert Grand, a defender last to the current
trick, partner currently ahead and at least two pure legal card actions.
Initial deal seat is irrelevant. Early FeedPartner (even a follow-class
master), Farbspiel feeding, Null, Ouvert, leads, one legal card and mixed
card/call actions remain excluded. Existing WinLast/WinEarly/UnderPartner
selection and the shared baseline/Easy source and frozen hashes are unchanged.

Every legal card is a candidate, including an overtake. Easy maximises
`(!is_trump, Augen)` in core legal order, retaining the **last exact tie**;
there is no power comparison. The public partner card identifies the current
winner, not a guarantee of the individual completed winner. Own cards, public
contract/trick/history and legal actions are the only sources. No hidden
holder, optimality, game result or later lead is promised. A follow-class
master can still be trumped in an early trick.

### Frozen independent fixture evidence

The original handwritten complete-deal oracle was frozen before baseline
execution on 2026-11-15 at 10:13:32 UTC. Its SHA-256 is
`f91cf881add14276ed16bc174dd51163ff8eba972cf6e73fe7db79b2ef8cf443`;
handwritten derivation
`dc810ee72786595aab6b102ed6952d917383f0e8954513cec80d3f58828018d2`;
separate correction/addendum
`e32bf9cbb7ddc49faf8020da882794bca9940d46231ae04409f5172e7cdfd29e`.
The [accepted preflight](https://github.com/naturelle137/grand-coach/issues/163#issuecomment-6893030950)
retains the original mistake: Kreuz legal actions omitted off-suit Buben.
Correct legal indices are [1,2,7,12,20], completed winners [1,1,2,2,2].
Ouvert replay sets mandatory Schneider/Schwarz flags. The separate eligible
Kontra history and complete early continuation trees stay negative boundaries.

`skat-coach/tests/support/feed_partner.rs` ports the handwritten complete
32-card deals and legal histories as **fixture-only** data, with the corrected
Kreuz expectations. `guided_feed_partner.rs` checks all 14 retained rows
(including the original Kreuz diagnostic and its corrected counterpart),
current/actual winners, every completion, early continuation trees and hidden
permutations before comparing policy. Indices are suit × 8 + rank, suits
Kreuz/Pik/Herz/Karo and ranks 7/8/9/10/Bube/Dame/König/Ass; seats 0/1/2.

| Fixture | Legal indices | Completed winners | Easy |
|---------|---------------|-------------------|------|
| Follow/overtake | 1,2,7 | 1,1,2 | 7 |
| Zero tie | 0,1,2 | 1,1,1 | 2 |
| Void/nontrump | 4,12,16,17,18,19,21,22,23,31 | 2,2,0,0,0,0,0,0,0,0 | 31 |
| Trump tie, no power | 4,20,28 | 2,0,0 | 28 |

Changing declarer ahead preserves WinLast; exchanging hidden Pik cards
preserves the entire permitted view and lesson. App fixture tests verify all
legal completions including a human overtake: the observation names HH, never
falsely the partner.

### Ordinary Session and real-widget evidence

Frozen route SHA-256:
`184aab000de96e8bd3c609679ded741d3142b6a91e109c7b74ba0c9a3f676c23`.
Start the ordinary app (`GrandCoachApp::new`) with seed 15, fixed three games,
two Easy opponents, closed display, Kontra/Re off, coach OnRequest and
figures/warnings off. Click Pass, Kreuz9, KreuzDame. In game 1, trick 3,
Player0 is the human defender with initial **Vorhand**, now acting **last**
after Player1 HerzBube and partner Player2 PikBube. Legal indices are
[8,10,11,13,14,17,18,25]; Easy chooses Pik10 (11), every completion leaves
Player2 winner. The source is the normal Session, never an injected view.

`enter_feed` and `feed_flow` drive actual widgets in both languages. They
cover orientation (partner HH), nobody remaining, legal candidate, rule answer
without playing, chosen Pik10 and alternative Pik7, actual void/individual
winner feedback and return. The widget test additionally checks wrong
orientation, public source feedback, retry and immediate return/free play.
Session tests check every legal completion and stale response rejection.
The screenshot generator includes `guided-grand-feed-{orient,candidate,rule,
observed,return}-{de,en}` and each `-narrow` variant, ordinary app at
1440×1000 and 960×640. Both OS CI screenshot jobs run all scenarios and hash
archives. Local PNG inspection and CI rendering are technical evidence only;
merged-main verification and protected integration remain terminal-owner work.
No learner UAT, transfer, training/persistence, interactive Windows 11,
packaged lifecycle, release or completion of #131 is claimed.

## Null declarer last to the current trick (#165)

`guided-null-last@1` is an explicitly entered heuristic dialogue, not a
catalogue mechanism. Eligibility requires Spiel, a non-Ouvert Null contract,
the own valid turn as declarer, exactly two contiguous public trick cards
with a consistent leader/acting seat, at least two pure legal card actions
and at least one current loser. Initial deal seat is irrelevant. Defenders
(including declarer ahead), leads/early seats, Ouvert, no-loser, single-card,
mixed/call-only and absent/inconsistent contract/turn/declarer views keep the
phase-specific unsupported fallback. The shared baseline is unchanged.

Null has no Trumpf; Buben follow their printed suit and rank between 10 and
Dame: 7, 8, 9, 10, Bube, Dame, König, Ass. `baseline::rule` supplies the exact
winner/loser sets and Easy's maximum Null power among losers, retaining the
**last exact tie in core legal order**. Power compares rank in each card's
own follow class; an off-suit Ass cannot win a led Kreuz. The dialogue states
this heuristic without an optimum, future lead, game guarantee or concealed
holder. The worked answer shows losing candidates and winning alternatives;
EVERY legal card stays playable. Before attempt/reveal, the preferred card,
sets, memory and estimates remain hidden. Wrong/retry, candidate attempts,
deliberate reveal and immediate free return reuse the existing state machine.
The Null panel scrolls below a permanently visible free-return control.

Last seat observes the human's actual completing card and individual winner,
including non-Easy moves and overtakes. Observation deductions remain hidden
until answer/reveal. Null void feedback names printed-suit following. Hand
memory leaves the Skat unknown; pickup keeps the own known discards. Figures
still require enabled figures and a separate request. Re-entry, reset,
changed assistance and subsequent decisions retain stale-response protection.

### Frozen evidence and discrepancy

Preimplementation acceptance is recorded in [#165](https://github.com/naturelle137/grand-coach/issues/165),
heading `Terminal preimplementation gate ACCEPTED — HQ1468`, on source
`316aa0e78a231aa26fad1260da38b584f5c10c69`, tree
`a51faefacf126092c6434176f778d402529f7409`. QA1458 froze its original at
2026-11-15 11:58:42.302220 UTC (event 10964), before baseline execution:
`6b3d308f271bf79dbf9be5f3f1fa29a522d507d58e673d9650c2b001a900238e`.
The separate **post-source** addendum remains distinct:
`f118dcac33807201d95c263e38c90e2366b6a12f28d14007291d10726024b4e9`.
Original synthetic `defender-declarer-ahead` metadata names all-loser-king,
but its harness actually cloned mixed-ten. Tests preserve the honest actual
mixed-ten case and separately replay the real defender boundary; the wrong
label is not silently promoted into an oracle.

`skat-coach/tests/support/null_last.rs` ports complete handwritten partitions
and legal histories. `guided_null_last` checks all 30 original completions,
the 10 changed-queen completions and four real-defender completions, legal
order, every candidate winner, Bube/10, mandatory following, void/tie and
changed maximum, Hand/pickup, real and synthetic exclusions. Three hidden
substitutions compare the ENTIRE permitted view and actual new Lesson:
opponent/unknown Hand Skat, opponent/opponent Hand, opponent/opponent pickup
with DK/DA retained. `None == None` is never privacy evidence. App `null_last`
tests exercise every admitted candidate, actual observations, consent,
wrong/retry, memory, return, re-entry, reset and stale identities.

### Ordinary route and fixture rendering

QA1458 `route.json` SHA-256:
`2d815cd0d3a306cb614d631f431697eadbcccac4d06732e47c0e0a1ce169ec58`.
Start `Session::new` / `GrandCoachApp::new`, seed6 / Fixed3 / two Easy,
closed display, Kontra/Re off, OnRequest, figures/warnings off. The exact
frozen action sequence is transcribed in `null_ordinary` and `enter_null`.
Game3/trick2: human initial MH acts CURRENT LAST after H8/HQ; legal order
H7/H9/HJ, all lose to VH; Easy HJ. Non-Easy H7 actually completes H8/HQ/H7
with winner VH. Bilingual widgets at 1440×1000 and 960×640 exercise every
legal card, retry, attempt/reveal, hidden memory, observation and free return.

Forty ordinary screenshot scenarios are
`guided-null-last-{orient,wrong,remaining,candidate,attempt,reveal,memory,
observe,complete,return}-{de,en}` and their `-narrow` variants. Thirty-two
explicit `guided-null-fixture-*` component views cover mixed alternatives,
actual overtaking, printed Bube, void/tie, changed queen and known pickup Skat
in both languages/sizes. They are rendered from actual app fixture outputs,
checked against committed JSON by `null_last_fixture_widget_models_match_checked_session_outputs`;
they NEVER count as ordinary gameplay. To regenerate those technical models,
set `SKAT_NULL_WIDGET_FIXTURES_OUT` to the absolute JSON destination and run
that exact app test. The handwritten oracle remains the domain reference.

Both OS CI screenshot jobs include these scenarios. Terminal #1477 owns the
one fresh review, at most one fix round, required CI, protected integration,
actual merged-main non-GPU regression including ignored tests and complete
both-OS archive/hash/visual acceptance. Local technical checks establish no
learner UAT, transfer, persistence, training, interactive Windows 11,
packaged lifecycle, release or completion of later #131 phases.

## Null declarer acting second (#169)

The #165 guard also admits the non-Ouvert Null declarer CURRENT SECOND,
after exactly one contiguous public card, on its own turn, with at least two
pure core-legal Play actions and one current loser. Physical Mittelhand is
not required. Lead, defender, Ouvert, no-loser fallback, one-card decisions,
mixed calls and restricted/inconsistent legal lists remain unsupported.
Hand Skat stays unknown; own pickup discards are permitted. Even at a God
Mode table, the rule dialogue uses the closed permitted view.

A current loser cannot win this trick for the declarer, regardless of the
last defender's card. That defender may change which defender wins. Easy's
unchanged maximum losing power, last exact legal-order tie, is a bounded
shedding heuristic, with no optimality or game-outcome guarantee. Null has
no Trumpf: printed-suit 7/8/9/10/Bube/Dame/König/Ass determines following
and rank. Every legal alternative remains playable without automatic play.

QA1591 sealed the independent oracle before policy inspection; cloud1593
accepted the gate. The original's changed C8 lead / C7 / C9 winner error is
preserved there and explicitly corrected to physical HH in the addendum,
in both completion representations. Cloud-authored
`skat-analysis/tests/null_middle_policy.rs` checks all 15 full legal deals,
the nine admitted states and 84 admitted continuations, exact full shared
rules, fallback and last ties. Failed local1592/1595 attempts remain
unaccepted; this is cloud completion in the combined feature, not accepted
local-model work. Whole non-absent lessons and full app dialogue models are
compared across three same-permitted-view hidden substitutions, including
unknown Hand Skat and known pickup discards.

The ordinary widget route uses Session/GrandCoachApp seed 1, Fixed(3), two
Easy opponents, closed display, Kontra off, OnRequest, figures/warnings off,
focus None. In game 1: Hold four times, Pass, then play indices
31/28/8/25/26/29/23/15/14/22, letting opponents act before each command.
NextGame; Bid20, Hand, announce Null; play 19/24/4 with the same automation.
Game 2, trick 4: physical MH leads Pik-Bube (12), physical HH acts second.
Legal 9/10/13/14 are Pik-8/Pik-9/Pik-Dame/Pik-König; losers 9/10,
Easy 10. Choose free alternative9: actual third Pik-Ass15, completed
12/9/15 won by physical VH. The new ignored widget tests use this exact
ordinary route; injected boundary components are explicitly labelled.

One question at a time covers winner, remaining seat, candidate, deliberate
reveal, actual third card and individual winner. Wrong/retry, worked answer,
I don't know, skip/return, collapsed memory, separate opt-in estimates and
original decision identity reuse the existing lifecycle. A changed decision,
reentry, replay, assistance change or reset invalidates old responses.
This supplies technical evidence only: independent PR review, final CI,
protected integration and actual-main/both-OS acceptance belong to the
terminal owner; human learning and release evidence remain pending.

## Grand lead with a follow-class master (#171)

Deliberate entry also admits a non-Ouvert Grand own turn at an empty trick,
with at least two pure core-legal Play actions and the actual shared
`LeadMaster` outcome. Both roles are supported where that branch applies;
fallback, single cards, mixed calls and restricted or
inconsistent views remain excluded; the declarer's `DrawTrumpf` lead was added in #175. No policy or Easy tie changes.

The learner identifies their public role/lead, then a legal side master,
then attempts Easy's largest-Augen choice. Master identification accepts every
shared master; the rule-choice question checks Easy's actual selected card,
including the last exact tie. Another master can still be played freely.
A correct rule-choice attempt or
explicit reveal shows the candidates, selected card and **last exact tie in
core legal order**. This is a bounded heuristic, not an optimum. Every legal
card remains freely playable, including a nonmaster. Wrong/retry, worked
answer, I don't know and permanent free return retain the existing lifecycle.
Before disclosure, no preferred card, candidates, memory or estimates appear.

The exact `Knowledge::is_master` predicate excludes own and played higher
cards, **not** known discarded higher cards. Thus a lower own rank can be a
master alongside a higher own rank; a known discarded higher card still
conservatively blocks it. Discarded Buben remain in the policy's
`trumps_out`, although M6 memory's outside count excludes the known Skat.
All Buben are Trumpf, including the Bube printed in the led suit. A void
follower may trump a side master; no unseen owner, future trick or game
winner is inferred. The empty-table shared `winners` list contains every
legal lead, not guaranteed future winners.

`Source::Lead` records the empty trick and actual entitled seat without a
fictional winning card. Both real reply cards remain visible in order; the
observation waits for trick completion and retains its individual winner
across later automation, including after a non-Easy choice. The last reply's
failure to follow is the bounded observation question. StartGuided uses the
permitted closed view even at a God Mode table. Whole non-absent lessons and
full dialog models are compared over two hidden substitutions. Estimates
remain separately requested, with the original decision identity and basis.

The independent gate accepted by HQ1633 preserves original1622 event12569,
SHA256 `9389ce395a66ab94597e087553669594e70f12dc225b182eee9c64affbf65984`,
and the separate post-source addendum
`572495f2434b477e2c39c4708e0334d1da158eac3f3e09451235e676565c6fcf`.
Original case10 is actually LeadMaster with four own Buben; separate case13
proves DrawTrumpf. Cloud-owned `lead_master_policy` ports literal complete
legal deals, all 3,324 next-trick and 106 terminal continuations. Local1639
remains rework; checker1640's cloud-corrected API inventory is navigation
only, with no accepted local Rust or feature-runtime credit.

The ordinary route is `Session::new` / `GrandCoachApp::new`, seed3,
Fixed(1), Easy/Easy, Closed, Kontra/Re off, OnRequest, figures/warnings off,
focus None. Let opponents act, choose Pass, let opponents complete pickup,
discard and Grand announcement. Game1/trick1: human player0, physical VH,
dealer player2; legal indices 5/6/8/13/14/22/23/27/28/29, Easy HA23.
Choose CQ5: actual CQ5/C7(0)/C10(3), winner physical HH/player2.
This route uses real commands/widgets without injection.

`lead-master-inventory.json` is derived from the same source selecting every
new screenshot: name, stage, kind, language, width, height and fixture case
where relevant. Ordinary stages cover orientation, wrong/retry, master,
candidate, attempt, reveal, memory, waiting, observation, completion, return
and details in de/en at 1440×1000 and 960×640. Labelled fixture components
show tie/own higher, pickup memory, rotated later lead, trumping, four own
Buben, no master, single legal card, DrawTrumpf and known-discard blocking.
Their committed models are checked against actual Session outputs, not
presented as ordinary routes or handwritten domain evidence. Existing
screenshot inventories remain intact.

One independent PR review, final required CI, protected integration and
actual-main/both-OS complete archives, hashes and personal view inspection
remain terminal1642's delivery work. No learner/UAT, transfer, persistence,
training, interactive Windows 11, packaging, release or later #131 completion
is established by these technical checks.

## Null opening lead (#176)

The Null opening-lead dialogue also admits a Null Ouvert declarer after the
own hand is revealed. It uses the same permitted-view Easy rule and preserves
free choice. The seeded application test checks every legal opening, both actual
replies, the winner and return to play. Bilingual ordinary-route renders cover
the explanation and observation; other Ouvert turns have their own scopes.

`guided::lesson` version 1 extends the explicit heuristic dialogue (not the
six-lesson mechanism catalogue) to Spiel, a Null declarer in physical
Vorhand, its own turn and the exactly empty first trick. It requires the exact
pure core-legal list, ten own cards, three ten-card hands and consistent Hand
(unknown Skat) or pickup (two known discarded cards). A Null declarer winning
any trick loses immediately; later own leads and positive MH/HH rotations are
unreachable. Real MH/HH declarer histories with VH to lead are exclusions.

Easy's shared `Branch::Null` is unchanged: minimum `(is_trump, Augen, power)`,
FIRST exact tie in core order. Null has no Trumpf; Buben follow printed suit,
and rank is 7,8,9,10,Bube,Dame,König,Ass. Power counts weaker cards (0..7).
Augen are a legacy sorting cost, neither Null scoring nor strategic value:
a zero-Augen 9 can precede another suit's positive-Augen 10. This is not
universally the lowest printed rank. All legal leads are current-table
`winners` only because the table is empty; neither actual reply, completed
winner nor game result follows. No unseen card gets an owner.

Orientation asks the own public role and lead. The next question accepts
every legal lead as a candidate; the attempt asks for Easy's ordering choice.
Wrong/retry, unknown, reveal, skip/free return, every legal card and collapsed
memory retain the existing dialogue transitions. There is one active prompt
and at most two supporting facts. Ordering detail stays collapsed. Free return
remains above the scrolling panel, including 960×640. Both actual defender
replies and the individual winner persist across automation and immediate
Null end. Figures require enabled figures plus a separate request after
observation, retaining the original decision; they never change the rule.

Accepted1695 evidence preserves original1694 SHA256
`8a608bf13b958750b859e45a5f13f02d29708d2db1c1622e0021ef10b0bccef7`,
freeze event 12996 (stream 23 before analysis 25), and the ONE separate
source-informed addendum SHA256
`e5f4cfaaf17ca0991cc73fc0a904f7defb8c86fc50afe2bc29d18562135dfcb7`.
The original's eight discrepancies remain: six internal powers use 0..7
rather than core 1..8; mixed returns None and a phase-only mutation Some.
This does not retroactively repair independence. No further correction.
Complete literals in `null_lead_policy.rs` retain all six deals/histories,
views, Knowledge, eleven rule fields, two hidden pairs and 286 legal next
tricks: VH 94 / MH 103 / HH 89; all 94 declarer wins immediately end Null as losses.
Named API mutations distinguish the broader shared rule from new admission.
Existing nonempty Null families remain supported.

Ordinary route: `Session::new`, seed 1, Fixed1, Easy/Easy, Closed, no Kontra;
OnRequest, no figures/warnings/focus. Game seed 7424550030962593201, dealer 2,
human VH, bid 24: SetCoach(default), AdvanceOpponent twice, then five
Hold/AdvanceOpponent pairs, Hand, announce Null. Easy Play 8 yields
[8,10,9], MH wins. Free Play 14 yields [14,12,13], VH wins and Null ends at -70.
The GUI replays these setup choices through ordinary widgets, without injection.

`guided-null-lead-*` images cover ordinary stages from entry through wrong,
retry, candidate/attempt, wrong attempt, reveal/unknown, memory, waiting,
observation, completion, actual Null end, free return and ordering details.
`guided-null-lead-fixture-*` are explicitly labelled core/app component
fixtures for Hand/pickup, printed Bube, Augen/rank distinction, actual Null
end and MH/HH exclusions. They are not ordinary routes or independent oracles.
The screenshot source emits `null-lead-inventory.json` with exact names,
stages, provenance, language and dimensions (de/en, 1440×1000 and 960×640),
and the existing Linux/Windows all-scenario pipeline includes this family.
Both entire OS archives, every hash, actual-main acceptance and personal
inspection on each OS remain terminal delivery checks. #131 later phases
remain OPEN; #175 is delivered in its own [section](#grand-declarer-drawtrumpf-lead-175). No learner/UAT/transfer, training,
persistence, interactive Windows 11, packaging or release evidence is claimed.

## Farbspiel WinEarly (#178)

The deliberate dialogue now admits non-Ouvert Farbspiel on the human's own
current second turn: exactly one contiguous public card, correct leader,
at least two pure actions in exact core-legal order and consistent Hand or
pickup information. Shared `baseline::rule` remains unchanged: partner-ahead
precedence, all current winners as candidates, minimum `(is_trump, power)`
and first exact tie. The selected cheapest winner itself must be Trumpf or
a conservative follow-class master; another own master does not qualify it.
All four Buben and the announced suit are Trumpf. A side master can still be
trumped; a lower Trumpf can be overtrumped. Unknown holders remain unknown.
New-family exclusions do not withdraw older supported families.

The existing stages retain one prompt, at most two supporting facts,
attempt/retry, unknown/reveal, skip, all legal choices and immediate return.
Memory starts collapsed; estimates need separate consent and enabled figures
and remain tied to the original decision. The observation retains the actual
third card and individual winner across automation, including a free choice.
The new family's complete model and basis label use the permitted player view
also on a God Mode table. Its long explanation scrolls while return remains
above the scroll area at the minimum supported size.

Accepted HQ1740 original SHA256
`ed1d45446b6ad287334e1c35aa2aa75d7f12346149ed434655c7378933c79511`
and sole HQ1751 supplement SHA256
`6f7051a935407985a4de4a286f666a811c5d135e7f29fe7a790f95c3a191e7ff`
remain separate. HQ1752 accepted the domain gate. The original 51 incorrect
expectations and eleven explicitly uncertain supplemental legacy predictions
remain evidence, not production defects or silently rewritten expectations.
The source fixture names which numeric/API values use disclosed calibration.
Twelve full legal histories cover all four announced suits, both roles,
physical Hinterhand acting second, known pickup Skat, Buben follow classes,
positive hidden pairs and a cheaper non-master/Fallback boundary. All 356
represented third-card completions are checked; no numeric settlement claim.
Selected side masters survive only 5/10, 3/10, 4/10 and 3/9 represented replies.

The ordinary widget route reuses the accepted seed0/Fixed3/EasyEasy/Closed
110-command prefix from HQ1740, stored as `suit-win-early-route.json`. Game3
uses core seed12896916468484187878, dealer1, Mittelhand declarer, pickup Karo.
Easy24 produces `[7,24,1]`, winner Mittelhand; free9 produces `[7,9,5]`, winner
Vorhand. No new route search or injected state is presented as ordinary.
`guided-suit-win-early-*` views use a source-derived inventory in de/en at
1440x1000 and 960x640. `fixture` names are labelled component views from the
checked full legal sessions, including trumping, overtrumping and Fallback.
Historical fixture tables and accepted images remain unchanged.

This is author technical evidence only. Independent review, final CI,
protected integration and merged-main/both-OS acceptance belong to the terminal
owner. Later #131 phases remain open; #175 is delivered separately. No learner
UAT, transfer, training, persistence, interactive Windows 11, packaging or
release acceptance follows.

## Farbspiel last-seat FeedPartner (#185)

Deliberate entry admits a non-Ouvert Farbspiel defender on its valid current
last turn, after exactly two contiguous public cards with a consistent leader,
at least two actions in the exact pure core-legal Play order and consistent
Hand/pickup information. The partner must currently lead and the shared rule
must actually return `FeedPartner` with `last=true`. Early feeding, declarer,
Ouvert, malformed/restricted/reordered inputs and single choices are excluded
from this new family. Missing declarer returns before the baseline call.
Existing families, including Grand, retain their support; a new-family false
result does not imply that every older lesson is absent.

Easy's unchanged shared policy considers EVERY legal card, maximizes
`(!is_trump, printed Augen)` and retains the LAST exact tie in core legal
order. There is no power tie-break. All four Buben and the announced suit
are Trumpf. A selected card can overtake the partner. The dialogue observes
the actual completing card and individual winner, without claiming an optimum,
a guaranteed partner trick, a game result or an unknown card's holder.

The public partner/last-seat role is available during orientation without the
preferred card, candidate set or rule. `task` itself stays absent in Orient
and Check. One prompt, at most two supporting facts, wrong/retry, any legal
candidate attempt, deliberate reveal/I don't know, collapsed memory and immediate
free return reuse the existing lifecycle. The panel scrolls below the return
control at 960x640. Nothing autoplays. Estimates need separately enabled figures
and a separate request, retain the original decision and frozen permitted input,
and cannot reuse privileged God Mode analysis. Full non-absent lessons and
complete app dialogue models match across BOTH positive hidden pairs, including
God Mode; the table's concealed hands and Skat never enter the dialogue.

HQ1827 accepted HQ1824's original SHA256
`abf899121b7de84ad4d53c8b5721ff7b9cb855c2e3082371bf79495e0892998f`,
freeze event13785, with zero corrections. Six full 32-card deals cover all four
suits, all three reachable defender seats, Hand/pickup, every candidate and all
37 represented own-card completions, plus four complete terminal paths.
Handwritten physical partitions, selected cards, ties and individual winners
remain distinct from core-serialized full views and post-freeze source-informed
Knowledge/CardRule numeric/API observations. The default-feature
`suit_feed_partner_policy` target ports those literals; missing-declarer's old
baseline panic is recorded as such, never a successful rejection. Cloud-authored
Rust gives no local-model coding credit. The rejected HQ1825 GUI proposal and
illustrative HQ1826 repair are not implementation evidence.

The accepted ordinary route starts Session/GrandCoachApp seed0, Fixed3,
Easy/Easy, Closed, no Kontra, OnRequest, figures/warnings off, focus None.
`suit-feed-partner-route.json` preserves all twelve commands: SetCoach,
AdvanceOpponent, Pass, four AdvanceOpponent, Play3, four AdvanceOpponent.
Game1/core seed13080132717333068652, dealer2: human physical Vorhand defender,
Mittelhand declarer, Herz pickup. At public [23,20], legal [4,12,16,28],
partner Hinterhand currently leads. Easy28 completes [23,20,28] with winner
Hinterhand; free4 completes [23,20,4] with winner Vorhand. All four legal
choices run through actual widgets. Advancing opponents is normal app automation.
No route search or injected play is presented as ordinary.

`guided-suit-feed-partner-*` screenshots and their source-derived
`suit-feed-partner-inventory.json` cover all new stages in de/en at 1440x1000
and 960x640. Names containing `fixture` are explicitly labelled component
views from checked core/app sessions, including all suits, ties, Trumpf,
late completions and reachable early/single exclusions. They are not ordinary
routes or new handwritten oracles. The existing all-scenario CI collection
includes this inventory and PNG family on both operating systems.

This is author technical evidence. Terminal HQ1829 owns the one fresh new-PR
review, at most one findings fix, final CI, protected integration and actual
new-main/both-OS archive and personal visual acceptance. #131 later phases
remain open; #182 remains finally deferred (#175 is delivered separately). No learner/UAT, transfer,
training, persistence, interactive Windows 11, packaging or release claim.

## Grand second-seat FeedPartner (#131)

The new `guided-grand-early-feed@1` rule dialogue supports non-Ouvert Grand,
acting defender CURRENT SECOND, partner leading, at least two pure Play actions
in exact core order, consistent Hand/pickup information and the actual shared
`FeedPartner` branch with `table_master=true`. Other new roles/contracts,
Ouvert, restricted/mixed/reordered actions, off-turn and single choices are
excluded; older families retain their admission. #182 remains finally
deferred, with no retry or implementation (#175 is delivered separately).

`Knowledge::is_master` excludes own and publicly played higher cards when
checking the follow class; it does not locate unknown cards or exclude trumping.
The current partner card may even be overtaken by a higher card in the own hand.
All legal cards remain candidates. The unchanged Easy rule maximises
`(!is_trump, Augen)`, retaining the LAST exact legal-order tie without power.
This is a heuristic, not an optimality or game-win claim. Only Grand's four
Buben are Trumpf. A side master can be trumped by the remaining declarer.

The original independent #163 full-deal witness
`B-early-master-can-be-trumped` remains unchanged in
`crates/skat-coach/tests/support/feed_partner.rs`: partner CA, defender CT/CQ,
then a Kreuz-void declarer. `guided_early_feed` checks both choices and all ten
legal replies per choice: CJ/SJ win for the declarer, the other eight leave
the partner ahead. New source-informed handwritten boundary cases cover a
Bube lead with an exact HJ/DJ tie and a CK lead with own CA/CT; these are not
retrospectively labelled independent #163 evidence. Full non-absent lesson,
memory and dialogue equality across hidden swaps, including God Mode and
original permitted-view estimates, are checked in coach/app tests.

### Ordinary walkthrough

Start ordinary `SessionConfig` seed **22**, fixed three games, Easy opponents,
Kontra/Re off and closed display. The complete human commands live in
`crates/gui-egui/tests/fixtures/early-feed-route.json` and are replayed through
real widgets, without an injected session. In game3/trick1 the human is physical
Mittelhand; partner Vorhand leads Pik-Ass15. Legal cards are Pik8/9/Dame
(indices9/10/13). Easy chooses Dame13. Any of these choices is followed by the
declarer's actual Karo-Bube28; Hinterhand wins. The dialogue observes that
actual third card and proves a void in plain Pik, without placing a hidden card
beforehand.

Answer the current-winner and remaining-seat questions, try a legal candidate
or deliberately reveal, inspect the worked rule and optional collapsed details,
then play any legal card. Wrong/retry, unknown/reveal, initially hidden memory,
separate requested figures and free return retain the shared controls. The
bilingual `early_feed` widget test exercises both Easy and free Pik8 at
1440×1000 and 960×640. Sixteen `guided-grand-early-feed-*` ordinary screenshots
show orientation, rule, observation question and feedback in both languages
and sizes. These are technical checks, not learner or transfer observations;
the wider #131 later-phase criterion remains open.

## Farbspiel lead with a follow-class master (#182)

Admission extends the Grand `LeadMaster` lesson (#171) to non-Ouvert Farbspiel:
the human's own legal empty-trick lead with a valid declarer, consistent
Hand/pickup information and at least two pure actions in exact core-legal
order, when the shared Easy branch is `LeadMaster`. Declarer `DrawTrumpf`
precedence is unchanged, so a declarer holding Trumpf while Trumpf are out gets
no lesson. Candidates are nontrump legal cards with a conservative
`Knowledge::is_master`; the announced suit and all Buben are Trumpf and never
candidates. Easy takes maximum Augen, LAST exact tie in core order. A master of
its follow class can still be trumped by a void opponent: no trick or game is
guaranteed. Ouvert, Null, other lead branches and restricted or malformed views
stay outside. Dialogue, texts (one Farbspiel definition key) and observation
reuse the Grand lead flow.

Evidence: `cargo test -p skat-analysis --test suit_lead_master_policy` holds
fresh handwritten legal deals for all four trump suits (own-higher, Trumpf-Ass
exclusion, maximum-Augen/last-tie, late lead after a won trick, void trumping,
two hidden-deal substitutions) and an independent oracle that re-derives
candidates, pick and the winner of all 1,803 legal next-trick completions
(counts/digests frozen, calibrated from the oracle). `cargo test -p skat-coach
--test guided_suit_lead_master` checks admission, every legal choice, boundary
edits and declarer role. App `suit_lead_master` tests run all completions and
the ordinary seed 2969 Pik route (human defends, all ten legal leads against
Easy) through `Session`, with full dialogue equality over both hidden-deal
pairs including God Mode. The ignored GUI target `guided_suit_lead_master` and
the screenshots example render the ordinary route in de/en at 1440x1000 and
960x640.

Not claimed: the original preimplementation gate of #182 was never accepted by
the CTO; this increment's oracle is author-written and source-informed, not an
accepted independent manual oracle. The 305 completions of the late case each continue to the end with first legal
cards, checking subsequent winners independently. Separately requested estimates
use the frozen permitted view, including in God Mode. Labelled God Mode fixture
frames render the actual session-derived dialogue panel, not a full table.
Independent terminal acceptance remains open. This is
technical evidence, not learner evidence.

## Grand declarer DrawTrumpf lead (#175)

Admission extends the Grand own empty-trick lead (#171) to the acting
non-Ouvert declarer when the actual shared Easy branch is `DrawTrumpf`: own
Trumpf (in Grand only the four Buben) and a nonempty conservative
`Knowledge::trumps_out`. It needs consistent Hand/pickup information and at
least two pure core-legal actions in exact order. Defenders, Farbspiel, Null,
Ouvert, nonempty tricks, single legal cards, no own Trumpf and malformed or
restricted views stay outside. Easy takes the highest-power own Trumpf (first
exact tie, which Buben power keys never produce). `trumps_out` excludes own and
played Buben but counts known discarded ones; it does not prove that a defender
holds Trumpf, and an unknown Skat stays a possible location. The highest Trumpf
is a heuristic, not optimal play, and no reply, trick or game result is implied.

The learner states role, names a legal Trumpf (candidates are the own Buben), then
tries Easy's card; every legal card stays playable, memory starts collapsed and
both actual replies plus the individual trick winner are observed. A view-model
flag `lead_trumpf` only selects the question type before disclosure.

Evidence (technical, author-run; no learner evidence):

- `crates/skat-analysis/tests/draw_trumpf_policy.rs` holds fourteen handwritten
  complete legal deals and histories reasoned from the ISkO and the admission
  text above, not copied from the earlier #171 fixtures: all three physical
  declarer seats (later leads for Mittelhand/Hinterhand after winning trick 1),
  Hand and pickup, higher and lower own Buben, remaining Buben in defenders
  versus a kept or discarded Skat Bube, the known-discard-only trigger, all four
  Buben own, outside Buben already played, no own Trumpf, a competing side
  master, a defender lead, and Ouvert and Farbspiel boundaries. It compares
  the full `PlayerView`-derived rule (all eleven `CardRule` fields) and
  `trumps_out`, and enumerates every legal next-trick continuation of the eight
  admitted cases (2,157 in total), checking each trick winner against an
  independent winner function written from the Grand rules. Each admitted case
  also plays out to the end with the first legal card and checks every winner.
  Counts describe only these represented cases, not universal coverage.
- `cargo test -p skat-coach --test guided_draw_trumpf` checks the coach
  admission on those cases (exactly eight admitted, candidates are exactly the
  own Buben, only Easy's card is the attempt), three same-permitted-view
  hidden-defender substitutions and malformed or single-card edits; it also
  keeps the older single-case checks.
- `skat-app` `guided_draw_trumpf_tests` drives a genuine `Session::new` route
  with explicit config and human commands (Easy opponents, Kontra/Re off:
  hold or bid up to 24, Hand, plain Grand, then the guided dialogue). The
  bounded search over seeds 0..32 (up to three games each) first hits seeds
  1, 3, 4 and 5, all at the first lead of game 1; seed 0 never reaches the
  lesson under that policy. On seed 1 the declarer holds only DJ, Easy leads it
  and both replies (SJ and HJ) show that it is not the best card; each of the
  ten legal leads was played with both actual replies and the individual
  trick winner checked, plus stale ids, free return and unchanged recordings.
  Whole non-absent dialogue privacy, including God Mode, compares the complete
  dialogue of two sessions with different concealed defender cards through
  every answer, the public continuation, each reply and the observation.
  Fixture sessions are labelled as such and are not ordinary routes.
- `gui-egui/tests/draw_trumpf.rs` and `examples/support/draw_trumpf_widgets.rs`
  drive the same seed-1 route through real widgets in both languages at
  1440×1000 and 960×640 (ten stages, forty `guided-draw-trumpf-*` screenshots),
  and every legal lead with immediate return. Boundary fixture frames of the
  declarer lesson live in `lead-master.json` (case 13).

The earlier independent-gate attempt (HQ1689/1691) stays recorded as failed and
partially frozen; this delivery supersedes it by the bounded CTO decision on the
issue and does not claim an independent pre-source domain gate. It claims no
learner, UAT, transfer, training, persistence, interactive Windows 11,
packaging or release evidence. #131 later phases remain open.

## Null defender following the declarer (#131)

A non-Ouvert Null defender acting second after the declarer's lead can
compare legal cards that leave the declarer ahead for now. Easy chooses the
highest Null rank among those cards, with the last exact tie in legal order;
the partner may still overtake. All legal cards remain playable. This is a
bounded heuristic, not a guarantee of winning or a claim about unseen cards.
This second-seat lesson excludes partner leads, absent losing cards, restricted actions,
Ouvert and other defender turns. It uses the public trick and own hand, and
observes the actual third card and individual winner.

For a reproducible ordinary route, use seed 2115 and a three-game series
against Easy. Pass and play the first legal card throughout the first two
games, then pass in game three. Vorhand's Kreuz-Sieben leads in Null; the
Mittelhand defender is void in Kreuz and Easy selects Karo-Ass as the last
highest-rank tie. Playing it is followed by Hinterhand's Kreuz-Bube, which
wins the trick. The widget test reproduces this route in both languages.

## Null defender playing last (#131)

A non-Ouvert Null defender playing last can enter the dialogue when the
declarer currently wins and a legal card leaves the declarer ahead. Easy
chooses the highest Null rank among those cards, last in legal order on a tie.
All legal cards remain playable; the observation reports the actual card and
winner, including if the learner overtakes. The lesson uses only the permitted
view and excludes a partner currently ahead, no losing card and Ouvert. This
heuristic does not guarantee the game outcome.

## Farbspiel second-seat FeedPartner (#131)

The Grand second-seat feeding admission also applies to a non-Ouvert Farbspiel
defender acting CURRENT SECOND with the partner leading: at least two pure Play
actions in exact core order, consistent Hand/pickup information and the actual
shared `FeedPartner` branch with `partner_ahead` and `table_master` true. The
unchanged Easy rule maximises `(!is_trump, Augen)` with the LAST exact tie; it
is a heuristic with no game guarantee. The remaining declarer may trump a side
master with any Trumpf, which the rule text states. Ouvert, declarer seats,
restricted or reordered actions and single choices stay excluded. The dialogue
observes the actual third card and individual winner. Technical coverage is the
`guided_suit_early_feed` coach test (full-deal replies with the declarer
trumping, hidden-holder equality, input mutations); the existing app/GUI
dialogue plumbing for permitted-view estimates is shared with the Grand
family. Learner, UAT and transfer evidence are not claimed.

### Ordinary walkthrough

Start ordinary `SessionConfig` seed **4**, fixed three games, Easy opponents.
The complete human commands live in
`crates/gui-egui/tests/fixtures/suit-early-feed-route.json` (Pass, Play index 0)
and are replayed through real widgets without an injected session. In
game1/trick2 Michael declares Pik; the human is physical Vorhand, partner
Hinterhand leads Karo-Ass and the declarer in Mittelhand plays last. Legal cards
are Karo-Neun and Karo-König. Easy chooses Karo-König by the last exact tie
(`FeedLast`). Either card is followed by the declarer's actual Karo-Sieben;
Hinterhand wins. The dialogue observes that card (it follows suit, so no void is
proven) and the individual winner, without placing any hidden card beforehand.

The `skat-app` test `suit_early_feed_ordinary_*` covers both legal cards, wrong
answers and retries, deliberate reveal, memory, stale ids and return. The
`suit_early_feed` widget test exercises both cards in both languages at
1440×1000 and 960×640, and sixteen `guided-suit-early-feed-*` screenshots show
orientation, rule, observation question and feedback. These are technical
checks, not learner or transfer observations; the wider #131 later-phase
criterion remains open (Grand/Null remaining branches, Ouvert and
other unsupported decisions).

## Ouvert Farbspiel declarer lead (#273)

On the declarer's own empty-trick lead in an Ouvert Farbspiel, the existing
`LeadMaster` dialogue is available when Easy selects a non-Trumpf follow-class
master and at least two pure legal cards are available. Easy compares all such
masters by Augen, taking the last exact tie. A follow-class master can still
be trumped; neither the open table display nor God Mode supplies defender hands
to this dialogue. Other branches remain unsupported. Every legal card remains
playable, and the actual replies are observed after the player's choice.

## Null defender opening lead (#289)

A Null or Null Ouvert defender in Vorhand can enter the existing lead dialogue
on the first empty trick, with at least two exact core-legal card actions.
Easy minimises `(is_trump, Augen, Null power)` and keeps the first exact tie
in legal order. Augen are a sorting cost, not Null scoring. The dialogue uses
the defender's hand, public contract and, in Ouvert, the revealed declarer
hand; the partner's hand remains concealed and neither reply is known. Every
legal card remains playable and both actual replies and the completed winner
are observed. Later leads, other Ouvert defender turns and restricted actions
remain unsupported. The core-validated Ouvert Hand fixture swaps a concealed
partner card with the Skat without changing the lesson; it checks all legal
first leads and rejects missing or inconsistent reveals.
The ordinary seed 1282 with two Easy opponents reaches the dialogue after
the human passes. Bilingual headless widgets exercise reveal, play, both
replies and return at 1440 and 960 pixels. This technical evidence does not
establish learner efficacy.

## Ouvert Grand declarer lead (#131)

The existing `LeadMaster` dialogue also admits the declarer's own empty-trick
lead in Grand Ouvert when the revealed hand matches the acting hand and Easy
selects a non-Trumpf follow-class master. At least two exact legal actions are
required. Easy chooses the highest Augen among those masters, breaking ties by
the last card in legal order. A side master can still be trumped; all legal
cards remain playable and the actual replies are observed. Other Grand Ouvert
branches remain unsupported. The core-accepted fixture in
`guided_suit_lead_master.rs` announces Grand Ouvert after Hand and verifies
the candidate set and the revealed-hand boundary; the app test exercises
reveal, free play and observation. No learner outcome is implied.

## Null Ouvert declarer playing last (#293)

After the reveal, a Null Ouvert declarer acting last may use the existing
last-seat dialogue when the revealed hand matches the acting hand and Easy
has a losing candidate. Easy selects the highest Null rank among losing cards,
with the last exact tie in legal order; every legal card remains playable.
The completed trick records the actual winner. Inconsistent or restricted views retain the fallback. The core-replayed
`real-ouvert` fixture checks all four legal choices and hidden substitutions.
These checks do not establish learning efficacy.

## Grand Ouvert declarer DrawTrumpf lead (#131)

The existing Grand declarer `DrawTrumpf` dialogue also accepts an own
empty-trick lead in Grand Ouvert after the hand is revealed and matches the
acting hand. The shared Easy rule selects its highest-power own Bube while
Trumpf remains unaccounted for; this does not locate the unseen Buben or
guarantee a result. Every legal card stays playable and the actual replies are
observed. Other Grand Ouvert branches remain unsupported. The core-accepted
Ouvert Hand witness in `guided_draw_trumpf.rs` and the app test exercise this
branch and its visibility and legal-action boundaries.

## Grand Ouvert declarer playing last (#131)

The declarer acting last with a matching revealed hand can use the existing
`WinLast` dialogue when the complete core-legal card list has at least two
choices. Easy selects its lowest winning card; every legal card remains
playable and the completed trick reports the actual winner. Other Ouvert
turns remain outside this admission.

The core fixture in `guided_grand.rs` announces Grand Ouvert Hand in
Hinterhand: after Kreuz-König and Kreuz-Acht, Kreuz-Ass wins and Kreuz-Dame
does not. Replacing the lead with Kreuz-Neun makes both cards winners.
Swapping an unseen defender card with the unknown Skat preserves the permitted
view and lesson. Enter the dialogue, reveal the rule, play either card and
inspect the actual winner; no learner outcome is inferred from this check.

## Null Ouvert declarer acting second (#131)

After the reveal, the declarer acting second may use the existing Null
losing-card dialogue when the revealed hand matches the acting hand and the
exact legal list contains a losing card. Easy chooses the highest losing Null
rank, with its existing last-in-order tie-break. All legal cards remain
playable; the actual third card and winner are observed. The core-replayed
mixed Hand fixture checks candidates, reveal and legal-action boundaries,
and the session test checks each legal choice and its actual continuation.
These checks do not establish learner outcomes.

## Farbspiel Ouvert declarer playing last (#318)

The declarer acting last with a matching revealed hand and a complete legal
card list can enter the shared `WinLast` dialogue. Every legal card remains
playable; the completed trick reports its actual winner. The Hand Skat and
defender cards remain unknown. The core fixture tests winning and losing
choices, hidden-card substitution and malformed views; app tests exercise
attempt, reveal, observation and return. The seeded egui test also walks the
real bilingual widgets from bidding through return to play. These checks do not establish
learning efficacy.

## Grand Ouvert defender under partner (#315)

A defender acting second after the partner may enter the existing
`UnderPartner` dialogue in Grand Ouvert when the revealed declarer hand and
the exact core-legal actions form a consistent view. The shared Easy rule
selects a current loser by nontrump, fewer Augen, then lower power, with the
first exact tie. Its follow-class master test does not separately examine
the revealed hand; the dialog says so. Every legal card remains playable,
and the actual declarer reply and winner are observed. Other Ouvert branches
remain outside this increment. The core fixture covers hidden Skat swaps,
reveal and legal-action boundaries, and all legal choices; the app exercises
attempt, reveal and actual continuation. No learner efficacy is inferred.
