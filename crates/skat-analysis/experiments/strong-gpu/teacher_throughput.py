# SPDX-License-Identifier: EUPL-1.2
"""Compare release generators sequentially, including actual shard writes."""

import argparse
import hashlib
import json
import statistics
import subprocess
import time
from pathlib import Path


def shard_hashes(directory):
    return {
        shard.name: hashlib.sha256(shard.read_bytes()).hexdigest()
        for shard in sorted(directory.glob("shard-*.jsonl"))
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--deals", type=int, default=1000)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--workers", type=int, nargs="+", default=[4, 10, 20])
    parser.add_argument("--cpus", type=int, default=20)
    args = parser.parse_args()
    if min(args.deals, args.repeats, *args.workers) < 1 or max(args.workers) > args.cpus:
        parser.error("expected positive counts and workers <= cpus")
    engines = {"reference": args.reference.resolve(), "candidate": args.candidate.resolve()}
    args.output.mkdir(parents=True, exist_ok=False)
    expected = None
    rates = {}
    for repeat in range(args.repeats):
        for index, workers in enumerate(args.workers):
            order = list(engines)
            if (index + repeat) % 2:
                order.reverse()
            for name in order:
                run = args.output / f"{repeat + 1}-{workers}-{name}"
                command = [
                    str(engines[name]), "generate", str(run), "--deals", str(args.deals),
                    "--seed", "0", "--cpus", str(args.cpus), "--workers", str(workers),
                    "--shard-deals", "10", "--minutes", "180", "--max-mb", "2048",
                ]
                with run.with_suffix(".log").open("w") as log:
                    start = time.monotonic()
                    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
                    elapsed = time.monotonic() - start
                    subprocess.run([engines["candidate"], "verify", run], stdout=log, check=True)
                deals = sum(
                    row["end_deal"] - row["first_deal"]
                    for row in (json.loads(p.read_text()) for p in run.glob("*.done"))
                )
                if deals != args.deals:
                    raise ValueError(f"{run}: only {deals} completed deals")
                hashes = shard_hashes(run)
                if expected is None:
                    expected = hashes
                if hashes != expected:
                    raise ValueError(f"{run}: teacher records differ")
                rate = deals * 3600 / elapsed
                rates.setdefault((workers, name), []).append(rate)
                print(f"{run.name}: {elapsed:.3f} s, {rate:.1f} deals/h, identical shards", flush=True)
    for key, values in sorted(rates.items()):
        print(f"{key}: median {statistics.median(values):.1f} deals/h")
    best = {
        name: max(statistics.median(rates[w, name]) for w in args.workers)
        for name in engines
    }
    print(f"best candidate / best reference: {best['candidate'] / best['reference']:.3f}x")


if __name__ == "__main__":
    main()
