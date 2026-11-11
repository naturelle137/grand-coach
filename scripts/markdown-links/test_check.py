#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Black-box CLI tests in isolated Git repositories; never use the worktree index."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
FIXTURE = json.loads((HERE / "fixtures.json").read_text(encoding="utf-8"))


class LinkCheckerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # Hooks may export GIT_INDEX_FILE/GIT_DIR; fixtures own their Git state.
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        self.git("init", "--quiet")
        for name, text in FIXTURE["files"].items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        self.git("add", ".")
        (self.root / "scratch.md").write_text("[broken](missing.md)\n", encoding="utf-8")

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, env=self.env,
                              check=True, capture_output=True)

    def run_check(self):
        return subprocess.run([sys.executable, str(HERE / "check.py"),
                               "--root", str(self.root)], env=self.env,
                              capture_output=True, text=True)

    def test_valid_links_and_ignored_examples(self):
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertIn("6 tracked documents", result.stdout)
        self.assertIn("0 errors", result.stdout)
        self.assertIn("20 local links", result.stdout)
        self.assertEqual(result.stdout, self.run_check().stdout)

    def test_broken_targets_report_location_and_exit_failure(self):
        for case in FIXTURE["failures"]:
            with self.subTest(case=case):
                source = self.root / case["source"]
                original = source.read_text(encoding="utf-8")
                source.write_text(original + "\n" + case["link"] + "\n", encoding="utf-8")
                result = self.run_check()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                line = original.count("\n") + 2
                self.assertIn(f'{case["source"]}:{line}:1:', result.stderr)
                self.assertIn(repr(case["target"]), result.stderr)
                self.assertIn(case["reason"], result.stderr)
                source.write_text(original, encoding="utf-8")

    def test_tracked_source_deleted_from_worktree_fails(self):
        (self.root / ".github/pull_request_template.md").unlink()
        result = self.run_check()
        self.assertEqual(result.returncode, 1)
        self.assertIn(".github/pull_request_template.md:1: missing tracked", result.stderr)

    def test_empty_repository(self):
        self.git("rm", "-r", "--cached", ".")
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("0 tracked documents", result.stdout)


if __name__ == "__main__":
    unittest.main()
