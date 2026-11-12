# SPDX-License-Identifier: EUPL-1.2
"""Regression fixtures for pairing, correlation and odd defender order."""
import unittest
from pathlib import Path
import tempfile
from campaign import repair_tsv
from analyze import interval, role_values


def row(deal, match, score, swapped=False, mismatch=0):
    defenders = ['medium', 'candidate'] if swapped else ['candidate', 'medium']
    points = [0, score] if swapped else [score, 0]
    return [str(deal), str(match), '0', 'Grand', 'None', '1', str(score),
            *map(str, points), *defenders, str(mismatch)]


class EvidenceTests(unittest.TestCase):
    def test_deal_is_independent_unit_and_defender_name_handles_swap(self):
        rows = [row(d, k, 10 if k < 3 else 4, bool(d % 2)) for d in [1, 2] for k in range(6)]
        strata, excluded = role_values(rows)
        self.assertEqual(strata['overall:all']['n'], 2)
        self.assertEqual(strata['role:declarer']['n'], 2)
        self.assertEqual(strata['role:defender']['mean'], 6)
        self.assertEqual(excluded, 0)

    def test_missing_pair_is_rejected_and_mismatch_is_counted(self):
        with self.assertRaises(AssertionError):
            role_values([row(1, 0, 1)])
        rows = [row(1, k, 1, mismatch=int(k == 0)) for k in range(6)]
        strata, excluded = role_values(rows)
        self.assertEqual(excluded, 1)
        self.assertNotIn('overall:all', strata)

    def test_resume_preserves_a_truncated_tail_but_rejects_interior_damage(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'arena.tsv'
            good = '\t'.join(row(1, 0, 10)).encode() + b'\n'
            p.write_bytes(good + b'2\t0\tpartial')
            repair_tsv(p)
            self.assertEqual(p.read_bytes(), good)
            self.assertEqual(p.with_suffix('.truncated-tail').read_bytes(), b'2\t0\tpartial')
            p.write_bytes(b'broken\n' + good)
            with self.assertRaises(AssertionError):
                repair_tsv(p)

    def test_uncertainty_can_remain_inconclusive(self):
        self.assertEqual(interval([-10, 10])['verdict'], 'inconclusive')
        self.assertEqual(interval([1])['verdict'], 'inconclusive')


if __name__ == '__main__':
    unittest.main()
