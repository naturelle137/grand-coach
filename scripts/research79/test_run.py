# SPDX-License-Identifier: EUPL-1.2
"""Checks the disposable comparison's output boundary on the frozen corpus."""
import unittest

import corpus
import run


class SelectionTest(unittest.TestCase):
    def test_acceptance_and_fallback_in_both_languages(self):
        for case in corpus.corpus():
            with self.subTest(case=case['case'], lang=case['lang']):
                expected = corpus.reference(case)
                good = run.select(case, '|'.join(expected))
                self.assertTrue(good['accepted'])
                self.assertEqual(good['pair'], expected)
                for bad in ('count_exact|99', '<think>secret</think>count_exact|1',
                            '|'.join(expected) + ' extra', '',
                            '[end of text]' + '|'.join(expected),
                            '|'.join(expected) + '[end of text][end of text]'):
                    shown = run.select(case, bad)
                    self.assertFalse(shown['accepted'])
                    self.assertEqual(shown['pair'], expected)
                    self.assertEqual(shown['text'], good['text'])
                self.assertTrue(run.select(case, '|'.join(expected) + '[end of text]')['accepted'])

    def test_frozen_corpus(self):
        self.assertEqual(len(corpus.corpus()), 86)
        self.assertEqual(corpus.digest(),
                         '9aeb384857022c7311558312fe781a7c6924f4310fea44c8837a42f81afcc7ab')


if __name__ == '__main__':
    unittest.main()
