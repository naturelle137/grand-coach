#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Identity, privacy and non-overwrite failures of the actual learner kit builder."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package

SPEC = importlib.util.spec_from_file_location("kit", Path(__file__).with_name("validation54-kit.py"))
kit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(kit)


class KitTests(unittest.TestCase):
    def setUp(self):
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
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {"grand-coach": b"binary"})), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()), \
                 patch.object(kit, "registered_tasks", return_value=[{
                     "id": c["id"], "version": c["version"], "set": c["set"],
                     "position_sha256": package.digest(json.dumps(c["position"], sort_keys=True,
                                                                 separators=(",", ":"), ensure_ascii=False).encode())}
                     for c in self.positions]):
                result = kit.prepare(archive, out)
                self.assertEqual(len(result["tasks"]), 8)
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
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(self.manifest, {})), \
                 patch.object(kit.subprocess, "check_output", return_value=json.dumps(self.positions).encode()):
                with self.assertRaisesRegex(ValueError, "identity/hash changed"):
                    kit.prepare(Path("unused"), out)
            self.assertFalse(out.exists())

    def test_wrong_source_and_dirty_source_fail_before_installation(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "kit"
            wrong = dict(self.manifest, commit="other")
            with patch.object(package, "run", side_effect=self.run_identity), \
                 patch.object(package, "verify", return_value=(wrong, {})):
                with self.assertRaisesRegex(ValueError, "commit mismatch"):
                    kit.prepare(Path("unused"), out)
            self.assertFalse(out.exists())
            with patch.object(package, "run", side_effect=["c" * 40, "dirty"]):
                with self.assertRaisesRegex(ValueError, "clean committed"):
                    kit.prepare(Path("unused"), out)
            self.assertFalse(out.exists())


if __name__ == "__main__":
    unittest.main()
