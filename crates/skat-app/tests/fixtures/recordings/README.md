# Recording fixtures

SPDX-License-Identifier: EUPL-1.2

Synthetic format-1 API fixtures, fixed seed 7 and explicit choices. Context
uses synthetic opponent seeds 1/2 and series seed 0; these are descriptive
provenance, not a claim that a current AI generated these choices. The JSON
was initially captured from the core; the assertions below are independently
specified from the cards, winners and ISkO scoring. The tests also compare
all prefixes to a separate core execution, including legal actions and events.
No production IO or current AI policy is used to reconstruct these fixtures.

Notation: C/S/H/D = Kreuz/Pik/Herz/Karo; T/J/Q/K/A = Zehn/Bube/Dame/Koenig/Ass.
Seed 7 deals:

- VH: C9 CT CJ CQ H9 HT HQ HK D9 DK
- MH: CK CA S8 ST SJ SQ H7 D7 D8 DJ
- HH: C7 C8 S7 SK SA H8 HJ HA DT DQ
- Skat: S9 DA

Expected facts (winner indices VH=0, MH=1, HH=2):

| Fixture | Winners by trick | Declarer's eyes, including Skat | Game value and score |
|---------|------------------|--------------------------------|----------------------|
| Herz pickup, C9/CT discarded | 0 1 1 0 2 0 1 2 2 0 | 4 + 0 + 17 + 14 + 10 discarded = 45 | mit 1, Spiel 2: 10 × 2 = 20; lost, −40 |
| Grand Hand Ouvert, Kontra/Re | 1 1 0 0 0 1 2 2 2 0 | 2 + 17 + 0 + 7 + 11 Skat = 37 | mit 1 + Spiel + Hand + 2 Schneider + 2 Schwarz + Ouvert = 8; 24 × 8 = 192; lost × 2 × Re 4 = −1536 |
| Null Hand Ouvert | 1 1 1 2 2 0 | HT/H8/H7 = 10, plus Skat 11 = 21 | fixed value 59; first declarer trick at trick 6 ends play, −118 |

The Grand defenders first bid/hold 18 and 20, making Kontra legal. The
announcement does not reveal the hand until Kontra and Re are resolved.
Hand Skat never becomes `own_skat`, even at completion. In the pickup
fixture only the declarer sees C9/CT after discarding. Null reveals the
hand on announcement; six tricks leave four cards in every hand.

These three fixtures all lose; they do not certify strategic quality or
exhaust every contract. All-pass and real session capture/game boundaries
have separate integration tests. Save/load, UI and practice remain outside
this API slice and issue #12 remains open.
