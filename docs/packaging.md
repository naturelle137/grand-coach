# Private RC packages

These packages are candidates for technical validation of #13, not a public
release or evidence of 1.0 readiness. The workspace version remains 0.1.0.
The archive name adds `rc`, the source commit and native target. No release,
tag, installer, administrator access, updater or network service is created.

## Formats and environments

| Target | Package | Build environment | Runtime prerequisites |
|--------|---------|-------------------|-----------------------|
| x86_64 Linux | portable `.tar.gz` | Ubuntu 24.04 CI; Fedora 44 local development | glibc compatible with the build host, libgcc, EGL/OpenGL driver, Wayland or X11 display, libxkbcommon and its X11 extension for X11; see manifest `ldd` output |
| x86_64 Windows | portable `.zip` | Windows Server 2025 CI, Rust MSVC and Visual Studio C++ Build Tools | Windows 11 x64, graphics driver supporting OpenGL 2.0+, Microsoft Visual C++ runtime when required by the toolchain |

Linux binaries are dynamically linked, not universal/static Linux binaries.
An Ubuntu build has a glibc 2.39 baseline; a Fedora-built artifact must not
be represented as having the Ubuntu baseline. EGL/GL and display libraries
can also be loaded dynamically and are not all visible in `ldd`. Xvfb plus
Mesa and `libxkbcommon-x11-0` are installed for the Ubuntu smoke (Fedora:
`libxkbcommon-x11`). A missing X11 extension causes a startup panic even when
the executable's direct `ldd` dependencies are satisfied.
Windows Server CI is not interactive Windows 11.
Its built-in OpenGL 1.1 fails the app's OpenGL 2.0+ requirement. CI therefore
downloads the MIT-licensed [mesa-dist-win 26.2.6 MSVC build](https://github.com/pal1000/mesa-dist-win/releases/tag/26.2.6),
checks SHA-256 `b2e00615feaba45942343c265cc8c5f576142ca682c1ac6661922d4b72492877`,
and passes its x64 DLL directory to `smoke --windows-mesa <directory>`.
Only the temporary test installation receives `opengl32.dll` and
`libgallium_wgl.dll`, with `GALLIUM_DRIVER=llvmpipe`. Their hashes are recorded
in `smoke.json`; they are never added to the application ZIP or distributed
with its notices. This verifies a declared software-driver prerequisite on
the server, not native Windows 11 hardware acceleration or a clean OS.
An installed runtime may satisfy prerequisites on CI without proving a clean
machine has them. Record and check these prerequisites on the target machine.

Cards (all three faces, God/Mephisto styles and backs), 24 portraits and de/en
Fluent translations are compiled into the executable by `gui-egui`. Copies
are included as an auditable inventory, not runtime lookup dependencies.
The archive also carries EUPL, CC BY-SA, dependency notices, Cargo.lock and
the third-party asset inventory. No model is shipped: Classic is the default
offline engine; selecting Neural without a valid model falls back to Strong.
The development executable can still find models in its original checkout;
do not mistake that for a shipped model. Optional model distribution requires
a separately verified manifest/hash and runtime/fallback evidence.

## Reproduce and inspect

Use a clean committed checkout, Python 3.11+, native Rust 1.95.0, Cargo and
a native linker (GCC on Linux; MSVC C++ Build Tools on Windows). Fetch locked
dependencies before disconnecting. The application itself needs no network.

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
   it using the same data location. Supported history formats 1/2 migrate
   losslessly to current format 3; corrupt/unsupported files are preserved
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
Probe creation/checks are API evidence, not GUI entry/answer evidence. They do
not establish migration, cross-version upgrade or complete-series behaviour.
Existing application/widget tests cover those behaviours separately.

Before release, execute the [release test plan](release-test-plan.md) on clean
Linux and actual Windows 11, with the exact artifact hash: valid history across
restart and a supported-version upgrade, all faces/languages, full series,
coach/counting/quiz, open display, recording/replay/post-game analysis and
training. Record prerequisites installed, install/uninstall observations,
hardware, build, commands, seeds, screenshots and findings. Actual learner
observations remain a separate required gate. Missing evidence stays pending;
neither server CI nor a cross-build establishes these claims.
