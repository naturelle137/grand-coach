#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Read-only verification of a frozen PPO campaign; never admits new compute.

Usage: python3 scripts/phase2/revalidate.py EVALUATION
The JSON report goes to stdout. Missing or changed artifacts fail closed.
"""
import argparse
import json
from pathlib import Path

from campaign import SHAPES, digest


def verified(path, expected):
    if digest(path) != expected:
        raise ValueError(f'Hash mismatch: {path}')


def child(root, name):
    path = (root / name).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError(f'Path escapes archive: {name}')
    return path


def audit(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    campaign = Path(manifest['campaign'])
    if not (campaign / 'STOP').is_file():
        raise ValueError('Stopped campaign marker missing; no compute is admitted')
    models = {}
    for name, model in manifest['models'].items():
        path = Path(model['path'])
        verified(path, model['sha256'])
        if path.stat().st_size != model['bytes']:
            raise ValueError(f'Size mismatch: {path}')
        source = Path(model['source']) if 'source' in model else None
        models[name] = dict(sha256=model['sha256'],
                            source_matches=(digest(source) == model['sha256']
                                            if source and source.is_file() else None))
    checkpoints = {}
    for shape in SHAPES:
        run = campaign / shape / 'rl'
        state = json.loads((run / 'checkpoint/state.json').read_text())
        if not state['checkpoint'] or not state['logs']:
            raise ValueError(f'No completed checkpoint: {shape}')
        name, hashes = state['checkpoint']
        if len(hashes) != 2:
            raise ValueError(f'Invalid checkpoint hashes: {shape}')
        folder = child(run / 'checkpoint', name)
        for filename, expected in zip(('weights.bpk', 'optimizer.bpk'), hashes):
            verified(folder / filename, expected)
        for snapshot in state['snapshots']:
            verified(child(run / 'snapshots', snapshot['file']), snapshot['sha256'])
        checkpoints[shape] = dict(checkpoint=name, iterations=len(state['logs']),
                                  snapshots=len(state['snapshots']))
    return dict(models=models, checkpoints=checkpoints, compute_admitted=False,
                limitation='Integrity only; no strength, convergence or resume guarantee.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    print(json.dumps(audit(args.root), indent=2))
