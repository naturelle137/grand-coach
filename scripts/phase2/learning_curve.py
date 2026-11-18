#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Read-only learning-curve summary of an `nn-train rl` train.log.

Usage: python3 scripts/phase2/learning_curve.py TRAIN_LOG [WINDOW]
Prints JSON with per-window means of the training reward and of each pool
kind (Seeger-Fabian points, mean of iteration means). These are on-policy
training statistics against a moving pool, not held-out strength evidence.
"""
import json
import re
import sys
from pathlib import Path

LINE = re.compile(r'^iteration\s+(\d+):.*?reward ([+-][\d.]+), vs pool \[(.*?)\],'
                  r'.*?KL\(ref\) ([\d.]+)')
KIND = re.compile(r'([\w+]+) ([+-][\d.]+) \((\d+)\)')


def parse(text):
    rows = []
    for line in text.splitlines():
        m = LINE.match(line)
        if m:
            kinds = {k: float(v) for k, v, _ in KIND.findall(m.group(3))}
            rows.append((int(m.group(1)), float(m.group(2)), kinds, float(m.group(4))))
    return rows


def mean(values):
    return round(sum(values) / len(values), 2)


def summarize(rows, window):
    out = []
    for i in range(0, len(rows), window):
        chunk = rows[i:i + window]
        names = sorted({k for r in chunk for k in r[2]})
        out.append({
            'iterations': [chunk[0][0], chunk[-1][0]],
            'reward': mean([r[1] for r in chunk]),
            'kl_ref': mean([r[3] for r in chunk]),
            'pool': {n: mean([r[2][n] for r in chunk if n in r[2]]) for n in names},
        })
    return out


def main(argv):
    if not 2 <= len(argv) <= 3:
        raise SystemExit(__doc__)
    window = int(argv[2]) if len(argv) == 3 else 50
    rows = parse(Path(argv[1]).read_text())
    if not rows or window < 1:
        raise SystemExit('No iterations parsed or invalid window')
    print(json.dumps(summarize(rows, window), indent=2))


if __name__ == '__main__':
    main(sys.argv)
