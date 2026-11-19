#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Failure-oriented tests of the actual RC archive format and resource contract."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import package


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.names = package.resources(package.ROOT)
        self.files = {name: (package.ROOT / name).read_bytes() for name in (*package.LEGAL, *self.names, *package.MODELS)}
        self.files.update({"grand-coach": b"test executable", "INSTALL.md": b"install", "THIRD-PARTY.md": b"sources"})
        self.manifest = {"binary": "grand-coach", "embedded_resources": self.names, "models": list(package.MODELS),
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

    def test_missing_or_mismatching_model_fails_even_with_rehashed_manifest(self):
        for name, content in ((package.MODELS[0], None), (package.MODELS[0], b"corrupt"), (package.MODELS[1], None)):
            files = dict(self.files)
            if content is None:
                del files[name]
            else:
                files[name] = content
            self.manifest["files"] = {k: package.digest(v) for k, v in files.items()}
            with tempfile.TemporaryDirectory() as directory:
                with self.assertRaises((ValueError, FileNotFoundError)):
                    package.verify(self.write(directory, self.with_manifest(files)))

    def test_incomplete_model_provenance_or_card_fails_even_with_rehashed_manifest(self):
        record = json.loads(self.files[package.MODELS[1]])
        cases = []
        for key in ("engine_commit", "source_provenance_sha256", "licence"):
            cases.append((package.MODELS[1], json.dumps({k: v for k, v in record.items() if k != key}).encode()))
        cases.append((package.MODELS[2], b"# card without sections\n"))
        for name, content in cases:
            files = {**self.files, name: content}
            self.manifest["files"] = {k: package.digest(v) for k, v in files.items()}
            with tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(ValueError):
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

    def test_runtime_identity_ignores_aslr_but_rejects_missing_libraries(self):
        with patch.object(package, "run", side_effect=[
            "libc.so.6 => /lib/libc.so.6 (0x1234)",
            "libc.so.6 => /lib/libc.so.6 (0xabcdef)",
            "libEGL.so.1 => not found",
        ]):
            first = package.linux_libraries("binary")
            self.assertEqual(first, package.linux_libraries("binary"))
            self.assertIn("/lib/libc.so.6", first)
            with self.assertRaisesRegex(ValueError, "runtime library"):
                package.linux_libraries("binary")

    def test_runner_identity_survives_windows_environment_case_normalization(self):
        expected = {"ImageOS": "win25", "ImageVersion": "20261111.1"}
        self.assertEqual(package.runner_image(expected), expected)
        self.assertEqual(package.runner_image({k.upper(): v for k, v in expected.items()}), expected)

    def test_windows_firewall_requires_reachable_endpoint_and_specific_denial(self):
        program = Path("app.exe")
        with patch.object(package.platform, "system", return_value="Windows"), \
             patch.object(package.subprocess, "run") as run:
            for exit_code in (0, 1, 42):
                run.reset_mock()
                run.return_value.returncode = exit_code
                if exit_code == 42:
                    package.block_windows_network([program])
                else:
                    with self.assertRaisesRegex(ValueError, "not proven"):
                        package.block_windows_network([program])
                self.assertEqual(run.call_count, 5)
                self.assertEqual(run.call_args_list[0].kwargs, {"check": True, "timeout": 15})
                self.assertEqual(run.call_args_list[-1].args[0][2:4], ["firewall", "delete"])
            run.reset_mock()
            run.side_effect = subprocess.CalledProcessError(1, "canary")
            with self.assertRaises(subprocess.CalledProcessError):
                package.block_windows_network([program])
            self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
