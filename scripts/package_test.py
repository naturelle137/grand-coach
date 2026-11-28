#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Failure-oriented tests of the actual RC archive format and resource contract."""

import importlib.util
import json
import subprocess
from types import SimpleNamespace
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package


class MeasurementTests(unittest.TestCase):
    def test_hung_gui_is_killed_and_reaped(self):
        spec = importlib.util.spec_from_file_location("package_measure", Path(__file__).with_name("package-measure.py"))
        measure = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(measure)
        with patch.object(measure.subprocess, "Popen") as popen, \
                patch.object(measure.time, "monotonic", side_effect=[0, 61]):
            process = popen.return_value
            process.poll.return_value = None
            with self.assertRaisesRegex(ValueError, "timed out"):
                measure.gui_run(Path("app"), False, {}, ".", 0)
            process.kill.assert_called_once()
            process.wait.assert_called_once()


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.names = package.resources(package.ROOT)
        self.files = {name: (package.ROOT / name).read_bytes() for name in (*package.LEGAL, *self.names, *package.MODELS, *package.PRE_PLAY)}
        self.files.update({"grand-coach": b"test executable", "INSTALL.md": b"install", "THIRD-PARTY.md": b"sources"})
        self.manifest = {"binary": "grand-coach", "embedded_resources": self.names, "models": [*package.MODELS, *package.PRE_PLAY],
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
                snapshot = path.read_bytes()
                path.write_bytes(b"replaced")
                self.assertEqual(package.verify(path, snapshot)[1], self.files)

    def test_release_manifest_is_deterministic_and_lists_absent_checks(self):
        inner = {"channel": "private-rc", "version": "1.0.0", "commit": "c" * 40, "rustc": "rustc 1.95.0",
                 "cargo": "cargo 1.95.0", "target": "x86_64-unknown-linux-gnu", "os": "Linux", "runner_image": {}}
        archive, data = Path("pkg.tar.gz"), b"archive bytes"
        first = package.release_manifest(archive, data, inner)
        self.assertEqual(first, package.release_manifest(archive, data, dict(reversed(list(inner.items())))))
        self.assertEqual(first["artifacts"], [{"name": "pkg.tar.gz", "sha256": package.digest(data), "bytes": 13}])
        self.assertEqual(first["checks"], {"archive-verify": "passed", "archive-reproducible": "passed",
                                           "native-smoke": "not run", "network-isolation": "not run"})
        smoke = {"archive_sha256": package.digest(data), "result": "passed", "os": "Linux",
                 "checks": ["launch"], "limitations": "none", "network_isolation": "not verified by this smoke"}
        done = package.release_manifest(archive, data, inner, smoke)
        self.assertEqual(done["checks"]["native-smoke"], "passed")
        self.assertEqual(done["checks"]["network-isolation"], "not run")
        smoke["network_isolation"] = "program-scoped outbound block"
        self.assertEqual(package.release_manifest(archive, data, inner, smoke)["checks"]["network-isolation"], "passed")
        with self.assertRaises(ValueError):
            package.release_manifest(archive, b"other", inner, smoke)

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

    def test_firewall_setup_failures_remove_every_created_rule(self):
        for failure in ("second_add", "canary"):
            active = set()
            def run(command, **kwargs):
                name = next(arg[5:] for arg in command if arg.startswith("name="))
                if "add" in command:
                    if failure == "second_add" and active:
                        raise subprocess.CalledProcessError(1, command)
                    active.add(name)
                else:
                    active.remove(name)
            def smoke_run(args, rules):
                package.block_windows_network(Path("install"), [Path("app"), Path("probe")], rules)
            with self.subTest(failure=failure), patch.object(package, "smoke_run", side_effect=smoke_run), \
                 patch.object(package.platform, "system", return_value="Windows"), \
                 patch.dict(package.os.environ, {"SystemRoot": "windows"}), \
                 patch.object(package.shutil, "copy2"), patch.object(package.subprocess, "run", side_effect=run), \
                 patch.object(package, "canary_reaches_network", side_effect=[True, failure == "canary"]):
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    package.smoke(None)
            self.assertEqual(active, set())

    def test_smoke_runs_all_probes_from_blocked_install_and_propagates_failure(self):
        for failing_mode in (None, "create", "check", "quiz-answer", "legacy-training"):
            with self.subTest(failing_mode=failing_mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                archive = root / "package.zip"
                archive.write_bytes(b"archive")
                (root / "persistence-probe.exe").write_bytes(b"probe")
                args = SimpleNamespace(archive=archive, out=root / "out", windows_mesa=None, block_network=True)
                manifest = {"binary": "grand-coach.exe", "models": list(package.MODELS), "commit": "test"}
                files = {"grand-coach.exe": b"app", **{name: self.files[name] for name in package.MODELS}}
                blocked = []
                calls = []
                def block(install, programs, rules):
                    blocked.extend(programs)
                    rules.append("rule")
                def run(command, **kwargs):
                    executable = Path(command[0])
                    if executable.name == "persistence-probe.exe":
                        calls.append((executable, command[1]))
                        self.assertIn(executable, blocked)
                        if command[1] == failing_mode:
                            raise subprocess.CalledProcessError(1, command)
                        if command[1] == "create":
                            Path(command[2]).write_bytes(b'{"events": []}')
                    else:
                        screenshot = Path(kwargs["env"]["GRAND_COACH_SCREENSHOT"])
                        screenshot.write_bytes(b"P6\n" + screenshot.name.encode() + b"x" * 100000)
                        if kwargs["env"].get("GRAND_COACH_SMOKE_SCENARIO") == "training":
                            history = Path(kwargs["env"]["GRAND_COACH_DATA"]) / "learning-history.json"
                            if screenshot.name == "legacy-training.ppm":
                                migrated = {**package.LEGACY_HISTORY, "format": 6,
                                            "events": package.LEGACY_HISTORY["events"] + [{}]}
                                if failing_mode == "legacy-training":
                                    migrated["events"] = [{}]
                                history.write_text(json.dumps(migrated))
                            else:
                                history.write_bytes(b"training history")
                        if kwargs["env"].get("GRAND_COACH_SMOKE_SCENARIO") == "quiz-answer":
                            (Path(kwargs["env"]["GRAND_COACH_DATA"]) / "learning-history.json").write_bytes(
                                b'{"events": [{"event": {"action": {"Answer": {"given": 1}}}}]}'
                                if failing_mode == "quiz-answer" else
                                b'{"events": [{"event": {"action": {"Answer": {"given": 0}}}}]}')
                with patch.object(package.platform, "platform", return_value="test"), \
                     patch.object(package, "verify", return_value=(manifest, files)), \
                     patch.object(package, "block_windows_network", side_effect=block), \
                     patch.object(package, "unblock_windows_network") as cleanup, \
                     patch.object(package.subprocess, "run", side_effect=run):
                    if failing_mode:
                        with self.assertRaises(ValueError if failing_mode in ("quiz-answer", "legacy-training") else subprocess.CalledProcessError):
                            package.smoke(args)
                    else:
                        package.smoke(args)
                        self.assertIn("create", [mode for _, mode in calls])
                        self.assertIn("check", [mode for _, mode in calls])
                    cleanup.assert_called_once_with(["rule"])


if __name__ == "__main__":
    unittest.main()
