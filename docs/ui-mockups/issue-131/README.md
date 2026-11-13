# Guided coach: UI mockups for issue #131

Four proposed desktop states follow the maintainer's table-centred sketch.
These are design references, not screenshots of implemented functionality.
The UI copy is German; repository documentation remains English.

| State | Image | Design point |
| --- | --- | --- |
| Orientation | [01-orientierung.png](01-orientierung.png) | One question, neutral legal choices, memory collapsed. |
| Check | [02-pruefen.png](02-pruefen.png) | Feedback after an answer; exact facts versus unknown location; memory explicitly opened. |
| Act | [03-handeln.png](03-handeln.png) | Deliberately revealed heuristic move; another reasonable choice; all legal cards remain available. |
| Observe | [04-beobachten.png](04-beobachten.png) | Actual next play, a retrieval question, and separately requested retrospective model comparison. |

The compact coach sits left of the table. The last trick and known Augen
sit right; the hand stays below the table. The same layout is retained as
the dialogue advances. Cards outside the led class are faded only because
they are illegal to play; a legal card set aside by the heuristic keeps its
normal contrast and remains selectable.

## Source and rendering

[mockup.html](mockup.html) is self-contained and contains the four states.
Open it locally with `?state=1`, `?state=2`, `?state=3` or `?state=4`.
The PNGs were rendered with Chromium at 1600 by 1000 pixels, scale 1.
This is a fixed-size visual reference, not a responsive application or
working game. The buttons do not implement the proposed lesson lifecycle.

The HTML and images use the project's `assets/cards/french4` card artwork,
embedded unchanged as SVG data URLs. No external fonts, images, services,
generated bitmap artwork or new third-party assets are included.
The layout, HTML and resulting PNGs are licensed under CC BY-SA 4.0,
following `LICENSE-ASSETS`; attribution: Grand-Coach contributors.

## Example semantics and limits

The learner defends in Mittelhand in a Herzspiel. Partner leads Kreuz-König
in trick 4; the learner can follow with Kreuz-Sieben, -Acht or -Zehn.
Other hand cards are Herz-Bube, Karo-Bube, Pik-Ass and Karo-Neun.
Prior tricks, in play order, are:

1. Partner: Kreuz-Bube; learner: Herz-Sieben; declarer: Herz-Acht.
2. Partner: Pik-Sieben; learner: Pik-König; declarer: Pik-Zehn.
3. Declarer: Karo-Sieben; partner: Karo-Ass; learner: Karo-Acht.

Those tricks establish 13 Augen for the defenders, 14 for the declarer
and three played Trumpf. Two Trumpf remain in the learner's hand;
six are outside that hand and the played cards, including possible Skat.
In state 4, the learner has played Kreuz-Sieben and the declarer Kreuz-Ass;
the declarer wins 15 more Augen. Playing the Ass proves neither a prior
inference about its holder nor a subsequent Kreuz void.

The position is constructed for layout and copy consistency. It is not a
core-validated lesson fixture or strategic certification. The numbers in
the model-comparison panel are explicitly fictional design placeholders,
not engine results, measurements or win probabilities. Displayed spans
represent one illustrative standard error around expected Seeger-Fabian
points and do not establish a decisive difference between the cards.

The distinction between proof, estimate and unknown information is binding
in [issue #131](https://github.com/naturelle137/grand-coach/issues/131).
These images do not settle its implementation or learner acceptance.
