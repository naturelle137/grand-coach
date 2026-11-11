#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Exercise the real shell entrypoint in disposable repositories; no Rust needed."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class Routing(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        # Git hooks export repository/index variables; fixtures own their state.
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        shutil.copytree(ROOT / "scripts", self.repo / "scripts")
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "CI fixture")
        self.write("README.md", "before\n")
        self.write("crates/skat-core/src/lib.rs", "before\n")
        self.write("crates/skat-core/Cargo.toml", 'name = "skat-core"\n[features]\nserde = []\n')
        self.write("tools/example/Cargo.toml", 'name = "example"\n')
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        self.base = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.repo, env=self.env, stderr=subprocess.PIPE).decode()

    def write(self, path, contents="changed\n"):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents)

    def check(self, *args, env=None, ok=True):
        result = subprocess.run(
            ["bash", "scripts/check.sh", *args], cwd=self.repo,
            text=True, capture_output=True, env={**self.env, **(env or {})},
        )
        if ok:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result.stdout

    def route(self, **kwargs):
        args = ["route", "--base", self.base]
        for key, value in kwargs.items():
            args.extend(["--" + key, value])
        return dict(line.split("=", 1) for line in self.check(*args).splitlines())

    def test_docs_and_working_tree(self):
        self.write("docs/guide.md")
        self.git("add", ".")
        self.git("commit", "-qm", "docs")
        self.assertEqual(self.route()["code"], "false")
        self.write("crates/skat-core/src/lib.rs")
        self.assertEqual(self.route()["code"], "true")

    def test_representative_changes(self):
        for path in (
            "crates/skat-core/src/lib.rs", "crates/gui-egui/src/lib.rs",
            "Cargo.toml", "Cargo.lock", "crates/skat-core/build.rs",
            ".cargo/config.toml", "rust-toolchain.toml", "assets/model.bin",
            "scripts/helper.py", ".githooks/pre-push", ".github/workflows/other.yml",
            "unknown/path", ".github/instructions.md",
        ):
            with self.subTest(path=path):
                self.write(path)
                self.git("add", ".")
                self.git("commit", "-qm", path)
                route = self.route()
                self.assertEqual(route["code"], "true")
                if path.startswith(("crates/gui-egui/", "assets/")):
                    self.assertEqual(route["gui"], "true")
                self.base = self.git("rev-parse", "HEAD").strip()

    def test_deleted_and_renamed_paths(self):
        self.git("mv", "crates/skat-core/src/lib.rs", "docs-moved.md")
        self.git("commit", "-qm", "rename code to docs")
        self.assertEqual(self.route()["code"], "true")
        self.git("rm", "README.md")
        self.git("commit", "-qm", "delete docs")
        self.assertEqual(self.route()["code"], "true")
        self.base = self.git("rev-parse", "HEAD~1").strip()
        self.assertEqual(self.route()["code"], "false")

    def test_unknown_empty_and_errors(self):
        self.assertEqual(self.route()["code"], "true")
        self.assertIn("code=true", self.check("route", "--base", "missing"))
        self.check("route", "--event", "typo", ok=False)
        self.write("fake-bin/git", '#!/bin/sh\nif [ "$1" = diff ]; then exit 42; fi\nexec /usr/bin/git "$@"\n')
        (self.repo / "fake-bin/git").chmod(0o755)
        self.check("route", "--base", self.base,
                   env={"PATH": str(self.repo / "fake-bin") + ":" + os.environ["PATH"]}, ok=False)

    def test_event_ownership(self):
        self.write("README.md")
        for event in ("schedule", "workflow_dispatch"):
            route = self.route(event=event)
            self.assertEqual(route["full"], "true")
            self.assertEqual(route["code"], "true")
        self.assertEqual(self.route(event="schedule", schedule="23 3 * * 1")["deep"], "true")
        self.assertIn("deep=true", self.check("route", "--event", "workflow_dispatch", "--deep"))
        self.write("crates/skat-core/src/lib.rs")
        self.assertEqual(self.route(event="push")["regression"], "true")

    def test_deep_entrypoint_and_native_render_failure(self):
        self.write("fake-bin/cargo", '#!/bin/sh\nprintf "%s|%s|%s\\n" "$SKAT_RANDOM_GAMES" "$SKAT_MEPHISTO_GAMES" "$*" >> "$CI_CALLS"\n')
        (self.repo / "fake-bin/cargo").chmod(0o755)
        calls = self.repo / "calls"
        env = {"PATH": str(self.repo / "fake-bin") + ":" + os.environ["PATH"],
               "CI_CALLS": str(calls), "SKAT_RANDOM_GAMES": "", "SKAT_MEPHISTO_GAMES": ""}
        self.check("deep", env=env)
        commands = calls.read_text().splitlines()
        self.assertEqual(len(commands), 2)
        self.assertTrue(all(c.startswith("1000000|100|") and "--release" in c for c in commands))
        # A cargo process that returns success without rendering must not pass native.
        self.check("native", env=env, ok=False)
        self.write("fake-bin/cargo", "#!/bin/sh\nexit 42\n")
        self.check("pr", env=env, ok=False)
        self.check("complete", env=env, ok=False)
        self.check("native", env=env, ok=False)

    def test_required_results_fail_closed(self):
        for invalid in ("{}", "null", "invalid-json"):
            self.check("ci-ok", env={"CI_NEEDS": invalid}, ok=False)
        for event, changed in (("pull_request", "README.md"),
                               ("pull_request", "Cargo.lock"),
                               ("push", "Cargo.lock"),
                               ("workflow_dispatch", "README.md"),
                               ("schedule", "README.md")):
            with self.subTest(event=event, changed=changed):
                self.write(changed, event + changed)
                self.git("add", ".")
                self.git("commit", "-qm", "change")
                route = self.route(event=event, schedule="23 3 * * 1")
                code = route["code"] == "true"
                results = {"changes": {"result": "success", "outputs": route}}
                for job, required in {
                    "lint": True, "portable": code, "native": code, "msrv": code,
                    "regression": route["regression"] == "true",
                    "complete": route["full"] == "true", "deep": route["deep"] == "true",
                }.items():
                    results[job] = {"result": "success" if required else "skipped"}
                self.check("ci-ok", env={"CI_NEEDS": json.dumps(results)})
                for job in results:
                    if results[job]["result"] == "success":
                        for failure in ("failure", "cancelled", "skipped", "", "neutral"):
                            bad = json.loads(json.dumps(results))
                            bad[job]["result"] = failure
                            self.check("ci-ok", env={"CI_NEEDS": json.dumps(bad)}, ok=False)
                for key, value in (("code", ""), ("full", "maybe"), ("gui", "")):
                    bad = json.loads(json.dumps(results))
                    bad["changes"]["outputs"][key] = value
                    self.check("ci-ok", env={"CI_NEEDS": json.dumps(bad)}, ok=False)
                bad = json.loads(json.dumps(results))
                del bad["changes"]["outputs"]["code"]
                self.check("ci-ok", env={"CI_NEEDS": json.dumps(bad)}, ok=False)
                self.base = self.git("rev-parse", "HEAD").strip()
                # Start next route without previous code modifications.


if __name__ == "__main__":
    unittest.main()
