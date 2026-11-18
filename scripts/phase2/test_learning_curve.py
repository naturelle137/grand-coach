# SPDX-License-Identifier: EUPL-1.2
import unittest

from learning_curve import parse, summarize

LOG = (
    'iteration 1: 10 decisions, reward +19.0, vs pool [easy +2.0 (5), snap+easy -4.0 (3)],'
    ' value RMSE 1.0, entropy 0.4, KL(ref) 0.01, clipped 1 %\n'
    'noise\n'
    'iteration 2: 10 decisions, reward +21.0, vs pool [easy +4.0 (5)],'
    ' value RMSE 1.0, entropy 0.4, KL(ref) 0.03, clipped 1 %\n'
)


class LearningCurve(unittest.TestCase):
    def test_window_means(self):
        rows = parse(LOG)
        self.assertEqual(len(rows), 2)
        s = summarize(rows, 2)[0]
        self.assertEqual(s['iterations'], [1, 2])
        self.assertEqual(s['reward'], 20.0)
        self.assertEqual(s['kl_ref'], 0.02)
        self.assertEqual(s['pool'], {'easy': 3.0, 'snap+easy': -4.0})


if __name__ == '__main__':
    unittest.main()
