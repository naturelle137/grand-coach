#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Read-only exploratory completed-prefix analysis; never Phase 2 acceptance.

Usage: python3 scripts/phase2/operational_selection.py EVALUATION OUTPUT.json
Only OUTPUT is written. Frozen markers, raw outputs and models are read only.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import statistics

from analyze import interval, role_values
from campaign import FIRST, pairs


def analyze(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    for name, model in manifest['models'].items():
        data = (root / 'models' / f'{name}.gcnn').read_bytes()
        assert len(data) == model['bytes']
        assert hashlib.sha256(data).hexdigest() == model['sha256']
    folders = {p for p in (root / 'validation').iterdir() if p.is_dir()}
    assert {p.name for p in folders} == {f'{a}--{b}' for a, b in pairs()}
    reports, all_values = [], {}
    for folder in sorted(folders):
        values, rows, times, chunks = {}, [], [], []
        for marker in sorted(folder.glob('[0-9]*.json')):
            record = json.loads(marker.read_text())
            first = record['first']
            assert first == FIRST + 100 * len(chunks), 'Non-contiguous prefix'
            assert record['n'] == 100
            chunks.append(first)
            times.append(record['seconds'])
            for name, digest in record['outputs'].items():
                assert hashlib.sha256((folder / name).read_bytes()).hexdigest() == digest
            text = (folder / f'{first}-cal.md.stderr').read_text()
            matches = re.findall(r'^deal\s+(\d+) seed\s+(\d+):\s+([+-]?[\d.]+) ', text, re.M)
            block = {int(deal): float(delta) for deal, _, delta in matches}
            assert len(matches) == len(block) == record['n']
            assert set(block) == set(range(first, first + 100))
            assert not set(block) & set(values)
            values.update(block)
            rows.extend(line.split('\t') for line in (folder / f'{first}.tsv').read_text().splitlines())
        strata, excluded = role_values(rows)
        all_values[folder.name] = values
        reports.append(dict(
            pair=folder.name, completed_chunks=chunks, n=len(values),
            primary=interval(list(values.values()), 1.96, .01),
            simultaneous6=interval(list(values.values()), 2.638, .01),
            roles=strata, excluded_contexts=excluded, role_rows=len(rows),
            role_deals=len({row[0] for row in rows}),
            chunk_seconds=dict(total=sum(times), median=statistics.median(times), maximum=max(times)),
            early=interval([v for k, v in values.items() if k < FIRST + 600], 1.96, .01),
            late=interval([v for k, v in values.items() if k >= FIRST + 600], 1.96, .01)))
    common = set.intersection(*(set(v) for v in all_values.values()))
    for report in reports:
        report['common_prefix'] = interval(
            [all_values[report['pair']][k] for k in sorted(common)], 1.96, .01)
    return dict(common_n=len(common), reports=reports,
                note='Exploratory exposed sample; n counts independent deals within each pair, never pooled across pairs. Whole hash-verified chunks only. Normal intervals, no fresh confirmation.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    assert not args.output.resolve().is_relative_to(args.root.resolve()), 'Keep frozen archive read-only'
    args.output.write_text(json.dumps(analyze(args.root), indent=2) + '\n')
