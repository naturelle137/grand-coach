# SPDX-License-Identifier: EUPL-1.2
"""Tiny corruption fixtures; no simulator, training or archive mutation."""
import json
from pathlib import Path
import tempfile
import unittest

from campaign import SHAPES, digest
from revalidate import audit


class RevalidationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'STOP').touch()
        model = self.root / 'model.gcnn'
        model.write_bytes(b'model')
        (self.root / 'manifest.json').write_text(json.dumps(dict(
            campaign=str(self.root), models={'fixture': dict(
                path=str(model), source=str(model), bytes=5, sha256=digest(model))})))
        for shape in SHAPES:
            folder = self.root / shape / 'rl/checkpoint/iter-00001'
            folder.mkdir(parents=True)
            hashes = []
            for name in ('weights.bpk', 'optimizer.bpk'):
                path = folder / name
                path.write_bytes(name.encode())
                hashes.append(digest(path))
            (folder.parent / 'state.json').write_text(json.dumps(dict(
                checkpoint=['iter-00001', hashes], logs=[{}], snapshots=[])))

    def test_valid_archive_is_read_only_and_never_admits_compute(self):
        before = {p: p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        report = audit(self.root)
        self.assertFalse(report['compute_admitted'])
        self.assertEqual(set(report['checkpoints']), set(SHAPES))
        self.assertEqual(before, {p: p.read_bytes() for p in before})

    def test_corrupted_optimizer_fails(self):
        (self.root / SHAPES[0] / 'rl/checkpoint/iter-00001/optimizer.bpk').write_bytes(b'bad')
        with self.assertRaisesRegex(ValueError, 'Hash mismatch'):
            audit(self.root)

    def test_missing_stop_fails(self):
        (self.root / 'STOP').unlink()
        with self.assertRaisesRegex(ValueError, 'marker missing'):
            audit(self.root)

    def test_missing_model_fails(self):
        (self.root / 'model.gcnn').unlink()
        with self.assertRaises(FileNotFoundError):
            audit(self.root)

    def test_escape_fails(self):
        path = self.root / SHAPES[0] / 'rl/checkpoint/state.json'
        state = json.loads(path.read_text())
        state['checkpoint'][0] = '../../outside'
        path.write_text(json.dumps(state))
        with self.assertRaisesRegex(ValueError, 'escapes archive'):
            audit(self.root)

    def test_mutable_source_drift_does_not_change_frozen_identity(self):
        path = self.root / 'manifest.json'
        manifest = json.loads(path.read_text())
        source = self.root / 'mutable.gcnn'
        source.write_bytes(b'replacement')
        manifest['models']['fixture']['source'] = str(source)
        path.write_text(json.dumps(manifest))
        self.assertFalse(audit(self.root)['models']['fixture']['source_matches'])

    def test_corrupted_snapshot_fails(self):
        run = self.root / SHAPES[0] / 'rl'
        (run / 'snapshots').mkdir()
        snapshot = run / 'snapshots/iter-00001.gcnn'
        snapshot.write_bytes(b'snapshot')
        path = run / 'checkpoint/state.json'
        state = json.loads(path.read_text())
        state['snapshots'] = [dict(file=snapshot.name, sha256=digest(snapshot))]
        path.write_text(json.dumps(state))
        snapshot.write_bytes(b'corrupt')
        with self.assertRaisesRegex(ValueError, 'Hash mismatch'):
            audit(self.root)
