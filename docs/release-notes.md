# Release notes: 1.0 candidate

Status: draft for the 1.0 candidate. The workspace version is 1.0.0-rc.1; no
tag, release or upload exists, and
[packaging](packaging.md) describes only private RC archives. Statements below
repeat existing documents and add no new evidence.

## Scope

Grand-Coach is an interactive Skat coach: full games and series against
Classic opponents, live coach and counting aid, recorded-game replay and
post-game review, and a local learning history with focused counting training
([roadmap](roadmap.md)). Optional Neural and Hybrid opponents play cards with
the shipped model ([learned models](learned-models.md#supported-phases-levels-and-limits)).

## Platforms

| Platform | Package | Evidence | Source |
|----------|---------|----------|--------|
| x86_64 Linux | portable `.tar.gz` | Ubuntu 24.04 CI build and smoke, also in a network-less runtime container | [packaging](packaging.md#formats-and-environments) |
| x86_64 Windows 11 | portable `.zip` | Windows Server 2025 CI build and smoke with a declared Mesa software OpenGL driver | [packaging](packaging.md#formats-and-environments) |

Packaged-smoke evidence for the history-format-7 candidate (commit `4875d969714a`):
[RC run 45737075480](https://github.com/naturelle137/grand-coach/actions/runs/45737075480),
Linux archive SHA-256 `8fac240e…6cdec`, Windows archive SHA-256 `b454df7b…e045`;
both smokes passed (complete series, open table, replay/review, practice,
coach/counting/quiz, training, all card faces, German and English).

Successful Windows CI is the Windows verification (D-36); native Windows 11
hardware, hardware acceleration and a clean OS install are not claimed. No
other platform is verified.

## Install, data and upgrade

Install, launch, upgrade and removal steps are in
[packaging](packaging.md#install-launch-upgrade-and-remove). In short: verify
the checksum, extract into a user-owned directory and run the executable.
History (`learning-history.json`) lives outside the installation, in
`$XDG_DATA_HOME/grand-coach` or `~/.local/share/grand-coach` on Linux and
`%APPDATA%\Grand-Coach` on Windows (`GRAND_COACH_DATA` overrides it).
Back up before upgrading; older history formats migrate on the next save.
Uninstalling keeps history and recordings; delete them separately to remove
personal data.

## Candidate evidence

Dispatched on `main` at commit `676bb65a95839fa1dbd593f34ff01ff4077891ab`
(workflow runs of 2026-11-26; later commits are not covered):

| Evidence | Result |
|----------|--------|
| [CI run 45742979009](https://github.com/naturelle137/grand-coach/actions/runs/45742979009) | `complete` suites on Ubuntu and Windows, `native`, both `screenshots` jobs, `msrv`, `portable`, `lint` and `ci-ok` succeeded; `regression` and `deep` were skipped |
| [RC packages run 45742982522](https://github.com/naturelle137/grand-coach/actions/runs/45742982522) | Ubuntu 24.04 and Windows 2025 jobs succeeded |
| Linux archive `grand-coach-0.1.0-rc-676bb65a9583-x86_64-unknown-linux-gnu.tar.gz` | SHA-256 `59a5ce7cbb24328ee31057569fe888abdc69c397c503a58fa7d8c9521ca1a69a`; `smoke.json` result `passed`, also in the clean-runtime container; network isolation not verified |
| Windows archive `grand-coach-0.1.0-rc-676bb65a9583-x86_64-pc-windows-msvc.zip` | SHA-256 `f297b396644620bd63b160ce8871a4a9a934510a209206569df8f3dc17b7c323`; `smoke.json` result `passed` with program-scoped outbound Firewall block verified by a curl canary |

The manifests were written before the smoke and still list `native-smoke` and
`network-isolation` as `not run`; the smoke results above come from the
`smoke.json` files of the same runs. The CI `complete` jobs did not run the
`regression` or `deep` tiers.

Not evidenced by these runs, and therefore unclaimed: interactive play and
widget-click flows of the packages, a clean OS or Windows 11 install,
target-hardware latency and RSS, learner comprehension, learning efficacy and
transfer, and the acceptance review of [modern egui #134](https://github.com/naturelle137/grand-coach/issues/134),
which was not assessed here. The release test plan rows on presentation,
live coach, Live Abstich, lessons/review, opponents and full game flows
remain open beyond what the smoke and the `complete`/screenshot jobs cover.

## Known limitations

- **Learning is unobserved.** Human comprehension, learning efficacy and
  transfer are not established; learner research is optional and unperformed
  ([release test plan](release-test-plan.md), [validation54](validation54.md)).
- **Selected review moments are not error rates.** The learning history
  classifies selected review moments, but ordinary-game risk/error
  prevalence remains unsupported ([roadmap](roadmap.md)); live-Abstich counts cannot estimate it
  ([live Abstich](live-abstich.md)).
- **Neural engines are optional and experimental.** Classic is the default;
  a missing or incompatible model falls back to Classic. The model covers
  card play only; strength ordering and Classic equivalence are not
  established ([learned models](learned-models.md#supported-phases-levels-and-limits),
  [AI](ai.md)).
- **Target-hardware performance is unmeasured.** No latency, RSS or package
  size result exists for the six-core integrated-graphics target
  ([packaging](packaging.md)).
- **Platform evidence is CI-server evidence.** Smoke checks drive app
  commands and controllers rather than clicking widgets, use short series
  where noted, and are not interactive play; Linux packages are dynamically
  linked with a glibc 2.39 baseline ([packaging](packaging.md)).
- **Unsigned private RCs.** Checksums identify bytes but do not authenticate
  a release ([packaging](packaging.md)).
- **No coordination between instances.** Two simultaneous instances share
  the history file and the last save wins ([packaging](packaging.md)).

## Zusammenfassung (DE)

Dies ist der Entwurf der Release Notes für den 1.0-Kandidaten; Version,
Tags und Veröffentlichung bleiben unverändert. Getestet sind Linux
(Ubuntu 24.04 CI) und Windows (Server 2025 CI mit Software-OpenGL); andere
Plattformen und echte Windows-11-Hardware gelten nicht als verifiziert.
Installation, Datenort und Upgrade stehen in [packaging](packaging.md).
Bekannte Grenzen: kein Nachweis für Lernwirkung oder Transfer, ausgewählte
Review-Momente ergeben keine Fehlerhäufigkeit, neuronale Gegner optional und
experimentell mit Classic-Rückfall, keine Messung auf der Zielhardware.
