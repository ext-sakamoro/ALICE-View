#!/usr/bin/env python3
"""Tests for .github/semver_gate.py (temporary git repositories; no cargo)."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import semver_gate as g  # noqa: E402

ENV = {**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t",
       "GIT_COMMITTER_EMAIL": "t@t"}


class Repo:
    def __init__(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.path = self.dir.name
        self.git("init", "-q", "-b", "main")
        self.git("config", "maintenance.auto", "false")

    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.path, check=True, capture_output=True, text=True,
                              env=ENV).stdout.strip()

    def commit(self, name: str) -> str:
        Path(self.path, name).write_text(name)
        self.git("add", name)
        self.git("commit", "-q", "-m", name)
        return self.git("rev-parse", "HEAD")

    def baseline(self, event: str, before: str = "", base_ref: str = "main") -> str:
        cwd = os.getcwd()
        os.chdir(self.path)
        try:
            return g.baseline(event, before, base_ref)
        finally:
            os.chdir(cwd)


class Baseline(unittest.TestCase):
    def test_a_push_of_several_commits_is_compared_with_main_before_the_push(self):
        # the first commit of the push changes the API, the last only a document: the parent of
        # the last commit already has the change, the state before the push does not
        r = Repo()
        before = r.commit("a")
        breaking = r.commit("api_change")
        r.commit("doc")
        self.assertEqual(r.baseline("push", before), before)
        self.assertNotEqual(r.baseline("push", before), breaking)

    def test_a_push_without_a_before_commit_falls_back_to_the_parent(self):
        r = Repo()
        r.commit("a")
        parent = r.commit("b")
        r.commit("c")
        self.assertEqual(r.baseline("push", g.ZERO), parent)
        self.assertEqual(r.baseline("push", "1" * 40), parent)  # not in the history

    def test_a_push_after_a_rewrite_of_main_falls_back_to_the_parent(self):
        # `before` exists but is not an ancestor of HEAD (main was rewritten and force-pushed):
        # comparing with it would compare with an unrelated history
        r = Repo()
        r.commit("a")
        r.git("switch", "-q", "-c", "old")
        old_before = r.commit("old_b")
        r.git("switch", "-q", "main")
        parent = r.commit("b")
        r.commit("c")
        self.assertEqual(r.baseline("push", old_before), parent)
        cwd = os.getcwd()
        os.chdir(r.path)
        try:
            self.assertIn("not an ancestor", g.baseline_reason("push", old_before))
        finally:
            os.chdir(cwd)

    def test_a_pull_request_is_compared_with_its_merge_base(self):
        r = Repo()
        base = r.commit("a")
        r.git("switch", "-q", "-c", "topic")
        r.commit("t1")
        r.commit("t2")
        r.git("switch", "-q", "main")
        r.commit("m1")
        r.git("switch", "-q", "topic")
        self.assertEqual(r.baseline("pull_request", "", "main"), base)

    def test_a_dispatch_on_a_branch_uses_the_merge_base_and_on_main_the_last_tag(self):
        r = Repo()
        r.commit("a")
        tagged = r.commit("b")
        r.git("tag", "v0.1.0")
        r.commit("c")
        self.assertEqual(r.baseline("workflow_dispatch", "", "main"), tagged)
        r.git("switch", "-q", "-c", "topic")
        r.commit("t1")
        main_tip = r.git("rev-parse", "main")
        self.assertEqual(r.baseline("workflow_dispatch", "", "main"), main_tip)

    def test_a_schedule_uses_the_last_tag_or_the_parent(self):
        r = Repo()
        r.commit("a")
        parent = r.commit("b")
        r.commit("c")
        self.assertEqual(r.baseline("schedule"), parent)
        r.git("tag", "v0.2.0", "HEAD~2")
        self.assertEqual(r.baseline("schedule"), r.git("rev-parse", "HEAD~2"))


COLOR = "\x1b[1m\x1b[32m"
RESET = "\x1b[0m"
KIND = {("0.3.0", "0.3.0"): "no change; assume minor", ("0.3.0", "0.3.1"): "minor change",
        ("0.3.0", "0.4.0"): "major change", ("1.2.0", "1.2.0"): "no change; assume patch",
        ("1.2.0", "1.2.1"): "patch change", ("1.2.0", "1.3.0"): "minor change",
        ("1.2.0", "2.0.0"): "major change", ("1.2.0", "0.9.0"): "major change",
        ("1.2.0", "1.1.9"): "minor change", ("1.2.0", "2.0.0-alpha.1"): "major change",
        ("2.0.0", "2.0.0-alpha.1"): "major change", ("2.0.0-alpha.2", "2.0.0-alpha.10"): "major change",
        ("2.0.0-alpha.10", "2.0.0-alpha.2"): "major change", ("2.0.0-alpha.1", "2.0.0"): "major change"}
MINOR = "semver requires new minor version: 0 major and 1 minor checks failed"
MAJOR = "semver requires new major version: 1 major and 0 minor checks failed"
NONE = "no semver update required"


def log(base: str | None, head: str = "", checks: int | None = 196, summary: str = NONE) -> str:
    """the lines of cargo-semver-checks' output decide() reads, coloured as with CARGO_TERM_COLOR=always
    (the shapes are from real runs of 0.49 / 0.50 on a scratch crate)"""
    lines = []
    if base is not None:
        lines.append(f"{COLOR}    Checking{RESET} alice-view v{base} -> v{head} ({KIND[base, head]})")
    if checks is not None:
        lines.append(f"{COLOR}     Checked{RESET} [   0.2s] {checks} checks: {checks} pass, 3 skip")
    if summary:
        lines.append(f"{COLOR}     Summary{RESET} {summary}")
    return "\n".join(lines) + "\n"


class Decide(unittest.TestCase):
    # each case is (base -> head, what the tool printed, its exit code) as measured, and the gate
    # must agree with the tool: it is the tool that compares the bump with the lint level

    def test_a_bump_that_covers_the_change_passes(self):
        for base, head in (("1.2.0", "1.3.0"), ("0.3.0", "0.3.0"), ("0.3.0", "0.3.1"), ("1.2.0", "1.2.0")):
            self.assertTrue(g.decide(log(base, head), 0)[0], (base, head))

    def test_a_minor_change_with_a_patch_bump_fails(self):
        ok, message = g.decide(log("1.2.0", "1.2.1", summary=MINOR), 100)
        self.assertFalse(ok)
        self.assertIn("requires a new minor version", message)
        self.assertFalse(g.decide(log("1.2.0", "1.2.0", summary=MINOR), 100)[0])

    def test_a_major_change_with_a_lower_bump_fails(self):
        for base, head in (("1.2.0", "1.3.0"), ("1.2.0", "1.2.1"), ("0.3.0", "0.3.1"), ("0.3.0", "0.3.0")):
            ok, message = g.decide(log(base, head, summary=MAJOR), 100)
            self.assertFalse(ok, (base, head))
            self.assertIn("requires a new major version", message)

    def test_a_major_bump_passes_though_the_tool_checks_nothing(self):
        for base, head in (("0.3.0", "0.4.0"), ("1.2.0", "2.0.0")):
            ok, message = g.decide(log(base, head, checks=0), 0)
            self.assertTrue(ok, (base, head))
            self.assertIn("major bump: the tool runs no checks across a major version", message)
            self.assertIn("API break is intended by the version change", message)

    def test_a_major_bump_to_a_pre_release_passes(self):
        self.assertTrue(g.decide(log("1.2.0", "2.0.0-alpha.1", checks=0), 0)[0])

    def test_a_version_that_goes_down_fails(self):
        # the tool labels a downgrade across a major, and a pre-release below its release, a "major
        # change" and checks nothing
        for base, head in (("1.2.0", "0.9.0"), ("1.2.0", "1.1.9"), ("2.0.0", "2.0.0-alpha.1"),
                           ("2.0.0-alpha.10", "2.0.0-alpha.2")):
            ok, message = g.decide(log(base, head, checks=0), 0)
            self.assertFalse(ok, (base, head))
            self.assertIn("the version goes down", message)

    def test_pre_releases_go_up_in_semver_order(self):
        for base, head in (("2.0.0-alpha.2", "2.0.0-alpha.10"), ("2.0.0-alpha.1", "2.0.0")):
            self.assertTrue(g.decide(log(base, head, checks=0), 0)[0], (base, head))

    def test_compared_nothing_fails(self):
        self.assertFalse(g.decide(log("1.2.0", "1.3.0", checks=0), 0)[0])  # not a major bump
        self.assertFalse(g.decide(log("0.3.0", "0.3.1", checks=None), 0)[0])
        self.assertFalse(g.decide(log("0.3.0", "0.4.0", checks=0, summary=""), 101)[0])
        self.assertFalse(g.decide(log(None), 0)[0])  # no Checking line
        self.assertFalse(g.decide("error: rustc version is not high enough\n", 1)[0])

    def test_another_failure_fails(self):
        self.assertFalse(g.decide(log("0.3.0", "0.3.0", summary="") + "error: failed to build baseline\n", 2)[0])


class Relax(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        for name, ver in (("ALICE-DB", "0.5.0"), ("ALICE-LOL", "0.4.1")):
            (root / name).mkdir()
            (root / name / "Cargo.toml").write_text(f'[package]\nname = "x"\nversion = "{ver}"\n')
        (root / "base").mkdir()
        self.toml = root / "base" / "Cargo.toml"

    def tearDown(self):
        self.tmp.cleanup()

    def test_only_unsatisfied_path_dep_loses_its_path(self):
        self.toml.write_text(
            '[dependencies]\n'
            'alice-db = { version = "0.4", path = "../ALICE-DB", optional = true, default-features = false }\n'
            'alice-lol = { version = ">=0.4.1, <0.6", path = "../ALICE-LOL", optional = true }\n')
        notes = g.relax_baseline(self.toml.parent)
        text = self.toml.read_text()
        self.assertEqual(len(notes), 1)
        self.assertIn("alice-db", notes[0])
        self.assertIn('alice-db = { version = "0.4", optional = true, default-features = false }', text)
        self.assertIn('path = "../ALICE-LOL"', text)

    def test_path_last_and_nothing_to_change(self):
        self.toml.write_text('[dependencies]\nalice-db = { version = "0.4", path = "../ALICE-DB" }\n')
        self.assertEqual(len(g.relax_baseline(self.toml.parent)), 1)
        self.assertIn('alice-db = { version = "0.4" }', self.toml.read_text())
        self.toml.write_text('[dependencies]\nalice-db = { version = "0.5", path = "../ALICE-DB" }\n')
        self.assertEqual(g.relax_baseline(self.toml.parent), [])
        self.assertIn('path = "../ALICE-DB"', self.toml.read_text())


if __name__ == "__main__":
    unittest.main()
