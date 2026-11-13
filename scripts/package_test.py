#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Failure-oriented tests of the actual RC archive format and resource contract."""

import json
from pathlib import Path
import tempfile
import unittest

import package


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.names = package.resources(package.ROOT)
        self.files = {name: (package.ROOT / name).read_bytes() for name in (*package.LEGAL, *self.names)}
        self.files.update({"grand-coach": b"test executable", "INSTALL.md": b"install", "THIRD-PARTY.md": b"sources"})
        self.manifest = {"binary": "grand-coach", "embedded_resources": self.names,
                         "files": {k: package.digest(v) for k, v in self.files.items()}}

    def write(self, directory, files, windows=False):
        path = Path(directory) / ("test.zip" if windows else "test.tar.gz")
        path.write_bytes(package.archive_bytes(files, windows, 1700000000))
        return path

    def with_manifest(self, files):
        return {**files, "manifest.json": json.dumps(self.manifest).encode()}

    def test_both_formats_repeat_and_roundtrip_real_resources(self):
        self.assertEqual(len(self.names), 317)  # 291 cards + 24 portraits + de/en
        for windows in (False, True):
            files = self.with_manifest(self.files)
            self.assertEqual(package.archive_bytes(files, windows, 1700000000),
                             package.archive_bytes(dict(reversed(list(files.items()))), windows, 1700000000))
            with tempfile.TemporaryDirectory() as directory:
                path = self.write(directory, files, windows)
                self.assertEqual(package.verify(path)[1], self.files)

    def test_missing_notices_translation_and_card_fail_even_with_rehashed_manifest(self):
        for missing in ("THIRD-PARTY-NOTICES", "assets/i18n/en.ftl", "assets/cards/french2/kreuz-ass.svg"):
            # Select an actual face filename, independent of spelling conventions.
            if missing.endswith("kreuz-ass.svg"):
                missing = next(n for n in self.names if n.startswith("assets/cards/french2/"))
            files = dict(self.files)
            del files[missing]
            self.manifest["files"] = {k: package.digest(v) for k, v in files.items()}
            with tempfile.TemporaryDirectory() as directory:
                with self.assertRaises((ValueError, FileNotFoundError)):
                    package.verify(self.write(directory, self.with_manifest(files)))

    def test_changed_bytes_extra_files_and_unsafe_paths_fail(self):
        for name, content in (("grand-coach", b"tampered"), ("extra", b"unexpected"), ("../escape", b"bad")):
            files = {**self.files, name: content}
            for windows in (False, True):
                with tempfile.TemporaryDirectory() as directory:
                    with self.assertRaises(ValueError):
                        package.verify(self.write(directory, self.with_manifest(files), windows))

    def test_stale_notices_fail(self):
        files = {**self.files, "THIRD-PARTY-NOTICES": b"stale"}
        self.manifest["files"] = {k: package.digest(v) for k, v in files.items()}
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "Cargo.lock"):
                package.verify(self.write(directory, self.with_manifest(files)))


if __name__ == "__main__":
    unittest.main()
