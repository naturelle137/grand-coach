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


def run(*args, cwd=ROOT, **kwargs):
    return subprocess.check_output(args, cwd=cwd, text=True, encoding="utf-8", **kwargs).strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


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


def read_archive(path):
    files = {}
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            entries = [(i.filename, archive.read(i)) for i in archive.infolist()]
    else:
        with tarfile.open(path, "r:gz") as archive:
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


def verify(path):
    files = read_archive(path)
    manifest = json.loads(files.pop("manifest.json"))
    expected = manifest["files"]
    if {name: digest(data) for name, data in files.items()} != expected:
        raise ValueError("archive contents/hashes differ from manifest")
    required = {*LEGAL, "INSTALL.md", "THIRD-PARTY.md", manifest["binary"], *manifest["embedded_resources"]}
    if not required <= files.keys() or not all(files[name] for name in required):
        raise ValueError("missing/empty required package files")
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for name, data in files.items():
            dest = root / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
        if resources(root) != manifest["embedded_resources"]:
            raise ValueError("incomplete resource inventory")
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
    notices(ROOT)
    sha = run("git", "rev-parse", "HEAD")
    epoch = int(run("git", "show", "-s", "--format=%ct", "HEAD"))
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    command = ["cargo", "build", "--locked", "--offline", "--release", "-p", "gui-egui", "--bin", "grand-coach", "--target", target, "--message-format=json-render-diagnostics"]
    env = dict(os.environ, SOURCE_DATE_EPOCH=str(epoch), CARGO_INCREMENTAL="0")
    compiled = subprocess.run(command, cwd=ROOT, env=env, check=True, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    artifacts = [json.loads(line) for line in compiled.stdout.splitlines() if line.startswith("{")]
    app_lib = next(Path(name) for a in artifacts if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "skat_app" for name in a["filenames"] if name.endswith(".rlib"))
    binary = "grand-coach.exe" if windows else "grand-coach"
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    files = {binary: (target_dir / target / "release" / binary).read_bytes()}
    for name in (*LEGAL, *names):
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
        "build_command": command, "features": "default", "models": [],
        "build_environment": {k: env.get(k, "") for k in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CC", "CXX", "CARGO_INCREMENTAL", "SOURCE_DATE_EPOCH")},
        "runner_image": {k: env.get(k, "") for k in ("ImageOS", "ImageVersion")},
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


def smoke(args):
    manifest, files = verify(args.archive)
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="grand-coach-package-") as directory:
        root = Path(directory)
        install = root / "install"
        install.mkdir()
        # Only the executable is required at runtime: resources are embedded.
        binary = install / manifest["binary"]
        binary.write_bytes(files[manifest["binary"]])
        binary.chmod(0o755)
        data = root / "user-data"
        data.mkdir()
        history = data / "learning-history.json"
        probe = (args.archive.parent / ("persistence-probe.exe" if manifest["binary"].endswith(".exe") else "persistence-probe")).resolve()
        subprocess.run([str(probe), "create", str(history)], check=True)
        preserved = history.read_bytes()
        valid_history_hash = digest(preserved)
        for stage in ("install", "restart", "upgrade", "damaged-history"):
            if stage == "upgrade":
                binary.unlink()
                binary.write_bytes(files[manifest["binary"]])
                binary.chmod(0o755)
            if stage == "damaged-history":
                preserved = b"packaging smoke: preserve damaged history\n"
                history.write_bytes(preserved)
            screenshot = (args.out / f"{stage}.ppm").resolve()
            screenshot.unlink(missing_ok=True)
            env = dict(os.environ, GRAND_COACH_DATA=str(data), GRAND_COACH_SEED="13", GRAND_COACH_SCREENSHOT=str(screenshot))
            # cwd has no checkout/assets and models are not shipped.
            with (args.out / f"{stage}.log").open("w") as log:
                subprocess.run([str(binary)], cwd=root, env=env, stdout=log, stderr=log, check=True, timeout=90)
            image = screenshot.read_bytes()
            if not image.startswith(b"P6\n") or len(image) < 100000:
                raise ValueError("native packaged launch did not produce a screenshot")
            if history.read_bytes() != preserved:
                raise ValueError("packaged launch changed user history")
            if stage != "damaged-history":
                subprocess.run([str(probe), "check", str(history)], check=True)
        shutil.rmtree(install)
        if history.read_bytes() != preserved:
            raise ValueError("uninstall removed user data")
    (args.out / "smoke.json").write_text(json.dumps({
        "commit": manifest["commit"], "archive_sha256": digest(args.archive.read_bytes()),
        "os": platform.platform(), "result": "passed",
        "valid_history_sha256": valid_history_hash, "probe_sha256": digest(probe.read_bytes()),
        "checks": ["archive resources/hashes/notices", "binary-only native launch", "actual app API training history creation/reopen", "valid-history restart", "same-build replacement upgrade", "damaged-history preservation", "uninstall retains data"],
        "limitations": "History creation/check uses linked app API probe, not GUI interaction. No cross-version migration or interactive flows; no clean OS or Windows 11 claim.",
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
    args = parser.parse_args()
    if args.action == "build":
        build(args)
    elif args.action == "verify":
        print(json.dumps(verify(args.archive)[0], indent=2))
    else:
        smoke(args)


if __name__ == "__main__":
    main()
