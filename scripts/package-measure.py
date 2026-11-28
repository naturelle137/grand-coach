#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Packaged timing and RSS driver for #423 (Linux).

Installs the release archive's executable and models into a scratch
directory, then runs the RC probe's `measure` scenario in fresh processes:
cold start (fresh process, warm OS cache), warm Neural decisions and one
full debriefing. RSS is sampled from `ps` at the probe's idle points
(Classic baseline, after the learned scenario) in the same process. Raw
observations go to --out/runs.jsonl, the record to --out/measurement.json.
Only a run on the named machine with --classification target is target
evidence; every other host must say proxy.
"""

import argparse
import hashlib
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

import package


def text(*cmd):
    try:
        return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        return f"unavailable: {error}"


def rss_kib(pid):
    return int(text("ps", "-o", "rss=", "-p", str(pid)))


def one_run(probe, env, cwd):
    proc = subprocess.Popen([str(probe), "measure"], cwd=cwd, env=env, stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, text=True)
    run, samples = {"events": []}, {}
    try:
        for line in proc.stdout:
            event = json.loads(line)
            run["events"].append(event)
            if "sync" in event:
                samples[event["sync"]] = {"time": datetime.now(timezone.utc).isoformat(), "rss_kib": rss_kib(proc.pid)}
                proc.stdin.write("go\n")
                proc.stdin.flush()
            else:
                run.update(event)
        if proc.wait(timeout=60) != 0 or set(samples) != {"baseline", "after"}:
            raise ValueError("measure run failed; pair invalid")
    except BaseException:
        proc.kill()
        raise
    run["rss"] = {**samples, "delta_kib": samples["after"]["rss_kib"] - samples["baseline"]["rss_kib"]}
    return run


def gui_run(binary, neural, env, cwd, delay):
    """One GUI process (screenshot smoke, closes itself); RSS polled every 0.25 s.

    Classic: new-series screen, Classic opponents. Neural: GRAND_COACH_PACKAGE_NEURAL
    plays the package smoke's learned opponent. Needs a display (xvfb-run is fine).
    """
    shot = Path(cwd) / "gui.ppm"
    gui_env = dict(env, GRAND_COACH_DATA=str(Path(cwd) / "data"), GRAND_COACH_SEED="13",
                   GRAND_COACH_SCREENSHOT=str(shot), GRAND_COACH_SCREENSHOT_DELAY=str(delay),
                   **{k: v for k, v in os.environ.items() if k in ("DISPLAY", "XAUTHORITY")})
    if neural:
        gui_env["GRAND_COACH_PACKAGE_NEURAL"] = "1"
    proc = subprocess.Popen([str(binary)], cwd=cwd, env=gui_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    samples = []
    deadline = time.monotonic() + delay + 60
    try:
        while proc.poll() is None:
            if time.monotonic() >= deadline:
                raise ValueError("GUI run timed out; pair invalid")
            try:
                samples.append(rss_kib(proc.pid))
            except ValueError:
                break
            time.sleep(0.25)
        if proc.wait(timeout=60) != 0 or not shot.is_file() or not samples:
            raise ValueError("GUI run failed; pair invalid")
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
    shot.unlink()
    return {"neural": neural, "samples_kib": samples, "last_kib": samples[-1], "max_sampled_kib": max(samples)}


def dist(values):
    values = sorted(values)
    pick = lambda p: values[min(len(values) - 1, int(p * len(values)))]
    return {"n": len(values), "min": values[0], "median": statistics.median(values),
            "p95": pick(0.95), "max": values[-1]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--classification", choices=("target", "proxy"), required=True)
    parser.add_argument("--machine", required=True, help="machine identity; for proxy, say so")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--gui-delay", type=float, default=0, help="also run the GUI process (Classic and Neural) per run, closing after this many seconds (e.g. 20); needs a display")
    args = parser.parse_args()
    if args.runs < 1 or not math.isfinite(args.gui_delay) or args.gui_delay < 0:
        parser.error("--runs must be positive and --gui-delay must be finite and non-negative")
    if platform.system() != "Linux":
        raise SystemExit("Linux only; record Windows with native tools")
    manifest, files = package.verify(args.archive)
    args.out.mkdir(parents=True, exist_ok=True)
    probe_src = args.archive.parent / "persistence-probe"
    with tempfile.TemporaryDirectory(prefix="grand-coach-measure-") as directory:
        install = Path(directory) / "install"
        install.mkdir()
        for name in (manifest["binary"], *manifest["models"]):
            (install / name).parent.mkdir(parents=True, exist_ok=True)
            (install / name).write_bytes(files[name])
        (install / manifest["binary"]).chmod(0o755)
        probe = install / probe_src.name
        shutil.copy2(probe_src, probe)
        env = {"PATH": "/usr/bin:/bin", "HOME": directory}  # no GRAND_COACH_MODELS: install dir only
        identity = {"files": {n: {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(), "bytes": p.stat().st_size}
                              for n, p in {args.archive.name: args.archive,
                                           manifest["binary"]: install / manifest["binary"],
                                           probe.name: probe,
                                           **{m: install / m for m in manifest["models"]}}.items()}}
        record = {
            "date": datetime.now(timezone.utc).isoformat(), "classification": args.classification,
            "machine": args.machine, "uname": text("uname", "-srmo"), "lscpu": text("lscpu"),
            "graphics": text("sh", "-c", "lspci -nn | grep -Ei 'vga|3d|display'"),
            "memory": text("free", "-b"), "load": text("uptime"),
            "release_commit": manifest["commit"], "build_command": manifest["build_command"],
            "features": manifest["features"], "installation": "release archive extracted by this driver; models from install directory",
            **identity,
            "scenario": "Easy opponents, seed 7, one-game series; Neural engine decisions; debriefing with learned review order, EveryMove coach, Budget::INTERACTIVE",
            "command": [str(probe_src.name), "measure"],
            "cold": "fresh process, OS file cache not cleared",
            "rss_method": "ps -o rss= (KiB) of the probe process at idle sync points, baseline = Classic session before Neural is created; delta from the same process; point samples, no peak",
            "limitations": ["RSS is of the RC probe (same skat-app build as the package), not the GUI process; the GUI baseline and GUI-selected scenario are not exercised",
                            "power mode and contention not controlled by this driver; record them manually"],
        }
        runs = [one_run(probe, env, directory) for _ in range(args.runs)]
        guis = [gui_run(install / manifest["binary"], neural, env, directory, args.gui_delay)
                for _ in range(args.runs) if args.gui_delay for neural in (False, True)]
    (args.out / "runs.jsonl").write_text("".join(json.dumps(r) + "\n" for r in runs), encoding="utf-8")
    if guis:
        (args.out / "gui.jsonl").write_text("".join(json.dumps(g) + "\n" for g in guis), encoding="utf-8")
        record["gui_rss"] = {name: {"last_kib": dist([g["last_kib"] for g in guis if g["neural"] == flag]),
                                    "max_sampled_kib": dist([g["max_sampled_kib"] for g in guis if g["neural"] == flag])}
                             for name, flag in (("classic", False), ("neural", True))}
        record["gui_method"] = ("grand-coach process with GRAND_COACH_SCREENSHOT smoke, closing after --gui-delay s; ps -o rss= every 0.25 s; "
                                "classic = new-series screen with Classic opponents, neural = GRAND_COACH_PACKAGE_NEURAL=1 (as the package smoke); "
                                "last = final sample before exit; sampled maximum is not the kernel peak; each GUI process is fresh")
        record["limitations"][0] = "RSS of the probe is not the GUI's; GUI RSS above comes from separate fresh GUI processes in the screenshot smoke, not an interactive session or a played game"
    record["summary"] = {
        "cold_start_ms": dist([r["cold_start_ms"] for r in runs]),
        "warm_decision_ms": dist([v for r in runs for v in r["warm_decision_ms"]]),
        "debriefing_ms": dist([r["debriefing_ms"] for r in runs]),
        "rss_after_kib": dist([r["rss"]["after"]["rss_kib"] for r in runs]),
        "rss_delta_kib": dist([r["rss"]["delta_kib"] for r in runs]),
        "aggregation": "per-run values; warm decisions pooled over runs; p95 = sorted[int(0.95 n)]",
    }
    (args.out / "measurement.json").write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(record["summary"], indent=2))


if __name__ == "__main__":
    main()
