#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Prepare a private learner kit from an exact clean native RC; never send it."""

import argparse
import json
import os
from pathlib import Path
import subprocess

import package


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(data)


def registered_tasks():
    return json.loads((package.ROOT / "docs/validation54-tasks.json").read_text())["tasks"]


def prepare(archive, out):
    commit = package.run("git", "rev-parse", "HEAD")
    if package.run("git", "status", "--porcelain", "--untracked-files=normal"):
        raise ValueError("kit requires clean committed source")
    archive_data = archive.read_bytes()
    manifest, files = package.verify(archive, archive_data)
    if manifest["commit"] != commit:
        raise ValueError("package/source commit mismatch")
    if out.exists() or out.is_symlink():
        raise ValueError("destination already exists; never replace personal records")
    if out.resolve().is_relative_to(package.ROOT.resolve()):
        raise ValueError("destination must be outside the source repository")
    if any(c in str(out) for c in '\n\r"`$%') or (os.name != "nt" and "\\" in str(out)):
        raise ValueError("unsupported launcher path")
    raw = subprocess.check_output([
        "cargo", "run", "--locked", "--offline", "--quiet", "-p", "skat-app",
        "--example", "validation54-manifest"], cwd=package.ROOT)
    cases = json.loads(raw)
    if len(cases) != 8 or len({(c["id"], c["version"]) for c in cases}) != 8:
        raise ValueError("expected eight immutable task identities")
    identities = [{"id": case["id"], "version": case["version"], "set": case["set"],
                   "position_sha256": package.digest(json.dumps(case["position"], sort_keys=True,
                                                               separators=(",", ":"), ensure_ascii=False).encode())}
                  for case in cases]
    if identities != registered_tasks():
        raise ValueError("reserved task identity/hash changed without a new preregistration")
    task_manifest = {
        "protocol": "validation54@1", "build_commit": commit,
        "source_tree": package.run("git", "rev-parse", "HEAD^{tree}"),
        "archive_sha256": package.digest(archive_data),
        "executable_sha256": manifest["files"][manifest["binary"]],
        "position_hash_encoding": "SHA-256 of UTF-8 JSON, sorted keys, compact separators, ensure_ascii=False",
        "tasks": identities,
    }
    encoded = (json.dumps(task_manifest, indent=2, ensure_ascii=False) + "\n").encode()
    template = json.loads((package.ROOT / "docs/validation54-blank.json").read_text())
    template.update(build_commit=commit, executable_sha256=task_manifest["executable_sha256"],
                    task_manifest_sha256=package.digest(encoded))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.mkdir(mode=0o700)
    for name, data in files.items():
        write_new(out / "app" / name, data)
    write_new(out / "app/manifest.json", (json.dumps(manifest, indent=2) + "\n").encode())
    binary = out / "app" / manifest["binary"]
    binary.chmod(0o755)
    write_new(out / "task-manifest.json", encoded)
    write_new(out / "Ergebnisse-lokal.json", (json.dumps(template, indent=2, ensure_ascii=False) + "\n").encode())
    write_new(out / "Anleitung.txt", (package.ROOT / "docs/validation54-learner-de.txt").read_bytes())
    # A new/missing history remains conservatively exposure-incomplete in the
    # product. Never initialise/reset it here to manufacture novelty.
    if manifest["binary"].endswith(".exe"):
        launch = out / "Grand-Coach-Lernvalidierung.cmd"
        content = ('@echo off\r\nset "GRAND_COACH_VALIDATION=1"\r\n'
                   'set "GRAND_COACH_DATA=%~dp0private-history"\r\n'
                   '"%~dp0app\\grand-coach.exe"\r\n')
    else:
        launch = out / "Grand-Coach-Lernvalidierung.sh"
        content = ('#!/bin/sh\nkit_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1\n'
                   'export GRAND_COACH_VALIDATION=1\nexport GRAND_COACH_DATA="$kit_dir/private-history"\n'
                   'exec "$kit_dir/app/grand-coach"\n')
    write_new(launch, content.encode())
    launch.chmod(0o755)
    if not manifest["binary"].endswith(".exe"):
        write_new(out / "Grand-Coach-Lernvalidierung.desktop", (
            "[Desktop Entry]\nType=Application\nName=Grand-Coach Lernvalidierung\n"
            f'Exec="{launch}"\nTerminal=false\nCategories=Game;Education;\n').encode())
    (out / "private-history").mkdir(mode=0o700)
    return task_manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.parent.resolve() / args.out.name
    result = prepare(args.archive.resolve(), out)
    print(json.dumps({"status": "prepared", "path": str(out),
                      "commit": result["build_commit"], "tasks": len(result["tasks"])}))


if __name__ == "__main__":
    main()
