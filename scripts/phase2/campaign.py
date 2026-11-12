#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Resource-reserved HQ entrypoints; never starts/stops training or changes weights."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time

SHAPES = ['4x384', '4x512', '4x896']
FIRST = 8_000_000
PILOT_FIRST = 7_900_000
TARGET_N = 2000
MAX_SECONDS = 96 * 3600  # summed 4-CPU wall time, not a promise of elapsed time
MASK = (1 << 64) - 1


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def save(p, data):
    tmp = p.with_suffix(p.suffix + '.tmp')
    tmp.write_text(json.dumps(data, indent=2) + '\n')
    tmp.replace(p)


def mix(h, v):
    z = h ^ ((v + 0x9E3779B97F4A7C15) & MASK)
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return z ^ (z >> 31)


def pairs():
    return [(s, b) for s in SHAPES for b in ['v1', s + '-start', 'easy', 'medium', 'strong']] + [
        (SHAPES[a], SHAPES[b]) for a, b in [(0, 1), (0, 2), (1, 2)]]


def spec(name, manifest):
    if name in manifest['models']:
        return 'nn:' + manifest['models'][name]['path'] + '@easy'
    return name


def config(a, b, m):
    # Pairwise replacement in identical contexts. Odd deals swap defenders.
    lines = [f'player {n} {spec(n, m)}' for n in dict.fromkeys([a, b, 'easy', 'medium', 'strong'])]
    for x in (a, b):
        lines += [f'match {x} easy easy', f'match {x} medium strong', f'match strong {x} medium']
    return '\n'.join(lines) + '\n'


def check_models(root):
    m = json.loads((root / 'manifest.json').read_text())
    for x in m['models'].values():
        p = Path(x['path'])
        assert p.stat().st_size == x['bytes'] and digest(p) == x['sha256'], str(p)
    return m


def idle():
    active = []
    for p in Path('/proc').glob('[0-9]*/cmdline'):
        try:
            args = p.read_bytes().split(b'\0')
            if not args:
                continue
            exe = Path(args[0].decode()).name
            if exe.startswith('nn-train') and b'rl' in args:
                active.append(str(p))
            if exe.startswith('tournament-fdb2109') or (exe.startswith('python') and b'orchestrate.py' in args):
                active.append(str(p))
        except (OSError, UnicodeError):
            continue
    assert not active, f'External training/arena active: {active}'


def run(cmd, out, timeout):
    """Calibrate restarts its current chunk; tournament resumes validated whole rows."""
    started = time.monotonic()
    with out.open('w') as stdout, out.with_suffix(out.suffix + '.stderr').open('w') as stderr:
        subprocess.run(cmd, check=True, stdout=stdout, stderr=stderr, timeout=timeout)
    return time.monotonic() - started


def binary(repo, name):
    return str(repo / 'target/release/examples' / name)


def audit(root, repo, data):
    """Enumerate actual u64 seeds: different suite keys alone are insufficient."""
    provenance = root / 'provenance'
    known_max = 0
    files = []
    for p in data.rglob('*.tsv'):
        lo, hi = None, None
        with p.open() as f:
            for line in f:
                col = line.split('\t', 1)[0]
                if col.isdecimal():
                    n = int(col)
                    lo = n if lo is None else min(lo, n)
                    hi = n if hi is None else max(hi, n)
        if hi is not None:
            known_max = max(known_max, hi)
            files.append({'path': str(p), 'sha256': digest(p), 'first': lo, 'last': hi})
    assert known_max < PILOT_FIRST, 'Prior arena reservation overlaps; new predeclaration needed'
    stream = Path(binary(repo, 'deal_seeds'))
    held = {}
    for label, suite, first, n in [('pilot', 'tuning', PILOT_FIRST, 4), ('validation', 'validation', FIRST, TARGET_N)]:
        out = root / (label + '-seeds.tsv')
        run([str(stream), suite, str(first), str(n)], out, 120)
        held[label] = {int(row.split()[1]) for row in out.read_text().splitlines()}
        assert len(held[label]) == n
    assert not held['pilot'] & held['validation']
    wanted = held['pilot'] | held['validation']
    checked = 0
    # Conservatively cover BOTH suites through the largest observed prior index.
    for suite in ['tuning', 'validation']:
        proc = subprocess.Popen([str(stream), suite, '0', str(known_max + 1)], stdout=subprocess.PIPE, text=True)
        for line in proc.stdout:
            assert int(line.split()[1]) not in wanted, 'Previous arena seed collision'
            checked += 1
        assert proc.wait() == 0
    manifests, ranges = [], set()
    for p in data.rglob('manifest.json'):
        try:
            d = json.loads(p.read_text())
            if 'seed_scheme' in d and 'deals' in d:
                assert 'mix(mix(seed, DEAL_KEY), deal)' in d['seed_scheme'], str(p)
                ranges.add((int(d['seed']), int(d['deals'])))
                manifests.append({'path': str(p), 'sha256': digest(p)})
        except json.JSONDecodeError:
            raise RuntimeError(f'Unreadable manifest: {p}')
    # Include embedded SL manifests (including shipped v1), even if datasets moved.
    for p in [*provenance.glob('*train.json'), repo / 'assets/models/card-play.json']:
        d = json.loads(p.read_text())
        for x in d['recipe']['data']:
            dm = x['manifest']
            assert 'mix(mix(seed, DEAL_KEY), deal)' in dm['seed_scheme']
            ranges.add((int(dm['seed']), int(dm['deals'])))
    for seed, count in sorted(ranges):
        base = mix(seed, 0x6465616c)
        for game in range(count):
            assert mix(base, game) not in wanted, 'Supervised training seed collision'
        checked += count
    rl = []
    for p in data.rglob('rl.json'):
        d = json.loads(p.read_text())
        recipe = d.get('recipe', {})
        h = recipe.get('hyper', {})
        if not d.get('logs'):
            continue
        seed, games = int(h['seed']), int(h['games'])
        # Include one extra iteration conservatively for interrupted earlier runs.
        iterations = max(int(x['iteration']) for x in d['logs']) + 1
        for iteration in range(iterations + 1):
            base = mix(seed ^ 0x726c6465616c, iteration)
            for game in range(games):
                assert mix(mix(base, game), 0) not in wanted, 'RL seed collision'
        checked += (iterations + 1) * games
        rl.append({'path': str(p), 'sha256': digest(p), 'seed': seed, 'iterations_inclusive': iterations, 'games': games})
    save(root / 'seed-audit.json', {'passed': True, 'checked': checked, 'prior_arena_max_index': known_max,
         'prior_tsv': files, 'dataset_manifests': manifests, 'supervised_ranges': sorted(ranges), 'rl_ranges': rl,
         'limitation': 'Audits known local data/manifests and observed arena indices, not inaccessible or unrecorded external campaigns. Equal seeds excluded; different seeds producing identical card permutations are not exhaustively enumerated.'})


def prepare(root, repo, data):
    m = check_models(root)
    idle()
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(repo / 'target'), CARGO_BUILD_JOBS='4')
    subprocess.run(['cargo', 'build', '--locked', '--release', '-p', 'skat-ai', '--example', 'tournament',
                    '--example', 'calibrate', '--example', 'latency', '--example', 'deal_seeds'], cwd=repo, check=True, env=build_env)
    audit(root, repo, data)
    for a, b in pairs():
        (root / f'{a}--{b}.cfg').write_text(config(a, b, m))
    save(root / 'configs.json', {p.name: digest(p) for p in root.glob('*.cfg')})
    save(root / 'binaries.json', {n: digest(Path(binary(repo, n))) for n in ['tournament', 'calibrate', 'latency', 'deal_seeds']})


def pilot(root, repo):
    m = check_models(root)
    idle()
    assert json.loads((root / 'seed-audit.json').read_text())['passed']
    rows = []
    pilot_start = time.monotonic()
    for a, b in pairs():
        stem = f'{a}--{b}'
        cfg = root / (stem + '.cfg')
        tsv = root / ('pilot-' + stem + '.tsv')
        # Existing TSV rows may resume, but cannot produce an understated timing.
        assert not tsv.exists(), 'Pilot is immutable; inspect failure rather than reusing partial timings'
        t = run([binary(repo, 'tournament'), str(cfg), str(tsv), 'tuning', '4', '4', str(PILOT_FIRST)], root / ('pilot-' + stem + '.out'), 600)
        c = run([binary(repo, 'calibrate'), 'tuning', spec(a, m), spec(b, m), '4', '4', str(PILOT_FIRST)], root / ('pilot-cal-' + stem + '.md'), 600)
        rows.append({'pair': [a, b], 'deals': 4, 'tournament_seconds': t, 'calibrate_seconds': c})
        assert time.monotonic() - pilot_start < 3600, 'Pilot exceeded one-hour budget'
    # Budget uses timings only, never outcome scores. Factor 2 allows heavy tails.
    seconds_per_deal = sum(x['tournament_seconds'] + x['calibrate_seconds'] for x in rows) / 4
    n = min(TARGET_N, int(MAX_SECONDS / (2 * seconds_per_deal)))
    if n < 200:
        n = 0
    for row in rows:
        row['validation_budget_seconds'] = 2 * n * (row['tournament_seconds'] + row['calibrate_seconds']) / 4
    save(root / 'budget.json', {'target_n': TARGET_N, 'n': n, 'pilot': rows, 'safety_factor': 2,
        'estimated_serial_seconds': n * seconds_per_deal, 'conservative_serial_seconds': 2 * n * seconds_per_deal,
        'max_summed_four_cpu_wall_seconds': MAX_SECONDS, 'cpu_hours_cap': 4 * MAX_SECONDS / 3600,
        'note': 'Frozen before held-out execution; N<2000 is reduced precision; N=0 forbids full matrix. No sample increase after viewing results.',
        'frozen_unix': time.time()})
    with (root.parent / 'notes.md').open('a') as notes:
        notes.write(f'\nPilot completed: {seconds_per_deal:.3f} summed4CPU seconds/deal; N={n}; estimated serial {n * seconds_per_deal / 3600:.2f}h, conservative {2 * n * seconds_per_deal / 3600:.2f}h, cap96h. Per-pair measurements frozen in evaluation/budget.json before held-out execution.\n')


def runtime(root, repo):
    idle()
    m = check_models(root)
    save(root / 'machine.json', {'platform': platform.platform(), 'cpu': Path('/proc/cpuinfo').read_text(),
        'loadavg': os.getloadavg(), 'workers': 4, 'gpu': False, 'note': 'HQ CPU reservation is not exclusive machine access; latency includes Strong fallback in unsupported phases.'})
    for name, x in m['models'].items():
        run([binary(repo, 'latency'), '30', 'nn:' + x['path'], 'neural-only'], root / ('latency-' + name + '.md'), 900)
    with (root / 'smoke.log').open('w') as out:
        for cmd in [['cargo', 'test', '--locked', '-p', 'skat-nn'], ['cargo', 'test', '--locked', '-p', 'skat-app', 'neural::tests::reads_the_manifest_hash']]:
            subprocess.run(cmd, cwd=repo, check=True, stdout=out, stderr=subprocess.STDOUT, timeout=900)


def repair_tsv(path):
    if not path.exists():
        return
    raw = path.read_bytes()
    rows = raw.splitlines(keepends=True)
    good, seen = [], set()
    for i, row in enumerate(rows):
        fields = row.decode().rstrip('\n').split('\t')
        if len(fields) != 12 or not row.endswith(b'\n'):
            assert i == len(rows) - 1, 'Malformed interior TSV row'
            path.with_suffix('.truncated-tail').write_bytes(row)
            break
        key = tuple(fields[:2])
        assert key not in seen, 'Duplicate TSV row'
        seen.add(key)
        good.append(row)
    if b''.join(good) != raw:
        path.write_bytes(b''.join(good))


def arena(root, repo, index):
    check_models(root)
    budget = json.loads((root / 'budget.json').read_text())
    assert budget['n'] >= 200, 'Pilot budget no-go: no held-out execution'
    assert json.loads((root / 'seed-audit.json').read_text())['passed']
    binaries = json.loads((root / 'binaries.json').read_text())
    for name, sha in binaries.items():
        assert digest(Path(binary(repo, name))) == sha, 'Evaluation binary changed'
    for name, sha in json.loads((root / 'configs.json').read_text()).items():
        assert digest(root / name) == sha, 'Evaluation config changed'
    a, b = pairs()[index]
    m = check_models(root)
    stem = f'{a}--{b}'
    directory = root / 'validation' / stem
    directory.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    pair_cap = budget['pilot'][index]['validation_budget_seconds']
    prior_elapsed = 0.0
    for marker in directory.glob('[0-9]*.json'):
        prior_elapsed += json.loads(marker.read_text()).get('seconds', 0)
    for offset in range(0, budget['n'], 100):
        n = min(100, budget['n'] - offset)
        first = FIRST + offset
        complete = directory / f'{first}.json'
        if complete.exists():
            record = json.loads(complete.read_text())
            assert all(digest(directory / name) == sha for name, sha in record['outputs'].items())
            continue
        chunk_start = time.monotonic()
        tsv = directory / f'{first}.tsv'
        repair_tsv(tsv)
        cap = int(pair_cap - prior_elapsed - (time.monotonic() - started))
        assert cap > 0, 'Per-pair wall budget exhausted'
        run([binary(repo, 'tournament'), str(root / (stem + '.cfg')), str(tsv), 'validation', str(n), '4', str(first)], directory / f'{first}.out', cap)
        log = (directory / f'{first}.out.stderr').read_text()
        reference = re.search(r'(\d+) of (\d+) deals played', log)
        assert reference and int(reference[2]) == n, 'Missing runner coverage count'
        rows = [line.split('\t') for line in tsv.read_text().splitlines()]
        assert len(rows) == 6 * int(reference[1]), 'Incomplete role tournament output'
        assert len({(r[0], r[1]) for r in rows}) == len(rows), 'Duplicate role row'
        md = directory / f'{first}-cal.md'
        cap = int(pair_cap - prior_elapsed - (time.monotonic() - started))
        assert cap > 0, 'Per-pair wall budget exhausted'
        run([binary(repo, 'calibrate'), 'validation', spec(a, m), spec(b, m), str(n), '4', str(first)], md, cap)
        # stderr contains the native per-deal differences (rounded to 0.01).
        assert len(re.findall(r'^deal\s+\d+ seed', md.with_suffix('.md.stderr').read_text(), re.M)) == n
        save(complete, {'first': first, 'n': n, 'seconds': time.monotonic() - chunk_start, 'outputs': {p.name: digest(p) for p in [tsv, md, md.with_suffix('.md.stderr')]}})
    save(directory / 'complete.json', {'pair': [a, b], 'n': budget['n'], 'seconds_this_attempt': time.monotonic() - started})


def main():
    p = argparse.ArgumentParser(__doc__)
    p.add_argument('stage', choices=['prepare', 'pilot', 'runtime', 'arena'])
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--repo', type=Path, required=True)
    p.add_argument('--data', type=Path)
    p.add_argument('--index', type=int)
    a = p.parse_args()
    if a.stage == 'prepare':
        prepare(a.root, a.repo, a.data)
    elif a.stage == 'arena':
        arena(a.root, a.repo, a.index)
    elif a.stage == 'pilot':
        pilot(a.root, a.repo)
    else:
        runtime(a.root, a.repo)


if __name__ == '__main__':
    main()
