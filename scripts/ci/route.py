#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Conservative CI routing and strict mandatory-result aggregation (D-30)."""

import argparse
import json
import os
from pathlib import PurePosixPath
import subprocess


def git(*args):
    return subprocess.check_output(["git", *args], stderr=subprocess.PIPE)


def docs(path):
    p = PurePosixPath(path)
    return p.suffix == ".md" and (
        path.startswith("docs/")
        or path in {"README.md", "CONTRIBUTING.md", "AGENTS.md", "CLAUDE.md"}
    )


def classify(base, event, schedule, deep):
    if event not in {"pull_request", "push", "schedule", "workflow_dispatch"}:
        raise ValueError("unsupported event")
    full = event in {"schedule", "workflow_dispatch"}
    run_deep = deep or (event == "schedule" and schedule == "23 3 * * 1")
    if full:
        code, gui = True, True
    else:
        try:
            mb = git("merge-base", base, "HEAD").decode().strip() if base else ""
        except subprocess.CalledProcessError:
            mb = ""  # Unknown base: complete FAST + native, never docs-only.
        if not mb:
            code, gui = True, True
        else:
            # Disable rename detection: both the deleted and added paths count.
            # A failed diff is an error, never an empty docs-only diff.
            paths = set(git("diff", "--no-renames", "--name-only", "-z", mb, "HEAD").split(b"\0"))
            paths.update(git("diff", "--no-renames", "--name-only", "-z", "HEAD").split(b"\0"))
            paths.discard(b"")
            names = [p.decode("utf-8", errors="strict") for p in paths]
            code = not names or any(not docs(p) for p in names)
            gui = any(p.startswith(("crates/gui-egui/", "assets/")) for p in names)
    return {
        "code": str(code).lower(),
        "gui": str(gui).lower(),
        "full": str(full).lower(),
        "regression": str(event == "push" and code).lower(),
        "deep": str(run_deep).lower(),
    }


def aggregate(needs):
    expected_jobs = {"changes", "lint", "portable", "native", "msrv", "regression", "complete", "deep"}
    if set(needs) != expected_jobs:
        raise ValueError("missing or unexpected jobs")
    if needs["changes"]["result"] != "success":
        raise ValueError("classification failed")
    route = needs["changes"]["outputs"]
    if set(route) != {"code", "gui", "full", "regression", "deep"}:
        raise ValueError("missing or unexpected classification")
    if any(v not in {"true", "false"} for v in route.values()):
        raise ValueError("invalid classification")
    code, full = route["code"] == "true", route["full"] == "true"
    if (full and not code) or (route["gui"] == "true" and not code) or (
        route["regression"] == "true" and (not code or full)
    ) or (route["deep"] == "true" and not full):
        raise ValueError("inconsistent classification")
    required = {
        "lint": True, "portable": code, "native": code, "msrv": code,
        "regression": route["regression"] == "true",
        "complete": full, "deep": route["deep"] == "true",
    }
    for job, mandatory in required.items():
        expected = "success" if mandatory else "skipped"
        if needs[job]["result"] != expected:
            raise ValueError(f"{job}: expected {expected}, got {needs[job]['result']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["route", "aggregate"])
    parser.add_argument("--base", default="")
    parser.add_argument("--event", default="pull_request")
    parser.add_argument("--schedule", default="")
    parser.add_argument("--deep", action="store_true")
    args = parser.parse_args()
    if args.mode == "aggregate":
        aggregate(json.loads(os.environ["CI_NEEDS"]))
    else:
        route = classify(args.base, args.event, args.schedule, args.deep)
        print("\n".join(f"{k}={v}" for k, v in route.items()))


if __name__ == "__main__":
    main()
