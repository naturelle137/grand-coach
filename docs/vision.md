# Product scope

Grand-Coach is an offline desktop Skat trainer for Linux and Windows 11.
The player plays against two computer opponents. The optional coach helps the
player count cards and reason from another seat's perspective. Before a move,
the player can ask what is known, what can be inferred and which options are
legal. After the move, the coach can discuss a concrete alternative.

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
