#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Build, verify and smoke-test private RC archives using only the standard library."""

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
LEGAL = ("LICENSE", "LICENSE-ASSETS", "THIRD-PARTY-NOTICES", "Cargo.lock")
# Shipped beside the executable, where the app looks for `assets/models` (D-63).
MODELS = ("assets/models/card-play.gcnn", "assets/models/card-play.json", "assets/models/card-play.md")


def run(*args, cwd=ROOT, **kwargs):
    return subprocess.check_output(args, cwd=cwd, text=True, encoding="utf-8", **kwargs).strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def runner_image(env):
    # A plain copy of Windows os.environ has upper-case keys.
    values = {key.upper(): value for key, value in env.items()}
    return {key: values.get(key.upper(), "") for key in ("ImageOS", "ImageVersion")}


def resources(root):
    """Require every known card face, portrait and both embedded languages."""
    paths = [root / "assets/cards/back.svg"]
    for face in ("french2", "french4", "german"):
        # Every face contains all 32 cards in ordinary, God and Mephisto styles.
        for style in ("", "god/", "mephisto/"):
            directory = root / "assets/cards" / style / face
            cards = sorted(directory.glob("*.svg"))
            if len(cards) != 32:
                raise ValueError(f"expected 32 cards: {directory}, got {len(cards)}")
            paths.extend(cards)
    # Special table backs are embedded too.
    paths.extend((root / "assets/cards/god/back.svg", root / "assets/cards/mephisto/back.svg"))
    portraits = sorted((root / "assets/portraits").glob("*.svg"))
    if len(portraits) != 24:
        raise ValueError("expected 24 opponent portraits")
    paths.extend(portraits)
    paths.extend(root / f"assets/i18n/{lang}.ftl" for lang in ("de", "en"))
    for path in paths:
        if not path.is_file() or not path.read_bytes():
            raise ValueError(f"missing/empty resource: {path}")
    return sorted(path.relative_to(root).as_posix() for path in paths)


def models(root):
    """Require the model, its training manifest and card; the recorded hash must match."""
    data = {name: (root / name).read_bytes() for name in MODELS}
    if not all(data.values()):
        raise ValueError("missing/empty model file")
    recorded = json.loads(data[MODELS[1]])
    if recorded["model_sha256"] != digest(data[MODELS[0]]) or recorded["model_bytes"] != len(data[MODELS[0]]):
        raise ValueError("model differs from its training manifest")
    for key in ("recipe", "engine_commit", "campaign", "selection", "supervised_start", "source_provenance_sha256"):
        if not recorded.get(key):
            raise ValueError(f"training manifest lacks {key}")
    if recorded.get("licence") != "CC-BY-SA-4.0":
        raise ValueError("training manifest licence is not CC-BY-SA-4.0")
    card = data[MODELS[2]].decode("utf-8")
    if "## Recipe and provenance" not in card or "## Evidence and limits" not in card or "CC BY-SA 4.0" not in card:
        raise ValueError("model card lacks provenance, limits or licence")
    return list(MODELS)


def notices(root):
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    expected = {f"{p['name']} {p['version']}" for p in lock["package"] if "source" in p}
    text = (root / "THIRD-PARTY-NOTICES").read_text(encoding="utf-8")
    actual = set(re.findall(r"^== (.+) ==$", text, re.M))
    actual = {s for s in actual if not s.startswith("Appendix")}
    if expected != actual:
        raise ValueError("THIRD-PARTY-NOTICES does not match Cargo.lock")


def linux_libraries(binary):
    libraries = run("ldd", str(binary))
    if "not found" in libraries:
        raise ValueError("missing native runtime library")
    # ASLR addresses vary per invocation; retain names/paths, not addresses.
    return re.sub(r" \(0x[0-9a-f]+\)", "", libraries)


def archive_bytes(files, windows, epoch):
    """Stable ordering, modes, owners and timestamps, independent of staging paths."""
    output = io.BytesIO()
    if windows:
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for name, data in sorted(files.items()):
                info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                info.create_system = 3
                info.external_attr = 0o100644 << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, data)
    else:
        with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as archive:
                for name, data in sorted(files.items()):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o755 if name == "grand-coach" else 0o644
                    info.mtime = epoch
                    archive.addfile(info, io.BytesIO(data))
    return output.getvalue()


def read_archive(path, data=None):
    files = {}
    source = io.BytesIO(data) if data is not None else path
    if path.suffix == ".zip":
        with zipfile.ZipFile(source) as archive:
            entries = [(i.filename, archive.read(i)) for i in archive.infolist()]
    else:
        with tarfile.open(fileobj=source, mode="r:gz") if data is not None else tarfile.open(path, "r:gz") as archive:
            entries = []
            for entry in archive:
                if not entry.isfile():
                    raise ValueError("archive must contain regular files only")
                entries.append((entry.name, archive.extractfile(entry).read()))
    for name, data in entries:
        p = PurePosixPath(name)
        if p.is_absolute() or ".." in p.parts or "\\" in name or ":" in name or name in files:
            raise ValueError(f"unsafe/duplicate archive entry: {name}")
        files[name] = data
    return files


def verify(path, data=None):
    files = read_archive(path, data)
    manifest = json.loads(files.pop("manifest.json"))
    expected = manifest["files"]
    if {name: digest(data) for name, data in files.items()} != expected:
        raise ValueError("archive contents/hashes differ from manifest")
    required = {*LEGAL, "INSTALL.md", "THIRD-PARTY.md", manifest["binary"], *manifest["embedded_resources"], *manifest["models"]}
    if not required <= files.keys() or not all(files[name] for name in required):
        raise ValueError("missing/empty required package files")
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory).resolve()
        for name, data in files.items():
            dest = root / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
        if resources(root) != manifest["embedded_resources"]:
            raise ValueError("incomplete resource inventory")
        if models(root) != manifest["models"]:
            raise ValueError("incomplete model inventory")
        notices(root)
    return manifest, files


def build(args):
    if run("git", "status", "--porcelain", "--untracked-files=normal"):
        raise ValueError("build requires a clean committed worktree")
    identity = run("rustc", "-Vv")
    if identity.splitlines()[0].split()[1] != args.rust_version:
        raise ValueError(f"rustc must be {args.rust_version}; got {identity}")
    target = next(line.split(": ", 1)[1] for line in identity.splitlines() if line.startswith("host: "))
    if target not in ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"):
        raise ValueError(f"unsupported native target: {target}")
    windows = target.endswith("msvc")
    names = resources(ROOT)
    shipped = models(ROOT)
    notices(ROOT)
    sha = run("git", "rev-parse", "HEAD")
    epoch = int(run("git", "show", "-s", "--format=%ct", "HEAD"))
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    command = ["cargo", "build", "--locked", "--offline", "--release", "-p", "gui-egui", "--bin", "grand-coach", "--target", target, "--message-format=json-render-diagnostics"]
    env = dict(os.environ, SOURCE_DATE_EPOCH=str(epoch), CARGO_INCREMENTAL="0", GRAND_COACH_BUILD_COMMIT=sha)
    compiled = subprocess.run(command, cwd=ROOT, env=env, check=True, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    artifacts = [json.loads(line) for line in compiled.stdout.splitlines() if line.startswith("{")]
    app_lib = next(Path(name) for a in artifacts if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "skat_app" for name in a["filenames"] if name.endswith(".rlib"))
    binary = "grand-coach.exe" if windows else "grand-coach"
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    files = {binary: (target_dir / target / "release" / binary).read_bytes()}
    for name in (*LEGAL, *names, *shipped):
        files[name] = (ROOT / name).read_bytes()
    files["INSTALL.md"] = (ROOT / "docs/packaging.md").read_bytes().replace(
        b"(release-test-plan.md)",
        f"(https://github.com/naturelle137/grand-coach/blob/{sha}/docs/release-test-plan.md)".encode())
    files["THIRD-PARTY.md"] = (ROOT / "docs/third-party.md").read_bytes()
    manifest = {
        "schema": 1, "channel": "private-rc", "version": version,
        "commit": sha, "source_date_epoch": epoch, "target": target,
        "binary": binary, "rustc": identity, "cargo": run("cargo", "-V"),
        "os": platform.platform(), "python": platform.python_version(),
        "build_command": command, "features": "default", "models": shipped,
        "build_environment": {k: env.get(k, "") for k in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CC", "CXX", "CARGO_INCREMENTAL", "SOURCE_DATE_EPOCH", "GRAND_COACH_BUILD_COMMIT")},
        "runner_image": runner_image(env),
        "embedded_resources": names,
        "files": {name: digest(data) for name, data in sorted(files.items())},
    }
    if not windows:
        manifest["runtime_libraries"] = linux_libraries(target_dir / target / "release" / binary)
    files["manifest.json"] = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    args.out.mkdir(parents=True, exist_ok=True)
    probe = args.out / ("persistence-probe.exe" if windows else "persistence-probe")
    subprocess.run(["rustc", "--edition=2024", "--crate-name", "package_persistence", "--target", target,
                    str(ROOT / "scripts/package-persistence.rs"), "--extern", f"skat_app={app_lib}",
                    "-L", f"dependency={app_lib.parent}",
                    "-L", f"dependency={target_dir / 'release/deps'}", "-o", str(probe)], check=True, cwd=ROOT)
    suffix = ".zip" if windows else ".tar.gz"
    path = args.out / f"grand-coach-{version}-rc-{sha[:12]}-{target}{suffix}"
    data = archive_bytes(files, windows, epoch)
    path.write_bytes(data)
    if data != archive_bytes(files, windows, epoch):
        raise ValueError("archive reproducibility check failed")
    verify(path)
    path.with_name(path.name + ".sha256").write_text(f"{digest(data)}  {path.name}\n", encoding="utf-8", newline="\n")
    print(path.resolve())


# Packaged feature views, each launched in German and English.
SCENARIOS = ("coach", "training", "series", "open", "replay", "review", "practice", "cards")
PLAYED = ("coach", "training", "series", "open", "replay", "review", "practice")


CANARY_URL = "https://github.com/"


def canary_reaches_network(canary):
    result = subprocess.run([str(canary), "--silent", "--head", "--max-time", "15", "--output", os.devnull, CANARY_URL],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
    return result.returncode == 0


def block_windows_network(install, programs, names):
    """Block outbound traffic of the installed programs and prove the block with a canary.

    A copy of curl.exe in the install directory must reach the network before
    and fail after the rules exist, so the rules (not a dead network) caused it.
    """
    if platform.system() != "Windows":
        raise ValueError("--block-network is Windows-only; Linux uses a --network=none container")
    canary = install / "network-canary.exe"
    shutil.copy2(Path(os.environ["SystemRoot"]) / "System32" / "curl.exe", canary)
    if not canary_reaches_network(canary):
        raise ValueError("network canary cannot reach the network before blocking; no isolation evidence possible")
    for index, program in enumerate([canary, *programs]):
        name = f"grand-coach-smoke-block-{os.getpid()}-{index}"
        subprocess.run(["netsh", "advfirewall", "firewall", "add", "rule", f"name={name}", "dir=out", "action=block",
                        f"program={program}", "enable=yes", "profile=any"], check=True)
        names.append(name)
    if canary_reaches_network(canary):
        raise ValueError("network canary still reaches the network after blocking")


def unblock_windows_network(names):
    for name in names:
        subprocess.run(["netsh", "advfirewall", "firewall", "delete", "rule", f"name={name}"], stdout=subprocess.DEVNULL)


def smoke(args):
    names = []
    try:
        smoke_run(args, names)
    finally:
        unblock_windows_network(names)


def smoke_run(args, rules):
    manifest, files = verify(args.archive)
    args.out.mkdir(parents=True, exist_ok=True)
    runtime_dlls = {}
    with tempfile.TemporaryDirectory(prefix="grand-coach-package-") as directory:
        root = Path(directory).resolve()
        install = root / "install"
        install.mkdir()
        # Only the executable is required at runtime: resources are embedded.
        binary = install / manifest["binary"]
        binary.write_bytes(files[manifest["binary"]])
        binary.chmod(0o755)
        if args.windows_mesa:
            if platform.system() != "Windows":
                raise ValueError("--windows-mesa is a Windows-only test prerequisite")
            for name in ("opengl32.dll", "libgallium_wgl.dll"):
                payload = (args.windows_mesa / name).read_bytes()
                (install / name).write_bytes(payload)
                runtime_dlls[name] = digest(payload)
        for name in manifest["models"]:
            (install / name).parent.mkdir(parents=True, exist_ok=True)
            (install / name).write_bytes(files[name])
        data = root / "user-data"
        data.mkdir()
        history = data / "learning-history.json"
        probe = (args.archive.parent / ("persistence-probe.exe" if manifest["binary"].endswith(".exe") else "persistence-probe")).resolve()
        installed_probe = install / probe.name
        shutil.copy2(probe, installed_probe)
        if args.block_network:
            block_windows_network(install, [binary, installed_probe], rules)
        subprocess.run([str(installed_probe), "create", str(history)], check=True)
        probe_env = dict(os.environ)
        probe_env.pop("GRAND_COACH_MODELS", None)
        subprocess.run([str(installed_probe), "neural"], cwd=root, env=probe_env, check=True, timeout=90)
        subprocess.run([str(installed_probe), "hybrid"], cwd=root, env=probe_env, check=True, timeout=90)
        model = install / manifest["models"][0]
        model_manifest = install / manifest["models"][1]
        original_model = model.read_bytes()
        original_manifest = model_manifest.read_bytes()
        model.unlink()
        subprocess.run([str(installed_probe), "missing"], cwd=root, env=probe_env, check=True, timeout=90)
        subprocess.run([str(installed_probe), "hybrid-missing"], cwd=root, env=probe_env, check=True, timeout=90)
        model.write_bytes(original_model)
        for mode, offset in (("format", 4), ("encoder", 6)):
            incompatible = bytearray(original_model)
            incompatible[offset:offset + 2] = b"\xff\xff"
            model.write_bytes(incompatible)
            metadata = json.loads(original_manifest)
            metadata["model_sha256"] = digest(incompatible)
            model_manifest.write_text(json.dumps(metadata), encoding="utf-8")
            for probe_mode in (mode, "hybrid-" + mode):
                subprocess.run([str(installed_probe), probe_mode], cwd=root, env=probe_env, check=True, timeout=90)
        model.write_bytes(original_model)
        model_manifest.write_bytes(original_manifest)
        preserved = history_valid = history.read_bytes()
        valid_history_hash = digest(preserved)
        stages = [(s, None, None) for s in ("install", "restart", "upgrade", "corrupt-model", "damaged-history")]
        shots = {}
        for stage, scenario, language in stages + [
            (f"{scenario}-{language}", scenario, language)
            for scenario in SCENARIOS for language in ("de", "en")
        ]:
            if scenario:
                # Valid history again; feature views must neither modify nor depend on damaged data.
                preserved = history_valid
            if stage == "corrupt-model":
                model = install / manifest["models"][0]
                model.write_bytes(b"corrupt" + model.read_bytes()[7:])
                for probe_mode in ("fallback", "hybrid-fallback"):
                    subprocess.run([str(installed_probe), probe_mode], cwd=root, env=probe_env, check=True, timeout=90)
            if stage == "upgrade":
                binary.unlink()
                binary.write_bytes(files[manifest["binary"]])
                binary.chmod(0o755)
            if stage == "damaged-history":
                preserved = b"packaging smoke: preserve damaged history\n"
                history.write_bytes(preserved)
            if scenario:
                history.write_bytes(history_valid)
            screenshot = (args.out / f"{stage}.ppm").resolve()
            screenshot.unlink(missing_ok=True)
            env = dict(os.environ, GRAND_COACH_DATA=str(data), GRAND_COACH_SEED="13", GRAND_COACH_SCREENSHOT=str(screenshot), GRAND_COACH_PACKAGE_NEURAL="1")
            env.pop("GRAND_COACH_MODELS", None)
            env.pop("GRAND_COACH_VALIDATION", None)
            if scenario:
                env.update(GRAND_COACH_SMOKE_SCENARIO=scenario, GRAND_COACH_LANGUAGE=language)
                env.pop("GRAND_COACH_PACKAGE_NEURAL")
            # cwd has no checkout/assets; models come only from the install directory.
            log_path = args.out / f"{stage}.log"
            with log_path.open("w") as log:
                try:
                    subprocess.run([str(binary)], cwd=root, env=env, stdout=log, stderr=log, check=True, timeout=960)
                except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                    log.flush()
                    print(log_path.read_text(encoding="utf-8", errors="replace")[-8192:])
                    raise
            image = screenshot.read_bytes()
            if not image.startswith(b"P6\n") or len(image) < 100000:
                raise ValueError("native packaged launch did not produce a screenshot")
            shots[stage] = digest(image)
            # Played scenarios may legitimately record learning observations; the
            # probe below still requires the history to stay valid.
            if scenario in PLAYED:
                preserved = history.read_bytes()
                if scenario == "training" and preserved == history_valid:
                    raise ValueError("packaged training did not persist its learning events")
            elif history.read_bytes() != preserved:
                raise ValueError("packaged launch changed user history")
            if stage != "damaged-history":
                subprocess.run([str(installed_probe), "check", str(history)], check=True)
        if len(set(shots[f"{c}-{l}"] for c in SCENARIOS for l in ("de", "en"))) != 2 * len(SCENARIOS):
            raise ValueError("feature DE/EN smoke screenshots are not all distinct views")
        shutil.rmtree(install)
        if history.read_bytes() != preserved:
            raise ValueError("uninstall removed user data")
    (args.out / "smoke.json").write_text(json.dumps({
        "commit": manifest["commit"], "archive_sha256": digest(args.archive.read_bytes()),
        "os": platform.platform(), "result": "passed", "network_isolation": ("program-scoped outbound Windows Firewall block on the installed application and probe, verified by a curl canary that reached the network before and failed after" if args.block_network else "not verified by this smoke"),
        "valid_history_sha256": valid_history_hash, "probe_sha256": digest(probe.read_bytes()),
        "test_runtime_dlls": runtime_dlls,
        "checks": ["archive resources/hashes/notices", "native launch without loose application resources", "actual app API training history creation/reopen", "valid-history restart", "same-build replacement upgrade", "damaged-history preservation", "same-build app API exercises Neural and Hybrid card play, and missing, corrupt and incompatible format/encoder model fallback with valid history", "packaged coach+counting aid+quiz view (first counting question open) and focused counting training view, each in German and English, launched from the same extracted executable (the coach question and a training answer, assisted retry, skip and next position record valid history)", "packaged complete 36-game series (first legal actions to the series result), open-table game (all hands shown, mid-game) and recording viewer (first game saved to the data directory, reopened and shown read-only) its post-game review (requested and finished with a summary), an isolated practice attempt, retry and return to the unchanged live view from the reopened recording, and a window with all 32 faces of all three card decks from the embedded images, each in German and English, from the same extracted executable", "uninstall retains data"],
        "screenshot_sha256": shots,
        "limitations": "History and Neural decisions use a linked app API probe, not GUI interaction. Coach/quiz/training scenarios open the real packaged views via smoke-only environment variables and a first-legal-action driver; training answer, assisted retry, skip and next use the training controller, not button clicks; coach/quiz answers remain widget-test coverage. The series, open-table and recording scenarios use the same driver: the series uses first legal actions and is not an interactive session; the review is requested by the driver; the card-face window shows the embedded images, inspected only by screenshot; practice starts, retries and returns through the app API, not button clicks; decision selection is not clicked. No cross-version migration; no clean OS or Windows 11 claim.",
    }, indent=2) + "\n", encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    b = sub.add_parser("build")
    b.add_argument("--out", type=Path, required=True)
    b.add_argument("--rust-version", default="1.95.0")
    v = sub.add_parser("verify")
    v.add_argument("archive", type=Path)
    s = sub.add_parser("smoke")
    s.add_argument("archive", type=Path)
    s.add_argument("--out", type=Path, required=True)
    s.add_argument("--windows-mesa", type=Path, help="test-only Mesa x64 DLL directory, never added to the application archive")
    s.add_argument("--block-network", action="store_true", help="Windows only: block outbound traffic of the installed programs and verify it with a canary")
    args = parser.parse_args()
    if args.action == "build":
        build(args)
    elif args.action == "verify":
        print(json.dumps(verify(args.archive)[0], indent=2))
    else:
        smoke(args)


if __name__ == "__main__":
    main()
