#!/usr/bin/env python3
"""Tests for probe/wasm_bindgen_match.py.

    python3 probe/test_wasm_bindgen_match.py
"""
import importlib.util
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("wbm", HERE / "wasm_bindgen_match.py")
wbm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wbm)


class Pure(unittest.TestCase):
    def test_a_different_cli_names_both_versions_and_the_install_command(self):
        msg = wbm.mismatch("0.2.130", "0.2.129")
        self.assertIsNotNone(msg)
        self.assertIn("0.2.129", msg)
        self.assertIn("0.2.130", msg)
        self.assertIn("cargo install -f wasm-bindgen-cli --version 0.2.130", msg)

    def test_the_same_version_passes(self):
        self.assertIsNone(wbm.mismatch("0.2.130", "0.2.130"))

    def test_the_cli_version_is_read_from_its_output(self):
        self.assertEqual(wbm.cli_version("wasm-bindgen 0.2.130\n"), "0.2.130")
        self.assertIsNone(wbm.cli_version("something else\n"))
        self.assertIsNone(wbm.cli_version(""))

    def test_the_crate_version_is_read_from_cargo_metadata(self):
        meta = {"packages": [{"name": "serde", "version": "1.0.0"}, {"name": "wasm-bindgen", "version": "0.2.130"}]}
        self.assertEqual(wbm.resolved_version(meta), "0.2.130")
        self.assertIsNone(wbm.resolved_version({"packages": []}))


class Cli(unittest.TestCase):
    """the script with a stand-in `wasm-bindgen` first on PATH, against this crate's real `cargo metadata`"""

    def run_with_cli(self, printed: str) -> subprocess.CompletedProcess:
        d = tempfile.mkdtemp()
        fake = Path(d) / "wasm-bindgen"
        fake.write_text(f"#!/bin/sh\necho '{printed}'\n")
        fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
        env = {**os.environ, "PATH": f"{d}{os.pathsep}{os.environ['PATH']}"}
        return subprocess.run([sys.executable, str(HERE / "wasm_bindgen_match.py")], cwd=HERE.parent, env=env,
                              capture_output=True, text=True)

    def test_a_cli_of_another_version_fails_naming_both(self):
        crate = wbm.resolved_version(__import__("json").loads(subprocess.run(
            ["cargo", "metadata", "--format-version", "1"], cwd=HERE.parent, capture_output=True, text=True,
            check=True).stdout))
        self.assertIsNotNone(crate)
        r = self.run_with_cli("wasm-bindgen 0.0.1")
        self.assertEqual(r.returncode, 1, r.stderr)
        self.assertIn("0.0.1", r.stderr)
        self.assertIn(crate, r.stderr)

    def test_a_cli_of_the_resolved_version_passes(self):
        crate = wbm.resolved_version(__import__("json").loads(subprocess.run(
            ["cargo", "metadata", "--format-version", "1"], cwd=HERE.parent, capture_output=True, text=True,
            check=True).stdout))
        r = self.run_with_cli(f"wasm-bindgen {crate}")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_an_unreadable_cli_version_is_exit_2(self):
        r = self.run_with_cli("no version here")
        self.assertEqual(r.returncode, 2, r.stderr)


if __name__ == "__main__":
    unittest.main()
