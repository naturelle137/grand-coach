#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Paired deal-level Phase2 evidence. No weight or default changes."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import statistics

from campaign import FIRST, check_models, pairs, save


def interval(values, z=1.96, rounding=0.0):
    if len(values) < 2:
        return {'n': len(values), 'mean': statistics.mean(values) if values else None, 'low': None, 'high': None, 'verdict': 'inconclusive'}
    mean = statistics.mean(values)
    se = statistics.stdev(values) / len(values) ** 0.5
    lo, hi = mean - z * se - rounding, mean + z * se + rounding
    return {'n': len(values), 'mean': mean, 'se': se, 'low': lo, 'high': hi,
            'verdict': 'positive' if lo > 0 else 'negative' if hi < 0 else 'inconclusive'}


def role_values(rows):
    """Return paired A-minus-B seat SF, not unpaired campaign score_tsv means."""
    grouped = defaultdict(dict)
    for f in rows:
        assert len(f) == 12
        deal, match = int(f[0]), int(f[1])
        assert match not in grouped[deal], 'Duplicate match in one deal'
        grouped[deal][match] = f
    result = defaultdict(list)
    excluded = 0
    for deal, games in grouped.items():
        assert set(games) == set(range(6)), f'Incomplete role deal {deal}'
        deltas = []
        for k in range(3):
            a, b = games[k], games[k + 3]
            if a[11] != '0' or b[11] != '0' or a[2:4] != b[2:4]:
                excluded += 1
                continue
            if k < 2:
                delta = int(a[6]) - int(b[6])
                role = 'declarer'
            else:
                # The candidate is paired with Medium; column name identifies it
                # independently of the runner's odd/even defender swap.
                def sf(f):
                    return int(f[8] if f[9] == 'medium' else f[7])
                delta = sf(a) - sf(b)
                role = 'defender'
            deltas.append(delta)
            for key in [('context', str(k)), ('role', role), ('contract', a[3]), ('seat', a[2]), ('role_contract', role + ':' + a[3])]:
                result[key].append((deal, delta))
        if len(deltas) == 3:
            result[('overall', 'all')].append((deal, statistics.mean(deltas)))
    # Multiple contexts within a deal are correlated: aggregate before intervals.
    output = {}
    for key, values in result.items():
        by_deal = defaultdict(list)
        for deal, value in values:
            by_deal[deal].append(value)
        output[':'.join(key)] = interval([statistics.mean(x) for x in by_deal.values()])
    return output, excluded


def analyze(root):
    check_models(root)
    budget = json.loads((root / 'budget.json').read_text())
    reports = []
    for a, b in pairs():
        folder = root / 'validation' / f'{a}--{b}'
        assert json.loads((folder / 'complete.json').read_text())['n'] == budget['n']
        diffs, role_rows, seen = [], [], set()
        for first in range(FIRST, FIRST + budget['n'], 100):
            record = json.loads((folder / f'{first}.json').read_text())
            assert record['n'] == min(100, FIRST + budget['n'] - first)
            from campaign import digest
            assert all(digest(folder / name) == sha for name, sha in record['outputs'].items())
            text = (folder / f'{first}-cal.md.stderr').read_text()
            for line in text.splitlines():
                m = re.match(r'^deal\s+(\d+) seed\s+(\d+):\s+([+-]?[\d.]+) ', line)
                if m:
                    deal = int(m[1])
                    assert deal not in seen and FIRST <= deal < FIRST + budget['n']
                    seen.add(deal)
                    diffs.append(float(m[3]))
            role_rows += [line.split('\t') for line in (folder / f'{first}.tsv').read_text().splitlines()]
        assert len(diffs) == budget['n']
        strata, excluded = role_values(role_rows)
        reports.append({'pair': [a, b], 'seat_rotation_primary': interval(diffs, 2.638, 0.01),
                        'nominal_95': interval(diffs, 1.96, 0.01), 'role_strata_95': strata,
                        'excluded_context_pairs': excluded,
                        'role_deals_observed': len({r[0] for r in role_rows}),
                        'role_rows': len(role_rows),
                        'note': 'Calibrate per-deal differences rounded to0.01; intervals widened0.01. Primary simultaneous six-comparison Bonferroni normal intervals. Cross-shape and Classic comparisons exploratory; contract strata descriptive, not separately powered.'})
    save(root / 'analysis.json', {'reports': reports, 'n': budget['n'],
         'acceptance': 'See predeclaration.md; this script provides evidence, does not auto-select or change weights.',
         'limitation': 'Normal approximation; role runner omits thrown-in deals, mismatched contexts excluded and counted. Sparse/empty strata inconclusive. Rotating compare_with covers all six A/B seatings, not a guarantee of balanced contracts.'})


if __name__ == '__main__':
    p = argparse.ArgumentParser(__doc__)
    p.add_argument('root', type=Path)
    analyze(p.parse_args().root)
