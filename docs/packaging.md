# Private RC packages

These packages are candidates for technical validation of #13, not a public
release or evidence of 1.0 readiness. The workspace version remains 0.1.0.
The archive name adds `rc`, the source commit and native target. No release,
tag, installer, administrator access, updater or network service is created.

## Formats and environments

| Target | Package | Build environment | Runtime prerequisites |
|--------|---------|-------------------|-----------------------|
| x86_64 Linux | portable `.tar.gz` | Ubuntu 24.04 CI; Fedora 44 local development | glibc compatible with the build host, libgcc, EGL/OpenGL driver, Wayland or X11 display, libxkbcommon; X11 also needs Xlib, X11-XCB, Xcursor, Xi and xkbcommon's X11 extension; see manifest `ldd` output |
| x86_64 Windows | portable `.zip` | Windows Server 2025 CI, Rust MSVC and Visual Studio C++ Build Tools | Windows 11 x64, graphics driver supporting OpenGL 2.0+, Microsoft Visual C++ v14 x64 runtime (`VCRUNTIME140.dll`) and Windows UCRT |

Linux binaries are dynamically linked, not universal/static Linux binaries.
An Ubuntu build has a glibc 2.39 baseline; a Fedora-built artifact must not
be represented as having the Ubuntu baseline. EGL/GL and display libraries
can also be loaded dynamically and are not all visible in `ldd`. Xvfb plus
Mesa and `libxkbcommon-x11-0` are installed for the Ubuntu smoke (Fedora:
`libxkbcommon-x11`). X11 also dynamically loads Xlib, X11-XCB, Xcursor and Xi.
Ubuntu runtime packages are `libx11-6 libx11-xcb1 libxcursor1 libxi6`; missing
dynamic libraries prevent startup even when direct `ldd` dependencies pass.
Windows Server CI is not interactive Windows 11.
Its built-in OpenGL 1.1 fails the app's OpenGL 2.0+ requirement. CI therefore
downloads the [mesa-dist-win 26.2.6 MinGW x64 build](https://github.com/pal1000/mesa-dist-win/releases/tag/26.2.6),
checks SHA-256 `da0f71d9181b672c1918f3c342f11118c687f6059a46b16502845160733cc388`,
and passes its x64 DLL directory to `smoke --windows-mesa <directory>`.
Only the temporary test installation receives `opengl32.dll` and
`libgallium_wgl.dll`, with `GALLIUM_DRIVER=llvmpipe`. Their hashes are recorded
in `smoke.json`; they are never added to the application ZIP or distributed
with its notices. This verifies a declared software-driver prerequisite on
the server, not native Windows 11 hardware acceleration or a clean OS.
The upstream release documents a missing-OpenGL regression in its MSVC and
32-bit MinGW distributions; the inspected x64 MinGW distribution is used.
An installed runtime may satisfy prerequisites on CI without proving a clean
machine has them. Record and check these prerequisites on the target machine.

Cards (all three faces, God/Mephisto styles and backs), 24 portraits and de/en
Fluent translations are compiled into the executable by `gui-egui`. Copies
are included as an auditable inventory, not runtime lookup dependencies.
The archive also carries EUPL, CC BY-SA, dependency notices, Cargo.lock and
the third-party asset inventory. The archive ships the card-play model
(`assets/models/card-play.gcnn`, its training manifest and model card, D-63);
keep its `assets/models` directory beside the executable; the app
searches first after `GRAND_COACH_MODELS`. Build and `verify` fail unless the
model matches the SHA-256 and size in its manifest. They also require the manifest's recipe, engine commit, campaign, selection, supervised start, source-provenance hashes and CC-BY-SA-4.0 licence, and a model card with provenance, limits and licence sections. These checks establish that metadata is present; the independent verification of retained training records and dataset shards is documented in the model card. Classic stays the default;
a missing or corrupt model makes Neural and Hybrid fall back to the corresponding Classic level without
touching user data. The smoke test launches with the model installed and again
with a corrupted one, checking valid history is unchanged. A same-build app API probe beside the executable checks Neural and Hybrid card-play decisions with the shipped model, and fallback for missing, corrupt and incompatible format/encoder models for both; these decisions are not GUI interactions. The Hybrid fallback probes compare decisions through the first card play with Classic Easy. The incompatible files have their manifest hashes updated so the probe checks the version rejection rather than a hash mismatch. The shipped model covers card play only,
so other phases (bidding, Skat, Kontra) stay Classic. Neural levels stay
experimental; no target-machine latency/RSS measurement is recorded yet (no
six-core integrated-graphics hardware available to agents), and results from
other machines are only proxies.

The Neural probe disables Kontra/Re and requires the observed decision or
fallback to occur in card play; a pre-play fallback or rule-only Kontra
decision cannot satisfy that check.

Local offline check (2026-11-19, Fedora 44, Rust 1.99.1 build identity, commit
79d6255): the Linux package built from a clean checkout was extracted and the
same-build probe ran in a network-less namespace (`unshare -rn`, only `lo`):
`neural` verified an actual decision from the shipped model; corrupt-model,
format-incompatible and encoder-incompatible files each fell back to Classic
Strong (`fallback`, `format`, `encoder`: all exit 0). The GUI smoke could not
run in that sandbox (`XOpenDisplayFailed`, also without the namespace), so GUI
launch evidence stays with the CI runtime container (`--network=none`) and the
Windows job. The six-core target machine is still unavailable: no latency,
RSS or package-size result for it exists.

Packaged coach/counting/quiz and training smoke (2026-11-19, workflow
[RC packages run 44565081576](https://github.com/naturelle137/grand-coach/actions/runs/44565081576),
commit `a59aacdbaaea`, `gh workflow run package-rc.yml --ref <branch>`): both jobs
passed all smoke checks on the exact archives
Linux `22f75d88…c7eb8` (Ubuntu 24.04, also in the `--network=none` runtime
container) and Windows `632b51d2…7d79c` (Server 2025, declared Mesa software
driver). Each launched the extracted executable four more times
(`coach`/`training` × `de`/`en`); the eight native screenshots were inspected
at reduced size: the German and English coach views show the open
counting question over the table, and the training views show the focus and
level choices in the matching language. The four screenshot hashes per
platform differ and the history stayed valid. No defect was observed. This is
CI-server rendering at 1280 × 820 (Linux) and 1024 × 768 (Windows), not
interactive play, native Windows 11 or hardware-accelerated evidence;
answering, skipping and retrying remain widget-test coverage.

Packaged series, open-table and recording-viewer smoke (2026-11-19, workflow
[RC packages run 44584249665](https://github.com/naturelle137/grand-coach/actions/runs/44584249665),
commit `ff3d0564789a`): both jobs passed on the exact archives Linux
`fb0b64e985c4…` (Ubuntu 24.04, also in the clean runtime container) and
Windows `3d780f2ef912…` (Server 2025). Per platform the extracted executable
was launched ten times (`coach`, `training`, `series`, `open`, `replay` ×
`de`/`en`); all ten screenshot hashes differ and the history stayed valid. The
Windows English `open` and `replay` and the German/English Linux `series`
screenshots were inspected: the open table shows both opponents' hands and the
Skat, the viewer shows the saved first game read-only, and the series ends
with the series result. Limits: the series has two games, not a full
36-game series; replay review and practice buttons were not clicked; CI-server
rendering, not Windows 11 hardware. Strategic classification/trends,
release notes and the closing evidence of #13 remain open.

## Reproduce and inspect

Use a clean committed checkout, Python 3.11+, native Rust 1.95.0, Cargo and
a native linker (GCC on Linux; MSVC C++ Build Tools on Windows). Fetch locked
dependencies before disconnecting. The Windows smoke does not verify network isolation or prove offline operation; Linux CI separately runs the smoke in a network-less container.
Windows Server 2025's native package smoke passed again in
[RC run 44605651765](https://github.com/naturelle137/grand-coach/actions/runs/44605651765)
on commit `32f1f729a360`; this run did not block network access.

```sh
cargo fetch --locked
python3 -B scripts/package_test.py
python3 -B scripts/package.py build --out /absolute/path/to/rc-output
python3 -B scripts/package.py verify /absolute/path/to/package.tar.gz
python3 -B scripts/package.py smoke /absolute/path/to/package.tar.gz --out /absolute/path/to/evidence
```

Use the `.zip` filename on Windows (Python launcher `py -3` is an alternative
to `python3`). For an isolated Linux smoke, prefix the command with
`env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1600x1000x24'`.
Unsetting inherited Wayland variables ensures the app uses this X11 display;
the native Wayland screenshot path is not established by the Xvfb evidence.
Fedora's native Rust 1.99.1 can be used with `build --rust-version 1.99.1`;
that is a distinct build identity, not a reproduction of the CI binary.
`build` invokes `cargo build --locked --offline --release -p gui-egui --bin
grand-coach --target <native-host>`, with default features and no GPU backend.
It rejects a dirty checkout, missing/empty resources and stale notice inventory.

The manifest pins the source commit, version, target, lockfile hash, exact
rustc/Cargo identity, build command, feature policy, OS/runner image, build
environment, resource list and per-file SHA-256 values. Linux records `ldd`.
The adjacent `.sha256` identifies the entire archive. The adjacent
`persistence-probe` (`.exe` on Windows) is a test helper linked to the same
skat-app artifacts; it is not inside the application archive or required to
play. Keep it alongside the archive when running `smoke`. Verify the archive independently
with `sha256sum` or PowerShell `Get-FileHash -Algorithm SHA256`. Checksums
identify bytes; these are unsigned private RCs, not authenticated releases.

Archive entry order, timestamps, ownership and modes are normalized. Each
build creates the archive twice and requires identical bytes from identical
payloads. Binary bytes are not claimed reproducible across hosts, compilers,
SDKs, linkers or checkout paths; the manifest makes those builds distinguishable.
Reproduction of a binary requires the recorded build environment in addition
to the locked Rust sources. Do not equate deterministic archive creation with
an independently demonstrated bit-identical compiler/linker rebuild.

The dedicated **RC packages** workflow accepts manual dispatch on the branch
and builds both native targets. It uploads archives, checksums, native launch
screenshots/logs and `smoke.json` for 14 days; it never publishes a release.
Record the exact run URL and head in the PR/handoff. Existing `ci.yml` and its
complete-platform release gate remain authoritative and unchanged.

Linux CI additionally builds the pinned-base `scripts/package-runtime.Dockerfile`
and repeats the smoke with runtime libraries only, no Rust/Cargo or source
checkout, and `--network=none`. Only the archive/probe and the single Python
smoke script are mounted read-only. An init process lets Xvfb signal readiness;
image metadata, OS release, exact package versions and smoke evidence are saved.
This fresh runtime-container check catches prerequisites already installed on
build runners. It is not a clean OS installation or an interactive desktop test.

## Install, launch, upgrade and remove

1. Verify the checksum; extract the archive into a new user-owned directory.
2. Run `./grand-coach` on Linux or double-click `grand-coach.exe` on Windows.
   Start from any working directory. No Cargo, Python or source checkout is
   needed to play. The executable can run alone; retain legal files when
   distributing any copy.
3. History lives in `learning-history.json`, outside the installation:
   `$XDG_DATA_HOME/grand-coach` (absolute XDG path), otherwise
   `~/.local/share/grand-coach` on Linux; `%APPDATA%\Grand-Coach` on Windows.
   `GRAND_COACH_DATA` overrides the directory. Recordings are saved to paths
   deliberately selected by the player. No directory means in-memory history.
4. Close all instances before upgrade. Back up the data directory and saved
   recordings; extract the new package into a separate directory and launch
   it using the same data location. The packaged commit defines the supported
   older history formats: these migrate in memory, and the next save writes
   that build's current format. Corrupt/unsupported files are preserved
   and require deliberate recovery. Do not promise backward compatibility
   after a newer build has saved a new format. Two simultaneous instances
   have no coordination (last save wins).
5. Uninstall by deleting the extracted installation and any manually created
   shortcut. This retains history and player-selected recordings. Delete
   those separately only when deliberately removing personal data.

## Automated evidence and remaining release checks

The archive tests check both formats, deterministic ordering/bytes, inventory,
tampering, extra/unsafe files, missing cards/translations/notices and stale
notices. The packaged smoke validates hashes, launches only the extracted
executable from an unrelated directory (plus explicitly declared test driver
DLLs on Windows CI), captures a native screenshot, restarts,
replaces the executable with the same build and removes the install directory.
The linked probe creates a valid history from actual training events through
the existing app API, saves/reopens it and checks it after each packaged launch.
The smoke requires byte-identical valid history across restart/replacement,
then separately tests damaged-history preservation and uninstall retention.
After these stages the same extracted executable is launched twelve more times
with smoke-only variables `GRAND_COACH_SMOKE_SCENARIO=coach|training|series|open|replay|review` and
`GRAND_COACH_LANGUAGE=de|en` (with valid history restored): `coach` enables
EveryMove coach, counting aid and quiz and plays first legal actions until a
counting question is open; `training` opens the focused counting training;
`series` plays a 36-game series with first legal actions to the series
result; `open` plays an open-table game (all hands shown) for 12 actions;
`replay` plays one game of a two-game series, saves it to the data directory,
reopens it and shows it in the read-only recording viewer; `review` does the same, requests
the post-game recording review and waits for its summary (analysis through the shared
replay coach, bounded by its time limit). Each saves a native
screenshot; the twelve must differ and history must stay valid (byte-identical for
training; played scenarios may record learning observations). These show the real packaged views in both languages; they
do not answer, skip or retry (widget tests cover that).
Probe creation/checks are API evidence, not GUI entry/answer evidence. They do
not establish migration or cross-version upgrade.
Existing application/widget tests cover those behaviours separately.

Before release, agents execute the required functional, package and visual
checks in the [release test plan](release-test-plan.md), with the exact artifact
hash: valid history across restart and a supported-version upgrade, all
faces/languages, full series, coach/counting/quiz, open display,
recording/replay/post-game analysis and training. Record prerequisites,
install/uninstall results, environment, build, commands, seeds, inspected
screenshots and findings. Successful Windows CI suffices for Windows verification;
no separate Windows device, clean Windows 11 session or CEO acceptance is a
prerequisite. Keep the CI environment limits above explicit. Learner research is
optional and absent results limit comprehension, efficacy and transfer claims,
without blocking development or release. No release/publication is authorised
by these procedures.
