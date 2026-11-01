# Grand Coach

[Deutsch](#deutsch) · [English](#english)

## Deutsch

### Über das Projekt

Skat-Trainer für PC (Linux und Windows 11): vollständiges Skat nach
Internationaler Skatordnung gegen zwei Computergegner. Dazu kommt ein
abschaltbarer Live-Coach, der den **Denkweg** erklärt, nicht nur den besten
Zug. Schwerpunkte sind Kartenzählen und das Hineinversetzen in die Gegner.

### Funktionen (vorhanden und geplant)

- Skat nach ISkO, optional mit Kontra/Re, Wertung nach Seeger-Fabian
- Gegnerstärke in Stufen (Grünschnabel, Leicht, Mittel, Stark) sowie die
  Modi Goldfisch und Mephisto
- Live-Coach in Stufen: aus / auf Anfrage / wichtige Entscheidungen / jeder Zug
- Kartenzählhilfe und Quiz
- Offener Modus, Replay, Analyse nach dem Spiel
- Umschaltbare Kartenbilder (französisch vier- und zweifarbig, deutsches
  Blatt); Oberfläche auf Deutsch und Englisch

Details: [docs/vision.md](docs/vision.md).

### Status

Spielbare Version: eine Serie gegen zwei Computergegner, deren Stärke
(Grünschnabel, Leicht, Mittel, Stark) einzeln wählbar ist, oder gegen
Goldfisch oder Mephisto, mit Kontra/Re, drei
Kartenbildern und deutscher oder englischer Oberfläche. Unter Linux
getestet, Windows 11 noch nicht.
Der Live-Coach mit gestuften Hinweisen, sechs belegten Lektionen, Lernfokus
und Rückblick auf frühere Entscheidungen ist implementiert. Kartenzählhilfe,
Quiz, gespeicherte Replays und Trainingsmodus folgen noch. Manuelle
Coach- und Release-Validierung stehen aus; sie werden vor Release 1.0 in
[#81](https://github.com/naturelle137/grand-coach/issues/81) nachgeholt.
Fortschritt und nächster Schritt (#68): [docs/roadmap.md](docs/roadmap.md).

### Systemvoraussetzungen

Linux (Wayland oder X11) mit OpenGL bzw. EGL; Windows 11 ist das zweite
Zielsystem, aber noch nicht getestet.

### Installation

*Noch offen. Es gibt noch keine Releases.*

### Aus dem Quellcode bauen

Benötigt Rust (stable, ab 1.95). Im Wurzelverzeichnis:

```sh
cargo build --release
cargo test
```

Weitere Prüfungen stehen in [CONTRIBUTING.md](CONTRIBUTING.md).

### Benutzung

```sh
cargo run --release -p gui-egui
```

Du spielst unten, die Gegner sitzen links und rechts. Beim Start öffnet sich
„Neue Serie“ mit den Einstellungen der ersten Serie; die Gegner warten, bis
du startest oder abbrichst. Über „Spiel → Neue Serie …“ wählst du feste Spielzahl (z. B. 36) oder freies Spiel, Kontra/Re
und die Gegner (Stärke je Gegner, Goldfisch oder Mephisto); unter
„Einstellungen“ Sprache und Kartenbild. Die Entwickleroption „Seed anzeigen“
zeigt den Startwert der Serie und macht ihn in „Neue Serie …“ einstellbar
(ganze Zahl von 0 bis 18446744073709551615; gleicher Seed, gleiche Karten).
Rechts stehen der letzte Stich und die Spielliste (Seeger-Fabian).

### Mitwirken

Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den
[Verhaltenskodex](CODE_OF_CONDUCT.md).

### Lizenz

- **Code**: [EUPL-1.2](LICENSE). Quelldateien tragen die Kopfzeile
  `SPDX-License-Identifier: EUPL-1.2`.
- **Grafiken, Dokumentation und sonstige Inhalte** (`assets/`, `docs/`,
  Texte): [CC BY-SA 4.0](LICENSE-ASSETS).
- **Fremde Dateien und Bibliotheken** nur mit kompatibler Lizenz; Herkunft und
  Lizenz stehen in [docs/third-party.md](docs/third-party.md), Lizenztexte und
  Copyright-Hinweise der Bibliotheken in
  [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).

## English

### About

A Skat trainer for PC (Linux and Windows 11): full Skat according to the
International Skat Rules against two computer opponents, plus an optional live
coach that explains the **line of thought** rather than just the best move.
Focus: card counting and putting yourself in the opponents' shoes.

### Features (available and planned)

- Skat per ISkO, optional Kontra/Re, Seeger-Fabian scoring
- Opponent levels (Rookie, Easy, Medium, Strong) and the modes Goldfish
  and Mephisto
- Live coach with levels: off / on request / important decisions / every move
- Card-counting aid and quiz
- Open mode, replay, post-game analysis
- Switchable card faces (French four- and two-colour, German deck); UI in
  German and English

Details: [docs/vision.md](docs/vision.md).

### Status

Playable version: a series against two computer opponents whose strength
(Rookie, Easy, Medium, Strong) is chosen separately, or against Goldfish or
Mephisto, with Kontra/Re, three card
faces and a German or English UI. Tested on Linux, not yet on Windows 11.
The live coach with staged hints, six evidence-backed lessons, learning
focus and historical decision reviews is implemented. Counting aid, quiz,
saved replays and training mode remain planned. Manual coach and release
validation is pending in [#81](https://github.com/naturelle137/grand-coach/issues/81)
before release 1.0.
Progress and next step (#68): [docs/roadmap.md](docs/roadmap.md).

### Requirements

Linux (Wayland or X11) with OpenGL or EGL; Windows 11 is the second target
platform but has not been tested yet.

### Installation

*To be defined. No releases yet.*

### Building from source

Requires Rust (stable, 1.95 or later). From the repository root:

```sh
cargo build --release
cargo test
```

Further checks are listed in [CONTRIBUTING.md](CONTRIBUTING.md).

### Usage

```sh
cargo run --release -p gui-egui
```

You play at the bottom; the opponents sit left and right. At launch, "New
series" opens with the settings of the first series; the opponents wait
until you start or cancel. "Game → New series …" chooses a fixed number of games (e.g. 36) or free play,
Kontra/Re and the opponents (a level for each, Goldfish or Mephisto);
"Settings" switches language and card faces. The developer option "Show
seed" shows the series seed and lets "New series …" set it (a whole number
from 0 to 18446744073709551615; same seed, same cards). The right-hand side
shows the last trick and the score sheet (Seeger-Fabian).

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md).

### Licence

- **Code**: [EUPL-1.2](LICENSE). Source files carry the header
  `SPDX-License-Identifier: EUPL-1.2`.
- **Graphics, documentation and other content** (`assets/`, `docs/`, texts):
  [CC BY-SA 4.0](LICENSE-ASSETS).
- **Third-party files and libraries** only with a compatible licence; origin
  and licence are listed in [docs/third-party.md](docs/third-party.md), the
  libraries' licence texts and copyright notices in
  [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).
