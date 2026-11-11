# Grand Coach

[Deutsch](#deutsch) · [English](#english)

## Deutsch

### Über das Projekt

Grand-Coach soll dir helfen, am Skattisch selbst wie ein Könner zu denken.
Im Spiel gegen zwei Computergegner begleitet dich ein interaktiver Live-Coach.
Er lenkt deinen Blick auf das, was für deinen nächsten Zug zählt. Wer konnte
Kreuz nicht bedienen? Welche Trümpfe sind noch draußen? Im Rückblick kannst
du nachvollziehen, wann dein Kreuz-Ass in Gefahr war und woran du das schon
vor dem Ausspiel erkennen konntest.

Geplant sind gezielte Trainingseinheiten, etwa zur Trumpfführung oder zum
Zusammenspiel im Gegenspiel. Du übst ein Thema an wechselnden Blättern,
bis du die entscheidende Situation auch am echten Tisch selbst erkennst.
[Produktvision und Lernkonzept](docs/vision.md)

### Spielumfang

Spiele eine feste Serie oder frei, mit optionalem Kontra/Re und Wertung nach
Seeger-Fabian. Wähle die Gegnerstärke je Sitz oder einen Sondertisch:
Goldfisch, Mephisto, God Mode, Experimentell oder KI-Netz (Test). Im God Mode
siehst du auch die verdeckten Karten. Der Coach bietet gestufte Hinweise,
sechs Lektionen, einen Lernfokus und einen Rückblick auf bis zu drei
Entscheidungen. Eine optionale Zählhilfe und Zählfragen im Spiel zeigen
exakte Fakten aus deiner Sicht. Oberfläche und Kartenbilder sind umschaltbar.

Der KI-Netz-Tisch verwendet das mitgelieferte überwacht trainierte v1-Modell.
v2 befindet sich im Reinforcement Learning. Abrufübungen mit abnehmender
Hilfe, gespeicherte Replays und Trainingsmodus sind geplant.
[Projektstand](docs/roadmap.md) · [Modelle und Training](docs/learned-models.md)

### Systemvoraussetzungen

Linux (Wayland oder X11) mit OpenGL bzw. EGL; Windows 11 ist das zweite
Zielsystem, mit Build und Headless-Tests in CI. Der interaktive Test auf Windows 11 steht aus.

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
und den Gegnermodus; unter
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

Grand-Coach aims to help you think like an expert at the Skat table. An
interactive live coach guides your attention as you play against two computer
opponents. Who failed to follow Kreuz? Which trumps are still out? In the
review you can see when your Kreuz-Ass was at risk and what you could have
known before leading it.

Planned training sessions let you choose a theme such as managing trumps
or working with your partner in defence. You practise on different deals so
you can recognise the moment for yourself at a real table.
[Product vision and teaching approach](docs/vision.md)

### Playable features

Play a fixed series or free play with optional Kontra/Re and Seeger-Fabian
scoring. Set the strength per opponent or select Goldfish, Mephisto, God
Mode, Experimental or Neural net (test). God Mode reveals the concealed
cards. The coach offers staged hints, six lessons, a learning focus and a
review of up to three decisions. An optional counting aid and in-game
counting questions show exact facts from your view. Language and card faces
are switchable.

The neural table uses the bundled supervised v1 model. v2 is undergoing
reinforcement learning. Retrieval practice with fading assistance, saved
replays and training mode are planned. [Project status](docs/roadmap.md) ·
[Models and training](docs/learned-models.md)

### Requirements

Linux (Wayland or X11) with OpenGL or EGL; Windows 11 is the second target
platform. CI covers Windows builds and headless tests. Interactive Windows 11 testing is pending.

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
Kontra/Re and the opponents;
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
