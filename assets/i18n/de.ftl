# SPDX-License-Identifier: CC-BY-SA-4.0
# German user interface texts of Grand-Coach (D-32).

## Menu and settings

menu-game = Spiel
menu-new-series = Neue Serie …
menu-end-series = Serie beenden
menu-settings = Einstellungen
setting-language = Sprache
language-de = Deutsch
language-en = English
setting-deck = Kartenbild
deck-french4 = Französisch, vierfarbig (Turnierbild)
deck-french2 = Französisch, zweifarbig
deck-german = Deutsches Blatt (Altenburger Bild)
setting-developer = Entwickleroptionen
setting-show-seed = Seed anzeigen

## New series

new-series-title = Neue Serie
series-fixed = Serie mit fester Spielzahl
series-games = Spiele
series-free = Freies Spiel
option-kontra-re = Mit Kontra und Re
option-see-all = Alle Karten sehen
option-see-all-description = God Mode: zeigt die Karten der Gegner und den Skat offen. Ausschalten, um ohne Hinsehen zu spielen.
series-seed = Startwert (Seed):
seed-description = Der Seed ist der Startwert des Zufallsgenerators: Derselbe Seed teilt dieselben Karten aus und lässt die Gegner gleich entscheiden, wenn du gleich spielst; so lässt sich eine Serie wiederholen. Erlaubt sind ganze Zahlen von 0 bis 18446744073709551615.
seed-invalid = Kein gültiger Seed: Gib eine ganze Zahl von 0 bis 18446744073709551615 ein.
button-start = Starten
button-cancel = Abbrechen

## Opponents

mode-levels = Gegner einzeln wählen
level-rookie = Grünschnabel
level-rookie-description = Grünschnabel: kennt die Regeln nur teilweise und vertut sich gelegentlich beim Reizen, bei der Spielwahl, beim Drücken und beim Ausspielen.
mode-goldfish = Goldfisch
mode-goldfish-description = Goldfisch: Ein zufällig bestimmter Gegner spielt stark, vergisst aber, welche Karten schon gefallen sind; der andere spielt auf Mittel.
mode-mephisto = Mephisto
mode-god = God Mode
mode-god-description = God Mode: Du siehst alle Karten, auch den Skat, und spielst gegen Mephisto. Auch der Coach sieht alle Karten; er sagt trotzdem nicht vor, sondern hilft beim Nachdenken.
mode-neural = KI-Netz (Test)
mode-neural-description = KI-Netz: Beide Gegner spielen ihre Karten mit dem trainierten neuronalen Netz im aktuellen Entwicklungsstand; Reizen, Skat und Ansagen übernimmt die Stufe Stark.
mode-neural-missing = Kein Modell gefunden (assets/models/card-play.gcnn): Die Gegner spielen durchgehend auf Stark.
opponent-left = Gegner links:
opponent-right = Gegner rechts:
level-easy = Leicht
level-medium = Mittel
level-strong = Stark
level-easy-description = Leicht: spielt nach einfachen Faustregeln und rechnet nichts voraus.
level-medium-description = Mittel: rechnet beim Kartenspiel und bei Kontra/Re einige mögliche Kartenverteilungen durch; reizt, drückt und sagt an nach Faustregeln.
level-strong-description = Stark: wie Mittel, rechnet aber viermal so viele mögliche Kartenverteilungen durch. Die stärkste Stufe; überlegt etwas länger.

## Status line

status-game = Spiel { $number } von { $total }
status-game-free = Spiel { $number }
status-dealer = Geber: { $player }
status-turn = Am Zug: { $player }
status-bid = Reizwert: { $value }
status-declarer = Alleinspieler: { $player }
status-seed = Seed { $seed }
status-levels = Gegner: { $left } (links), { $right } (rechts)
status-opponents = Gegner: { $mode }
status-waiting = { $player } überlegt …

## Players and seats

player-human = Du
player-left = Links
player-right = Rechts
seat-vorhand = Vorhand
seat-mittelhand = Mittelhand
seat-hinterhand = Hinterhand
label-declarer = Alleinspieler
label-cards-tricks = { $cards } Karten, { $tricks } Stiche
label-open-cards = Offene Karten:
label-skat = Skat
label-own-skat = Gedrückt:

## Phases

phase-reizen = Reizen
phase-skataufnahme = Hand oder Skat aufnehmen
phase-druecken = Drücken
phase-ansage = Spielansage
phase-spiel = Kartenspiel
phase-beendet = Spiel beendet
phase-eingepasst = Eingepasst

## Cards and games

suit-kreuz = Kreuz
suit-pik = Pik
suit-herz = Herz
suit-karo = Karo
suit-eichel = Eichel
suit-laub = Laub
suit-schellen = Schellen
suit-cross = Kreuz
suit-mond = Mond
suit-stern = Stern
name-alias = { $name } ({ $alias })
rank-sieben = Sieben
rank-acht = Acht
rank-neun = Neun
rank-zehn = Zehn
rank-bube = Bube
rank-dame = Dame
rank-koenig = König
rank-ass = Ass
card-name = { $suit }-{ $rank }
game-grand = Grand
game-null = Null
contract-hand = Hand
contract-schneider = Schneider angesagt
contract-schwarz = Schwarz angesagt
contract-ouvert = Ouvert
kontra-level-kontra = Kontra
kontra-level-re = Kontra und Re

## Bidding and actions

call-bid = { $value }
call-hold = Ja ({ $value })
call-pass = Passe
button-bid = { $value } sagen
label-jump-bid = Sprung auf:
button-jump = Springen
button-hold = Ja, { $value } halten
button-pass = Passe
button-hand = Hand spielen
button-take-skat = Skat aufnehmen
hint-discard = Wähle zwei Karten zum Drücken.
button-discard = Drücken
label-announce = Spiel ansagen:
button-announce = Ansagen
button-kontra = Kontra
button-kein-kontra = Kein Kontra
button-re = Re
button-kein-re = Kein Re
hint-play = Du bist dran: Spiel eine Karte.
button-next-game = Nächstes Spiel

## Tricks

trick-current = Aktueller Stich
trick-last = Letzter Stich
trick-winner = { $player } sticht ({ $augen } Augen)

## Results

result-title = Ergebnis
result-won = { $player } gewinnt { $game }.
result-lost = { $player } verliert { $game }.
result-augen = Augen des Alleinspielers: { $augen }
result-tricks = Stiche des Alleinspielers: { $count }
result-spitzen-mit = Mit { $count }
result-spitzen-ohne = Ohne { $count }
result-schneider = Schneider
result-schwarz = Schwarz
result-value = Spielwert: { $value }
result-ueberreizt = Überreizt: Reizwert { $bid }, gewertet mit { $value }
result-score = Listenpunkte: { $score }
result-points = { $player }: { $points } Punkte (Seeger-Fabian)
result-eingepasst = Alle haben gepasst – das Spiel ist eingepasst.

## Score sheet and series

sheet-title = Spielliste
sheet-number = Nr.
sheet-game = Spiel
sheet-total = Summe
sheet-eingepasst = eingepasst
series-finished = Serie beendet nach { $games } Spielen
series-rank = { $rank }. { $player }: { $total } Punkte ({ $won } gewonnen, { $lost } verloren)

## Errors

error-not-your-turn = Du bist nicht am Zug.
error-no-opponent-to-move = Kein Gegner ist am Zug.
error-stale-opponent-move = Der Gegnerzug passt nicht mehr zur aktuellen Lage.
error-game-in-progress = Das laufende Spiel ist noch nicht beendet.
error-series-finished = Die Serie ist beendet. Starte eine neue Serie.
error-not-free-play = Nur eine Serie im freien Spiel kann vorzeitig beendet werden.
error-game-over = Das Spiel ist bereits beendet.
error-unexpected-action = Das ist in der Phase „{ $phase }“ nicht möglich.
error-invalid-bid-value = { $value } ist kein gültiger Reizwert.
error-bid-too-low = { $value } liegt nicht über dem Reizwert { $current }.
error-invalid-discard = Drücke genau zwei Karten aus deiner Hand.
error-invalid-announcement = Diese Kombination aus Spiel und Ansagen ist nicht erlaubt.
error-null-value-below-bid = Dieses Nullspiel ist nur { $value } wert, gereizt wurde { $bid }.
error-card-not-in-hand = { $card } ist nicht auf deiner Hand.
error-must-follow-suit = { $card } geht nicht: Du musst bedienen.
error-kontra-not-allowed = Kontra ist hier nicht erlaubt.

## Coach

menu-coach = Coach
setting-coach-level = Coach meldet sich
coach-level-off = Aus
coach-level-on-request = Auf Nachfrage
coach-level-important = Bei wichtigen Entscheidungen
coach-level-every-move = Bei jedem Zug
setting-coach-warnings = Vor schwachen Zügen warnen
setting-coach-warnings-description = Hält einen deutlich schwächeren Zug an, erklärt warum und lässt dich ihn trotzdem spielen oder neu entscheiden.
setting-coach-figures = Zahlen anzeigen
setting-coach-focus = Lernschwerpunkt
setting-coach-focus-description = Ein Thema, auf das der Coach in diesem Spiel zuerst achtet. Du wählst es selbst; der Coach stellt keine Diagnose. Es bleibt fürs nächste Spiel, bis du es änderst.
coach-focus-none = Kein Schwerpunkt
coach-focus-current = Dein Schwerpunkt: { $focus }
coach-concept-hand-structure = Eigene Buben, Trümpfe und Spitzen
coach-concept-game-value = Spielwert und Reizwert
coach-concept-proven-void = Wer nicht bedienen konnte
coach-concept-remaining-trumpf = Wo die Trümpfe sind
coach-concept-trumpf-vulnerability = Kann ein volles Blatt abgestochen werden?
coach-concept-partner-overtake = Kann der Partner übernehmen?
coach-question-compare = Prüfe jede Möglichkeit an dem, was du sicher weißt: Was ändert sie für dich und für die anderen?
coach-status-no-lesson = Zu dieser Entscheidung hat der Coach keine geprüfte Lektion. Hinweise gibt es auf Nachfrage.
button-coach-skip = Ausblenden
coach-moment-focus = dein Schwerpunkt
coach-moment-chosen-provisional = Du hast { $action } gewählt. Die Analyse ist hier nicht fertig geworden; das Modell bewertet den Zug deshalb nicht.
coach-title = Coach
button-coach-considerations = Was soll ich bedenken?
button-coach-situation = Wie ist die Lage?
button-coach-figures = Zahlen zeigen
coach-section-situation = Die Lage
coach-situation-forced = Nur ein Zug ist möglich.
coach-situation-one = Eine Option ist klar besser als alle anderen. Welche?
coach-situation-several = { $count } Optionen sind etwa gleich gut.
coach-situation-unclear = Die Lage lässt sich nicht sicher einschätzen.
coach-section-figures = Zahlen je Option (ohne Wertung – vergleiche selbst)
coach-note = Dein letzter Zug ({ $action }): { $note }
coach-note-good = gut.
coach-note-about-equal = etwa so gut wie der beste.
coach-note-somewhat-weaker = etwas schwächer als der beste.
coach-note-clearly-weaker = deutlich schwächer als der beste.
coach-note-forced = es gab keine Wahl.
coach-note-uncertain = keine sichere Bewertung.
coach-note-computing = der Coach bewertet ihn noch …
coach-debriefing-title = Nachbesprechung
coach-debriefing-computing = Der Coach wertet noch { $count } Entscheidungen aus …
coach-debriefing-none = In diesem Spiel gab es keine Entscheidung mit einer geprüften Lektion.
coach-moment-trick = Stich { $trick }
coach-moment-chosen-best = Du hast { $action } gewählt. Das Modell bewertet das hier am höchsten.
coach-moment-chosen-equivalent = Du hast { $action } gewählt. Das Modell bewertet das etwa so hoch wie { $best }.
coach-moment-chosen-worse = Du hast { $action } gewählt. Das Modell bewertet das etwas niedriger als { $best }.
coach-moment-chosen-weak = Du hast { $action } gewählt. Das Modell bewertet das deutlich niedriger als { $best }.
coach-moment-chosen-unrated = Du hast { $action } gewählt. Das Modell hat dafür keine verlässliche Bewertung.
coach-moment-no-reason = Die Lektion begründet nicht die Bewertung des Modells; den Modellvergleich findest du in den Analysedetails.
coach-context-position = Damals: du in { $seat }, { $role }; Geber { $dealer }.
coach-context-bid = Reizwert bis dahin: { $value }
coach-context-no-bid = Noch kein Gebot.
coach-context-declarer = Alleinspieler: { $player }, gereizt bis { $value }
coach-context-contract = Spiel: { $game } von { $player }, gereizt bis { $value }
coach-context-kontra = Angesagt: { $level }
coach-context-own-skat = Gedrückt: { $cards }
coach-context-open = Offene Karten von { $player }: { $cards }
coach-context-hand = Deine Karten vor dem Zug:
coach-context-trick = Stich { $trick } bis dahin:
coach-context-lead = Du spielst Stich { $trick } an.
coach-context-history = Verlauf bis zu dieser Entscheidung
coach-context-calls = Reizverlauf: { $calls }
coach-context-no-calls = Reizverlauf: noch kein Gebot.
coach-context-call = { $player } { $call }
coach-context-past-trick = Stich { $trick }: { $cards } – { $winner } sticht ({ $augen } Augen)
coach-context-card = { $player } { $card }
coach-context-no-tricks = Noch kein Stich gespielt.
coach-details = Analysedetails
coach-details-comparison = Modellvergleich von { $action } mit { $other }:
coach-details-rating = Modellbewertung von { $action }: { $judgement }
coach-metric-label-win = Gewinnchance ({ $party })
coach-metric-label-points = Bewertungswert (erwartete Seeger-Fabian-Punkte)
coach-metric-label-points-defender = Bewertungswert (erwartete Seeger-Fabian-Punkte plus die Hälfte der verlorenen Spielpunkte des Alleinspielers)
coach-metric-label-declarer = Spielpunkte des Alleinspielers (mit Kontra/Re)
coach-metric-label-thrown-in = Chance, dass alle passen
coach-moment-trap = { $trap } bewertet das Modell hier deutlich niedriger.
coach-moment-trap-loss = Abstand im Bewertungswert des Modells: etwa { $points } Punkte.
coach-status-analysing = Der Coach rechnet …
coach-status-computing = Der Coach rechnet noch an deiner Frage …
coach-status-idle = Frag den Coach, wenn du Hilfe möchtest.
coach-status-waiting = Der Coach meldet sich bei deiner nächsten Entscheidung.
coach-unavailable-cancelled = Die Analyse wurde abgebrochen. Frag erneut.
coach-unavailable-failed = Für diese Entscheidung ist keine Analyse möglich.
coach-checking = Der Coach prüft deinen Zug: { $action }
button-coach-proceed = Trotzdem spielen
button-coach-skip-check = Ohne Prüfung spielen
button-coach-reconsider = Zurück zur Entscheidung
coach-warning-title = Bist du sicher?
coach-warning-text = { $action } ist voraussichtlich deutlich schwächer als die beste Option. Überleg noch einmal.
coach-warning-loss = Erwarteter Verlust gegenüber dem besten Zug: etwa { $points } Punkte (Seeger-Fabian).
coach-action-bid = { $value } sagen
coach-action-hold = { $value } halten
coach-action-discard = { $cards } drücken
coach-action-announce = { $game } ansagen
coach-decision-bid = Reizen: sagen oder passen
coach-decision-answer = Reizen: halten oder passen
coach-decision-pickup = Hand spielen oder Skat aufnehmen
coach-decision-discard = Drücken
coach-decision-announce = Spiel ansagen
coach-decision-card = Karte spielen
coach-decision-kontra = Kontra oder nicht
coach-decision-re = Re oder nicht
coach-role-bidder = noch ohne Rolle
coach-role-declarer = du bist Alleinspieler
coach-role-defender = du bist Gegenspieler
coach-objective-undecided = Noch kein Spiel angesagt: Jede Option wird danach bewertet, wie das folgende Spiel voraussichtlich ausgeht.
coach-objective-augen = Ziel: mindestens { $target } Augen.
coach-objective-augen-defender = Ziel: den Alleinspieler unter { $target } Augen halten.
coach-objective-all-tricks = Ziel: alle Stiche (Schwarz angesagt).
coach-objective-all-tricks-defender = Ziel: mindestens einen Stich machen (Schwarz angesagt).
coach-objective-no-trick = Ziel (Null): keinen einzigen Stich machen.
coach-objective-no-trick-defender = Ziel (Null): den Alleinspieler zu einem Stich zwingen.
coach-important = Wichtige Entscheidung: Eine naheliegende Option kostet deutlich Punkte.
coach-provisional = Vorläufig: Das Rechenbudget reichte nicht für alle Verteilungen – keine sichere Lageeinschätzung.
coach-section-facts = Was du sicher weißt
coach-section-inferred = Was wahrscheinlich ist (geschätzt, nicht sicher)
coach-section-options = Mögliche Züge (ohne Wertung)
coach-section-basis = Worauf die Schätzung beruht
coach-fact-void = { $player } hat keine { $class } mehr (nicht bedient).
coach-fact-trumps-out = Noch { $count } Trümpfe bei den anderen oder im Skat.
coach-fact-augen-won = Augen in Stichen: Alleinspieler { $declarer }, Gegenspieler { $defenders }.
coach-fact-augen-out = Noch { $augen } Augen bei den anderen oder im Skat.
coach-fact-skat-known = Im Skat liegen: { $cards }.
coach-fact-bid-shown = { $player } hat bis { $value } gereizt.
coach-fact-kontra-eligible = { $player } darf noch Kontra sagen.
coach-fact-trumpf-unseen = { $count } Trumpf hast du noch nicht gesehen.
coach-fact-structure = { $game }: Du hast { $count } Trumpf.
coach-fact-game-values = { $game }: mögliche Spielwerte { $value }.
coach-fact-commitment = Das Spiel muss mindestens { $value } wert sein.
coach-section-lesson = Worauf achten?
coach-section-lesson-review = Was sicher feststeht
coach-lesson-class-trumpf = Trumpf
coach-lesson-remaining-trumpf-question = Zähl nach: Welche Trumpf hast du noch nicht gesehen – und wo können sie liegen?
coach-lesson-remaining-trumpf-total = { $kind ->
        [grand] Im Grand sind nur die { $count } Buben Trumpf.
       *[suit] Im { $game }-Spiel gibt es { $count } Trumpf: die 4 Buben und die 7 { $game }-Karten.
    }
coach-lesson-remaining-trumpf-own = In deiner Hand: { $count ->
        [0] keiner.
       *[other] { $count } ({ $cards }).
    }
coach-lesson-remaining-trumpf-played = Schon gespielt: { $count ->
        [0] keiner.
       *[other] { $count } ({ $cards }).
    }
coach-lesson-remaining-trumpf-skat = In deinem gedrückten Skat: { $count } ({ $cards }). Die hält niemand auf der Hand.
coach-lesson-remaining-trumpf-revealed = Offen beim Alleinspieler: { $count } ({ $cards }).
coach-lesson-remaining-trumpf-unseen = Nicht gesehen: { $count ->
        [0] keiner. Verdeckt liegt kein Trumpf mehr.
       *[other] { $count } ({ $cards }), in einer verdeckten Hand{ $skat ->
            [unknown] {""} oder im Skat.
           *[known] . Den Skat kennst du.
        }
    }
coach-lesson-remaining-trumpf-exception = { $skat ->
        [unknown] Achtung: Wer die nicht gesehenen Trumpf hat, steht damit nicht fest. Sie können auf die verdeckten Hände verteilt sein oder im Skat liegen.
       *[known] Achtung: Trumpf im eigenen Skat sind keine Trumpf der Gegner. Wie sich die nicht gesehenen auf die verdeckten Hände verteilen, steht damit nicht fest.
    }
coach-lesson-remaining-trumpf-exception-none = Achtung: Das sagt nur etwas über Trumpf, nicht über die Fehlfarben.
coach-lesson-remaining-trumpf-transfer = Beim nächsten Trumpfstich: Wie viele Trumpf sind danach noch nicht gesehen?
coach-lesson-proven-void-question = Hat gerade jemand nicht bedient? Was weißt du dadurch sicher über seine Karten?
coach-lesson-proven-void-evidence = Stich { $trick }: { $player } hat { $card } auf { $led } gespielt, also nicht { $class } bedient. Wer bedienen kann, muss bedienen (ISkO) – { $player } hat { $follow ->
        [trumpf] keinen Trumpf
       *[suit] kein { $class }
    } mehr.
coach-lesson-proven-void-exception = { $case ->
        [trumpf] Das schließt nur Trumpf aus. Wer einen späteren Stich gewinnt, folgt daraus nicht.
        [null] Im Null gibt es keinen Trumpf: Wer frei ist, wirft ab. Wer einen späteren Stich nimmt, folgt daraus nicht.
       *[suit] Das schließt nur { $class } aus. Wer frei ist, darf stechen oder abwerfen – ob er das tut und wer den Stich gewinnt, folgt daraus nicht.
    }
coach-lesson-proven-void-transfer = Wenn wieder { $class } gespielt wird: Wer kann dann noch bedienen?
coach-section-lesson-mechanism = Was in diesem Stich möglich war
coach-lesson-trumpf-vulnerability-evidence = { $player } spielt in diesem Stich nach deiner Karte ({ $card }). { $certainty ->
        [certain] Sicher ist: { $player } hat kein { $suit } mehr und mindestens einen Trumpf (folgt aus den gesehenen Karten und den bewiesenen Fehlfarben) – { $player } kann also stechen.
       *[possible] { $player } kann nur stechen, wenn er kein { $suit } mehr und mindestens einen Trumpf hat. Beides ist möglich, nicht sicher: { $count } { $suit } und { $trumps } Trumpf hast du nicht gesehen.
    }
coach-lesson-trumpf-vulnerability-rule = Regel (ISkO): Wer bedienen kann, muss bedienen. Nur wer kein { $suit } hat, darf stechen – oder stattdessen abwerfen.
coach-lesson-trumpf-vulnerability-exception = Achtung: Stechen können heißt nicht stechen werden. Ein möglicher Abstich allein zeigt nicht, dass vorher Trumpfziehen besser gewesen wäre; die Bewertung deines Zugs oben ist ein eigenes Modellurteil.
coach-lesson-trumpf-vulnerability-transfer = Bevor du ein Ass oder eine Zehn einer Fehlfarbe spielst: Wer spielt nach dir, kann er in der Farbe frei sein – und hat er noch Trumpf?
coach-lesson-partner-overtake-evidence = { $declarer } hat als Alleinspieler { $best } ausgespielt; nach deiner Karte ({ $card }) nimmt sie den Stich weiterhin. { $player } spielt als Letzter.
coach-lesson-partner-overtake-ways = { $player } kann sie { $way ->
        [both] mit einer höheren Karte ({ $cards }) oder, ohne { $class }, mit einem Trumpf
        [trump] nur ohne { $class } mit einem Trumpf
       *[follow] mit einer höheren Karte ({ $cards })
    } schlagen. { $certainty ->
        [certain] Das ist sicher: Wie die verdeckten Karten auch liegen, { $player } hat eine solche Karte.
       *[possible] Das ist möglich, nicht sicher: Es hängt von Karten ab, die du nicht gesehen hast.
    }
coach-lesson-partner-overtake-rule = { $case ->
        [trumpf] Regel (ISkO): { $player } muss Trumpf bedienen, wenn er kann; nur ein höherer Trumpf gewinnt.
       *[suit] Regel (ISkO): { $player } muss { $class } bedienen, wenn er kann; dann gewinnt nur eine höhere { $class }-Karte. Ohne { $class } gewinnt jeder Trumpf.
    }
coach-lesson-partner-overtake-exception = Achtung: Schlagen können heißt nicht schlagen werden. Über spätere Stiche und darüber, welche Karte du hättest spielen sollen, sagt das nichts.
coach-lesson-partner-overtake-transfer = Wenn der Alleinspieler ausspielt und dein Partner hinter dir sitzt: Mit welchen Karten kann er die Karte des Alleinspielers noch schlagen?
coach-lesson-spitzen = { $case ->
        [mit] mit { $count }
       *[ohne] ohne { $count }
    }
coach-lesson-preplay-model = Die Bewertung deines Zugs oben ist eine grobe Modellschätzung: Vor dem Kartenspiel simuliert das Modell nur wenige Verteilungen, und darin spielen alle so, als sähen sie alle Karten.
coach-lesson-hand-structure-question = Für jede Farbe und für Grand: Welche Buben und wie viele Trumpf hast du – und mit oder ohne wie viele?
coach-lesson-hand-structure-game = { $game }: { $kind ->
        [grand] { $count } Buben
       *[suit] { $count } Trumpf
    } ({ $cards }), { $value }.
coach-lesson-hand-structure-discarded = Davon in deinem gedrückten Skat: { $count } ({ $cards }). Sie zählen für die Spitzen, werden aber nicht gespielt.
coach-lesson-hand-structure-range = { $case ->
        [fixed] Der unbekannte Skat kann diese Spitzen nicht ändern.
       *[range] Mit dem unbekannten Skat: { $min } bis { $max } Spitzen möglich (mit oder ohne), bis zu { $count } { $kind ->
            [grand] Buben
           *[suit] Trumpf
        }.
    }
coach-lesson-hand-structure-skat = { $skat ->
        [unknown] Der Skat ist unbekannt. Seine zwei Karten zählen für die Spitzen des Alleinspielers (auch im Handspiel), die Spitzen können sich also noch ändern.
        [held] Du hast zwölf Karten. Die Spitzen zählen alle zwölf, auch die zwei, die du drückst; gespielt werden nur die zehn, die du behältst.
       *[discarded] Die zwei gedrückten Karten zählen weiter für die Spitzen, werden aber nicht gespielt.
    }
coach-lesson-hand-structure-exception = Achtung: Diese Zählung beschreibt deine Karten. Ob ein Spiel gewonnen wird, welches Spiel du spielen, wie hoch du reizen oder was du drücken solltest, folgt daraus nicht.
coach-lesson-hand-structure-transfer = Beim nächsten Blatt: Welche Buben und wie viele Trumpf hast du für jede Farbe und für Grand – und mit oder ohne wie viele?
coach-lesson-game-value-question = Was ist jedes deiner möglichen Spiele mit deinen Karten wert – und was kann der unbekannte Skat noch ändern?
coach-lesson-game-value-bid = { $case ->
        [call] Wer hier reizt oder hält, legt sich auf mindestens { $value } fest.
       *[declarer] Gereizt ist { $value }.
    } Ein Spiel, das weniger wert ist, ist überreizt und verloren (ISkO), egal wie viele Augen du bekommst.
coach-lesson-game-value-game = { $game }: { $value }
coach-lesson-game-value-entry = { $kind ->
        [plain] mit Skataufnahme
        [hand] Hand
        [ouvert] Ouvert
       *[handouvert] Hand Ouvert
    } { $value } ({ $case ->
        [reached] reicht
        [skat] nur mit passendem Skat
        [play] nur mit Schneider oder Schwarz im Spiel
       *[never] reicht nie
    })
coach-lesson-game-value-skat = { $skat ->
        [unknown] Der Skat ist unbekannt: Seine Karten zählen für die Spitzen, ein Bereich reicht also vom niedrigsten bis zum höchsten Wert, den ein Skat ergeben kann.
       *[known] Du kennst den Skat: Die Spitzen und damit diese Werte stehen fest.
    }
coach-lesson-game-value-levels = Werte ohne erspielten Schneider oder Schwarz. Jede erspielte Stufe zählt eine Stufe mehr, jede angesagte (nur Hand) zwei. Ouvert im Farbspiel oder Grand geht nur Hand mit angesagtem Schneider und Schwarz; Null hat feste Werte.
coach-lesson-game-value-kontra = Kontra und Re verdoppeln die notierten Punkte, nicht den Spielwert: Ob der Reizwert erreicht ist, ändern sie nicht.
coach-lesson-game-value-exception = Achtung: Erreicht ein Spiel den Reizwert, ist nur das Überreizen ausgeschlossen. Ob du gewinnst, folgt daraus nicht, und kein Spiel und kein Reizwert wird hier als besser genannt.
coach-lesson-game-value-transfer = Beim nächsten Reizwert: Welche deiner Spiele erreichen ihn sicher – und welche nur mit passendem Skat?
coach-holder-skat = Der Skat
coach-class-trumps = Trümpfe
coach-class-jacks = Buben
coach-inferred-certain = { $holder } hat sicher { $count } { $class } (aus den Regeln ableitbar).
coach-inferred-hardly = { $holder } hat vermutlich kaum { $class }.
coach-inferred-about-one = { $holder } hat vermutlich etwa einen der { $class }.
coach-inferred-several = { $holder } hat vermutlich mehrere { $class }.
coach-inferred-figures = (im Mittel { $expected }, zwischen { $min } und { $max })
coach-judgement-best = am besten
coach-judgement-equivalent = etwa gleich gut
coach-judgement-worse = etwas schwächer
coach-judgement-weak = deutlich schwächer
coach-judgement-screened = nur grob geprüft
coach-judgement-not-evaluated = nicht bewertet
coach-party-own = deine Partei
coach-party-declarer = du als Alleinspieler
coach-party-defenders = ihr Gegenspieler
coach-metric-win = Gewinnchance ({ $party }): { $chance }
coach-metric-score = erwartete Listenpunkte (Seeger-Fabian): { $score } ± { $se }
coach-metric-game-points = erwartete Spielpunkte des Alleinspielers: { $points }
coach-reason-win-more = gewinnt öfter als { $other }
coach-reason-win-less = gewinnt seltener als { $other }
coach-reason-points-more = bringt im Schnitt mehr Punkte als { $other }
coach-reason-points-less = bringt im Schnitt weniger Punkte als { $other }
coach-reason-declarer-more = mehr Spielpunkte für den Alleinspieler als { $other }
coach-reason-declarer-less = weniger Spielpunkte für den Alleinspieler als { $other }
coach-reason-thrown-in-more = wird eher eingepasst als { $other }
coach-reason-thrown-in-less = wird seltener eingepasst als { $other }
coach-reason-same = führt zum selben Ergebnis wie { $other }
coach-reason-uncertain = Der Unterschied liegt innerhalb der Unsicherheit der Schätzung.
coach-more-options = … und { $count } weitere Optionen
coach-basis-exact = Alle möglichen Verteilungen der verdeckten Karten wurden durchgerechnet (exakt im Modell).
coach-basis-known = Berechnet mit der echten Kartenverteilung: Der Coach sieht alle Karten (God Mode).
coach-basis-sampled = Schätzung über zufällig gezogene Verteilungen der verdeckten Karten.
coach-basis-sampled-figures = Schätzung über { $finished } zufällig gezogene Verteilungen der verdeckten Karten.
coach-assumption-uniform = Annahme: Die verdeckten Karten sind gleich wahrscheinlich verteilt, soweit die Regeln es erlauben.
coach-assumption-bidding = Annahme: Die Gegner reizen nach Faustregeln; Verteilungen, die nicht zum Reizen passen, zählen weniger.
coach-assumption-double-dummy = Annahme: Im Kartenspiel spielen alle so, als sähen sie alle Karten.
coach-assumption-policy = Annahme: Spätere Entscheidungen vor dem Kartenspiel folgen den Faustregeln.
coach-assumption-known = Keine Annahme über die verdeckten Karten: Die echte Verteilung aller Karten ist bekannt.
coach-assumption-bid-limit = Annahme: Ein Reizwert heißt „bis hierhin mitgehen“.
error-no-decision = Du hast gerade keine Entscheidung zu treffen.
error-coach-off = Der Coach ist ausgeschaltet.
error-nothing-pending = Es wartet kein angehaltener Zug.
error-action-pending = Entscheide zuerst über den angehaltenen Zug.
error-stale-coach-result = Die Analyse passt nicht mehr zur aktuellen Lage.
mode-experimental = Experimentell
mode-experimental-description = Exakte Kartensuche mit zusätzlicher Augenschranke. Der Coach zeigt beim Kartenspiel nach rund 200 ms einen Zwischenstand.
coach-preview-live = Zwischenstand aus { $finished } von { $total } Kartenverteilungen. Die Suche läuft weiter.
error-question-pending = Eine Zählfrage ist offen: Beantworte sie, überspringe sie oder schalte das Quiz zuerst aus.
error-practice-attempt = Ein Übungsversuch ist genau ein Spiel. Zurück zur Serie oder einen neuen Versuch starten.
error-no-question = Es ist keine Zählfrage offen.
menu-assist = Hilfen
setting-aid = Zählhilfe
setting-aid-description = Zeigt, was du über gespielte Karten, Trümpfe, Augen und fehlende Farben sicher wissen kannst. Verborgen, solange eine Zählfrage offen ist.
setting-quiz = Zählfragen
setting-quiz-description = Stellt nach dem dritten und dem sechsten Stich bei deiner Kartenwahl eine Zählfrage. Dein Zug wartet; du kannst antworten, überspringen oder das Quiz ausschalten.
aid-title = Zählhilfe
aid-exact = Jede Angabe ist exakt und folgt aus dem, was du gesehen hast.
aid-withheld = Verborgen, solange die Zählfrage offen ist.
aid-played = Bisher gespielt (mit dem Stich auf dem Tisch): { $count } von 32 Karten
aid-played-cards = Gespielte Karten: { $cards }
aid-trumpf = Trümpfe draußen: { $outside } von { $total } (deine { $own }, gespielt { $played })
aid-trumpf-discard = Dein eigener Drück enthielt { $discarded } Trümpfe.
aid-trumpf-skat = Den Skat kennst du nicht; ein Teil davon kann dort liegen.
aid-trumpf-null = Null hat keine Trümpfe.
aid-trumpf-later = Trümpfe: bekannt, sobald das Spiel angesagt ist.
aid-augen = Augen in Stichen: Alleinspieler { $declarer }, Gegner { $defenders }
aid-augen-later = Gewonnene Augen: bekannt, sobald der Alleinspieler feststeht.
aid-augen-outstanding = Augen weder in deiner Hand noch gespielt noch in deinem eigenen Drücken: { $count }
aid-augen-skat = Dein eigener Drück zählt { $count } Augen (nicht Teil der Stiche).
aid-voids = Nachgewiesene Fehlfarben
aid-void-line = { $player }: { $classes }
aid-void-none = Noch hat niemand eine Farbe nicht bedient.
aid-void-trumpf = Trumpf
quiz-title = Zählfrage
quiz-q-trumpf = Wie viele Trümpfe sind weder in deiner Hand noch gespielt noch in deinem eigenen Drück? Ein unbekannter Skat zählt als draußen.
quiz-q-augen-declarer = Wie viele Augen hat der Alleinspieler bisher in Stichen gewonnen?
quiz-q-augen-defenders = Wie viele Augen haben die beiden Gegner bisher zusammen in Stichen gewonnen?
quiz-q-played = Wie viele Karten der Gruppe { $class } sind bisher gespielt, einschließlich des Stichs auf dem Tisch?
quiz-class-trumpf = Trumpf
quiz-hint = Dein Zug wartet. Zählhilfe und Coach bleiben verborgen, bis du antwortest oder überspringst.
quiz-value = Deine Antwort: { $value }
quiz-minus-ten = − 10
quiz-minus-one = − 1
quiz-plus-one = + 1
quiz-plus-ten = + 10
quiz-submit = Antworten
quiz-skip = Überspringen
quiz-off = Quiz ausschalten
quiz-feedback-title = Letzte Zählfrage
quiz-correct = Richtig: { $expected }.
quiz-incorrect = Nicht ganz: Du hast { $given } geantwortet, richtig sind { $expected }.
quiz-skipped = Übersprungen. Richtig sind { $expected }.
quiz-why-trumpf = { $total } Trümpfe − { $own } in deiner Hand − { $played } gespielt − { $discarded } in deinem Drück = { $outside }.
quiz-why-trumpf-skat = Ein unbekannter Skat kann einen Teil davon enthalten.
quiz-why-augen = Diese Seite hat { $augen } Augen gewonnen, die andere { $other }.
quiz-why-played = { $played } der { $size } Karten der Gruppe { $class } sind gespielt, { $unplayed } noch nicht.

# Focused retrieval (#48)
setting-retrieval = Gezieltes Abrufen: Resttrümpfe und Fehlfarben
setting-retrieval-level = Hilfe beim gezielten Abrufen
retrieval-visible = Zählhilfe sichtbar
retrieval-request = Erst selbst zählen, auf Wunsch aufdecken
retrieval-answer = Erst antworten, dann aufdecken
retrieval-hint = Dein Zug wartet auf Antwort, Überspringen oder Ausschalten. Hilfe kannst du jederzeit zurückholen; der Versuch zählt dann als unterstützt.
retrieval-help = Hilfe einblenden
retrieval-assisted = Hilfe sichtbar – dieser Versuch ist unterstützt.
retrieval-withheld = Zählhilfe beim gezielten Abrufen verborgen. Über die Einstellungen kannst du sie wieder dauerhaft einblenden.
quiz-q-voids = Bei wie vielen anderen Spielern ist öffentlich nachgewiesen, dass sie keine Karte der Gruppe { $class } mehr haben?
quiz-why-voids = Andere Spieler, die die Gruppe { $class } nachweislich nicht bedienen konnten: { $count }. Unbekannte Hände werden nicht geraten.
retrieval-enter = Eine Zahl eingeben

# Local game recordings, deliberately separate from live play.
record-save = Spiel speichern …
record-open = Aufnahme öffnen …
record-file-help = Lokaler JSON-Spielstand (max. 1 MiB), keine Fortsetzung einer Serie. Vollständigen Dateipfad eingeben. Bestehende Dateien werden nicht überschrieben.
record-path = Dateipfad:
record-current = Aktuelles Spiel
record-completed = Abgeschlossenes Spiel { $number }
record-no-game = Kein Spiel vorhanden
record-export-warning = Die Datei enthält alle verdeckten Karten, den Skat und die Entscheidungen.
record-export-consent = Verdeckte Informationen bewusst in die Datei schreiben
record-save-file = Datei speichern
record-open-file = Datei öffnen
record-title = Aufnahme — nur lesen
record-isolation = Die laufende Serie pausiert. Hier sind keine Spielzüge möglich; Kommentare gibt es nur auf Anfrage zu einer aufgenommenen Entscheidung.
record-identity = Spiel { $number } · Seed { $seed } · Erstellt mit { $version }
record-incomplete = Unvollständige Aufnahme: endet bei einer offenen Entscheidung.
record-all-pass = Eingepasst: kein Spielvertrag und keine Stiche.
record-finished = Abgeschlossen. Null kann vor dem zehnten Stich enden; Restkarten bleiben sichtbar, wenn aufgedeckt.
record-first = Anfang
record-previous = Zurück
record-next = Weiter
record-last = Ende
record-boundary = Nach { $count } Aktionen
record-reveal = Alle verdeckten Karten und den Skat aufdecken
record-privileged = Vollständige Information — enthält damals unbekannte Karten; keine Schlussfolgerung aus Spielersicht.
record-hands = Restkarten an dieser Stelle
record-deal = Ursprünglich ausgeteilte Karten
record-bidding = Reizverlauf
record-tricks = Abgeschlossene Stiche an dieser Stelle
record-no-tricks = Noch keine abgeschlossenen Stiche.
record-trick = Stich { $number } · Ausspiel: { $leader } · Gewonnen: { $winner }
record-current-trick = Begonnener Stich
record-close = Aufnahme schließen
record-error-json = Ungültige JSON-Datei oder fehlerhafte Datenfelder. Bitte eine andere Datei wählen.
record-error-format = Diese Aufnahmeversion wird nicht unterstützt.
record-error-engine = Diese Regel- oder Austeilversion wird nicht unterstützt.
record-error-history = Ungültige Aufnahme: Spielverlauf, Karten oder Ergebnis stimmen nicht überein.
record-error-large = Aufnahme zu groß: höchstens 1 MiB und 512 Aktionen erlaubt.
record-error-missing = Datei oder Zielordner nicht gefunden. Bitte den Pfad prüfen.
record-error-exists = Der Pfad existiert bereits und wurde nicht verändert. Bitte einen neuen Dateinamen wählen.
record-error-io = Datei konnte nicht gelesen oder vollständig geschrieben werden. Pfad und Zugriffsrechte prüfen; nach Schreibfehler einen neuen Dateinamen wählen.
record-source = Spielauswahl
record-practice-start = Dieses Blatt üben
record-practice-help = Startet einen neuen Übungsversuch vor dem Reizen: dieselben Karten, Plätze, Regeln und Gegner. Die aufgenommenen Züge werden nicht nachgespielt; du entscheidest neu. Die Serie pausiert unverändert.
record-practice-unavailable = Während eines Übungsversuchs kann kein weiterer gestartet werden.
record-practice-error = Übung nicht möglich: Die Aufnahme ist ungültig oder wird nicht unterstützt.
practice-banner = ÜBUNG · Spiel { $number } der aufgenommenen Serie · Versuch { $attempt }
practice-isolation = Zählt nicht für die Spielliste, wird nicht gespeichert und ändert die Serie nicht.
practice-return = Zurück zur Serie
practice-restart = Neuer Versuch
practice-finished = Übungsspiel beendet. Oben zurück zur Serie oder einen neuen Versuch starten.
practice-not-scored = Übung: kein Eintrag in die Spielliste. Die Spielliste der Serie bleibt unverändert.
practice-limit = Höchstzahl an Versuchen erreicht. Zurück zur Serie und die Aufnahme erneut öffnen.
replay-coach-title = Frühere Entscheidung kommentieren
replay-coach-help = Wähle eine deiner aufgenommenen Entscheidungen. Ein Kommentar wird nur auf Anfrage berechnet, immer nur einer und getrennt von der laufenden Serie.
replay-coach-decision = Entscheidung
replay-coach-none-selected = Keine Entscheidung gewählt
replay-coach-option = Aktion { $number } · { $when } · { $action }
replay-coach-forced-option = { $label } (einzige Möglichkeit)
replay-coach-no-decisions = Diese Aufnahme enthält noch keine Entscheidung von dir. Es gibt nichts zu kommentieren.
replay-coach-perspective-historical = Sicht: damaliger Wissensstand. Nur, was du vor diesem Zug sehen konntest; verdeckte Karten werden geschätzt.
replay-coach-perspective-hindsight = RÜCKBLICK: alle Karten bekannt. Modellvergleich im tatsächlichen Blatt mit damals unbekannten Karten; kein Beweis, dass dein Zug damals ein Fehler war.
replay-coach-request = Kommentar anfordern
replay-coach-retry = Erneut berechnen
replay-coach-hindsight = Rückblick: mit allen Karten vergleichen
replay-coach-historical = Zurück zum damaligen Wissensstand
replay-coach-cancel = Berechnung abbrechen
replay-coach-computing = Kommentar wird berechnet …
replay-coach-idle = Noch nicht berechnet. Der Kommentar startet erst auf Anfrage.
replay-coach-before = Der Tisch vor deinem aufgenommenen Zug
replay-coach-alternative = Verglichen mit { $action }: Modellbewertung { $judgement }.
replay-coach-forced = Du hattest nur { $action }. Ohne Alternative gibt es keinen Vergleich.
replay-coach-fallback = Zu dieser Entscheidung passt keine geprüfte Lektion. Es gibt nur den Modellvergleich, keine strategische Begründung.
replay-coach-provisional = Vorläufig: Die Analyse wurde nicht vollständig. Kleine Unterschiede entscheiden hier nichts.
replay-coach-basis-known = Berechnet im tatsächlichen Blatt dieser Aufnahme: alle Karten bekannt (Rückblick).
record-reveal-note = Aufdecken ändert nur die Anzeige, nicht die Sicht des Kommentars.
