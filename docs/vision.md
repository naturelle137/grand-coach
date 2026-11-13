# Product vision

Grand-Coach aims to help a player become an expert at the Skat table. Knowing
the rules gets a learner into a game. Expertise means noticing which cards and previous tricks matter for the next
decision, reading a position
from a partner's or opponent's seat, and changing a plan when a trick brings
new information. An interactive live coach and focused training sessions are
the two ways the player should be able to develop that judgement.

## Learning in a live game

From Reizen and Drücken through the last trick, the coach should help the
player find the question an experienced player would ask here. The question
changes with the contract, the role, who still has to play and what the player
has seen. A hint draws attention to one relevant detail and leaves the choice
to the player. After the move, the coach can revisit the position, compare
alternatives and explain when a familiar Skat tactic works or fails.
A defender might give up the lead so their partner can lead through the
declarer on the next trick. Seeing that opportunity takes an understanding
of seat order as well as card ranks.

For example, imagine playing Hinterhand as declarer in a Herzspiel. You win
a trick and now have the lead. You hold the Kreuz-Ass. A defender failed to
follow Kreuz earlier. Before you lead again, which cards in the previous
tricks matter? The void tells you that defender can no longer follow Kreuz.
If they still hold Trumpf, they can take the Ass. If all opposing Trumpf
are accounted for, that danger has passed. The skill is recognising when
counting Trumpf changes the decision to cash a side Ass.

## Practising a theme until it transfers

The planned training mode lets the player choose a theme such as protecting
a side Ass, managing Trumpf or supporting a partner in defence. A session
can start from a moment encountered in live play. In a session on the
Kreuz-Ass, the player would first identify the void with help, then compare
two legal positions that differ in the outstanding Trumpf. In a fresh deal
with another trump suit, the player would decide unaided and explain which
observation mattered. Recognising the same opportunity at a real table is
the long-term measure of success.

The deliberately entered [guided Farbspiel dialogue](coach.md#guided-farbspiel-dialogue-131)
now lets the learner answer one question at a time, reveal Easy's actual
conditional rule move and inspect the resulting public play. Free choice of
every legal card and an immediate return to play remain available. Early,
automatic and unaided help conceal the move; figures require a separate
request and enabled figures. This first bounded slice does not establish
learning efficacy or cover other phases/contracts.

The current [live coach](coach.md) also provides staged hints, six supported
lessons, a learning focus and a review of selected decisions. Optional focused
retrieval asks for exact counts before feedback,
with explicitly selected or restored help. It records support for the current
opportunity without claiming mastery. A first focused counting training
practises these two counts on fixed positions outside a game. A separate
[twelve-case catalogue](practice-catalogue.md) prepares varied legal positions
for the planned focus sessions; it is not yet a learner-facing session. The [roadmap](roadmap.md)
covers the next steps from guided play to independent practice.

## Playing environment

The game uses ISkO rules with optional Kontra/Re and Seeger-Fabian scoring.
It supports fixed-length series and free play. The two UI languages are German
and English. Card legibility takes priority over animation. Multiplayer,
mobile, Ramsch and Bock are outside the project scope.

## Architecture (guideline)

The rule engine owns legal actions and scoring. Analysis works from the
acting seat's view. Opponents choose actions and the coach explains shared
analysis results. Only explicitly privileged modes receive concealed cards.
Frontends access sessions through the plain-data `skat-app` interface.

See the [core architecture](architecture.md), [implementation status and next
work](roadmap.md) and [binding decisions](decisions.md).
