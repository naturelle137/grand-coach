#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Identity, privacy and non-overwrite failures of the actual learner kit builder."""
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

import package

SPEC = importlib.util.spec_from_file_location("kit", Path(__file__).with_name("validation54-kit.py"))
kit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(kit)
inside_git_worktree = kit.inside_git_worktree


class KitTests(unittest.TestCase):
    def setUp(self):
        worktree_check = patch.object(kit, "inside_git_worktree", return_value=False)
        self.addCleanup(worktree_check.stop)
        worktree_check.start()
        self.manifest = {"commit": "c" * 40, "binary": "grand-coach", "files": {"grand-coach": package.digest(b"binary")}}
        self.positions = [{"id": f"validation54.{s}.{f}", "version": 1, "set": s,
                           "position": {"view": {"test": f}, "question": "test"}}
                          for s in ("Initial", "Delayed") for f in range(4)]

    def run_identity(self, *args, **kwargs):
        if args[:3] == ("git", "rev-parse", "HEAD"):
            return "c" * 40
        if args[:3] == ("git", "rev-parse", "HEAD^{tree}"):
            return "t" * 40
        if args[:2] == ("git", "status"):
            return ""
        raise AssertionError(args)

    def test_manifest_hashes_exact_positions_and_blank_records_bind_executable(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "app.tar.gz"
            archive.write_bytes(b"verified archive")
            out = Path(directory) / "private kit"
            original_write = kit.write_new

            def verify_snapshot(path, data):
                path.write_bytes(b"replacement archive")
                return self.manifest, {"grand-coach": b"binary"}

            def private_write(path, data):
                self.assertEqual(stat.S_IMODE(out.stat().st_mode), 0o700)
                original_write(path, data)

            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", side_effect=verify_snapshot), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()), \
                 patch.object(kit, "write_new", side_effect=private_write), \
                 patch.object(kit, "registered_tasks", return_value=[{
                     "id": c["id"], "version": c["version"], "set": c["set"],
                     "position_sha256": package.digest(json.dumps(c["position"], sort_keys=True,
                                                                 separators=(",", ":"), ensure_ascii=False).encode())}
                     for c in self.positions]):
                previous_umask = os.umask(0)
                try:
                    result = kit.prepare(archive, out)
                finally:
                    os.umask(previous_umask)
                self.assertEqual(len(result["tasks"]), 8)
                self.assertEqual(result["archive_sha256"], package.digest(b"verified archive"))
                self.assertNotIn("position", result["tasks"][0])
                expected = package.digest(json.dumps(self.positions[0]["position"], sort_keys=True,
                                                      separators=(",", ":"), ensure_ascii=False).encode())
                self.assertEqual(result["tasks"][0]["position_sha256"], expected)
                template = json.loads((out / "Ergebnisse-lokal.json").read_text())
                self.assertEqual(template["opportunities"], [])
                self.assertEqual(template["executable_sha256"], package.digest(b"binary"))
                self.assertEqual(template["task_manifest_sha256"], package.digest((out / "task-manifest.json").read_bytes()))
                self.assertFalse((out / "private-history/learning-history.json").exists())
                self.assertIn("GRAND_COACH_VALIDATION=1", (out / "Grand-Coach-Lernvalidierung.sh").read_text())
                self.assertFalse(any("key" in str(p.name) or p.suffix == ".rs" for p in out.rglob("*")))
                (out / "private-history/personal.json").write_bytes(b"retain personal data")
                with self.assertRaisesRegex(ValueError, "destination already exists"):
                    kit.prepare(archive, out)
                self.assertEqual((out / "private-history/personal.json").read_bytes(), b"retain personal data")

    def test_changed_reserved_position_fails_before_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "kit"
            archive = Path(directory) / "archive"
            archive.write_bytes(b"archive")
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {})), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()):
                with self.assertRaisesRegex(ValueError, "identity/hash changed"):
                    kit.prepare(archive, out)
            self.assertFalse(out.exists())

    def test_windows_launcher_uses_its_own_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "app.zip"
            archive.write_bytes(b"archive")
            out = Path(directory) / "private kit"
            manifest = dict(self.manifest, binary="grand-coach.exe",
                            files={"grand-coach.exe": package.digest(b"binary")})
            tasks = [{"id": c["id"], "version": c["version"], "set": c["set"],
                      "position_sha256": package.digest(json.dumps(c["position"], sort_keys=True,
                                                                  separators=(",", ":"), ensure_ascii=False).encode())}
                     for c in self.positions]
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(manifest, {"grand-coach.exe": b"binary"})), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()), \
                 patch.object(kit, "registered_tasks", return_value=tasks):
                kit.prepare(archive, out)
            launcher = (out / "Grand-Coach-Lernvalidierung.cmd").read_bytes()
            self.assertIn(b'set "GRAND_COACH_DATA=%~dp0private-history"\r\n', launcher)
            self.assertIn(b'"%~dp0app\\grand-coach.exe"\r\n', launcher)
            self.assertNotIn(os.fsencode(out), launcher)

    @unittest.skipIf(os.name == "nt", "Linux shell launcher requires a POSIX host")
    def test_linux_launcher_uses_moved_kit_and_private_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "app.tar.gz"
            archive.write_bytes(b"archive")
            out = root / "original kit"
            tasks = [{"id": c["id"], "version": c["version"], "set": c["set"],
                      "position_sha256": package.digest(json.dumps(c["position"], sort_keys=True,
                                                                  separators=(",", ":"), ensure_ascii=False).encode())}
                     for c in self.positions]
            binary = b'#!/bin/sh\nprintf "%s\\n%s\\n" "$GRAND_COACH_VALIDATION" "$GRAND_COACH_DATA"\n'
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {"grand-coach": binary})), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()), \
                 patch.object(kit, "registered_tasks", return_value=tasks):
                kit.prepare(archive, out)
            moved = root / "moved kit"
            shutil.move(out, moved)
            result = subprocess.check_output([str(moved / "Grand-Coach-Lernvalidierung.sh")],
                                             cwd=root, text=True).splitlines()
            self.assertEqual(result, ["1", str(moved / "private-history")])

    def test_cli_refuses_dangling_destination_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            out = root / "kit"
            archive = root / "archive"
            archive.write_bytes(b"archive")
            target = root / "missing"
            out.symlink_to(target, target_is_directory=True)
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {})), \
                 patch.object(sys, "argv", ["validation54-kit.py", "--archive", str(archive),
                                            "--out", str(out)]):
                with self.assertRaisesRegex(ValueError, "destination already exists"):
                    kit.main()
            self.assertTrue(out.is_symlink())
            self.assertFalse(target.exists())

    @unittest.skipIf(os.name == "nt", "backslashes are Windows path separators")
    def test_refuses_backslash_in_launcher_path_before_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "kit\\name"
            archive = Path(directory) / "archive"
            archive.write_bytes(b"archive")
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {})):
                with self.assertRaisesRegex(ValueError, "unsupported launcher path"):
                    kit.prepare(archive, out)
            self.assertFalse(out.exists())

    def test_wrong_source_and_dirty_source_fail_before_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "kit"
            archive = Path(directory) / "archive"
            archive.write_bytes(b"archive")
            wrong = dict(self.manifest, commit="other")
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(wrong, {})):
                with self.assertRaisesRegex(ValueError, "commit mismatch"):
                    kit.prepare(archive, out)
            self.assertFalse(out.exists())
            with patch.object(package, "run", side_effect=["c" * 40, "dirty"]):
                with self.assertRaisesRegex(ValueError, "clean committed"):
                    kit.prepare(Path("unused"), out)
            self.assertFalse(out.exists())

    def test_refuses_destination_in_source_even_through_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            alias = root / "alias"
            alias.symlink_to(source, target_is_directory=True)
            archive = root / "archive"
            archive.write_bytes(b"archive")
            with patch.object(package, "ROOT", source), \
                 patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {})):
                for out in (source / "kit", alias / "kit"):
                    with self.subTest(out=out), self.assertRaisesRegex(ValueError, "outside the source repository"):
                        kit.prepare(archive, out)
                    self.assertFalse(out.exists())

    def test_refuses_other_git_worktrees_before_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "archive"
            archive.write_bytes(b"archive")
            other = root / "other"
            other.mkdir()
            alias = root / "alias"
            alias.symlink_to(other, target_is_directory=True)
            exists, is_symlink = Path.exists, Path.is_symlink

            def local_marker(path, check):
                return check(path) if path.name != ".git" or path.is_relative_to(other) else False

            with patch.object(Path, "exists", lambda path: local_marker(path, exists)), \
                 patch.object(Path, "is_symlink", lambda path: local_marker(path, is_symlink)):
                self.assertFalse(inside_git_worktree(other / "nested/kit"))
                self.check_other_worktree_markers(archive, other, alias)

    def check_other_worktree_markers(self, archive, other, alias):
            for marker_is_file in (False, True):
                marker = other / ".git"
                if marker_is_file:
                    marker.write_text("gitdir: elsewhere\n")
                else:
                    marker.mkdir()
                with patch.object(kit, "inside_git_worktree", wraps=inside_git_worktree), \
                     patch.object(package, "run", side_effect=self.run_identity), \
                     patch.object(package, "verify", return_value=(self.manifest, {})):
                    for out in (other / "nested/kit", alias / "kit"):
                        with self.subTest(out=out, marker_is_file=marker_is_file), \
                             self.assertRaisesRegex(ValueError, "outside Git worktrees"):
                            kit.prepare(archive, out)
                        self.assertFalse(out.exists())
                if marker_is_file:
                    marker.unlink()
                else:
                    marker.rmdir()


if __name__ == "__main__":
    unittest.main()
