#!/usr/bin/env python3
"""Fail before the build when the wasm-bindgen CLI is not the version of the wasm-bindgen crate the build resolves.

    python3 probe/wasm_bindgen_match.py      (from crates/alice-view-web; probe/build.sh runs it first)

The CLI refuses a .wasm whose schema version differs from its own ("rust Wasm file schema version: 0.2.130, this
binary schema version: 0.2.129"), and it says so only after the whole release build. This reads the resolved crate
version from `cargo metadata` and the CLI's from `wasm-bindgen --version`, and exits 1 at once naming both and the
install command. Exit 2 when either version cannot be read (a check that reads nothing proves nothing).
"""
import json
import re
import subprocess
import sys


def resolved_version(metadata: dict) -> str | None:
    """the wasm-bindgen crate version in `cargo metadata` output, or None"""
    found = {p["version"] for p in metadata.get("packages", []) if p.get("name") == "wasm-bindgen"}
    return found.pop() if len(found) == 1 else None


def cli_version(output: str) -> str | None:
    """the version in `wasm-bindgen --version` output (`wasm-bindgen 0.2.130`), or None"""
    m = re.fullmatch(r"\s*wasm-bindgen\s+(\d+\.\d+\.\d+)\S*\s*", output)
    return m.group(1) if m else None


def mismatch(crate: str, cli: str) -> str | None:
    """the message when the two versions differ, else None"""
    if crate == cli:
        return None
    return (f"wasm-bindgen CLI {cli} does not match the wasm-bindgen crate {crate} this build resolves: "
            f"cargo install -f wasm-bindgen-cli --version {crate}")


def main() -> int:
    meta = subprocess.run(["cargo", "metadata", "--format-version", "1"], capture_output=True, text=True)
    crate = resolved_version(json.loads(meta.stdout)) if meta.returncode == 0 else None
    try:
        out = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True).stdout
    except FileNotFoundError:
        out = ""
    cli = cli_version(out)
    if crate is None or cli is None:
        print(f"wasm_bindgen_match: could not read a version (crate {crate}, CLI {cli or 'not found'})", file=sys.stderr)
        return 2
    msg = mismatch(crate, cli)
    if msg:
        print(f"wasm_bindgen_match: {msg}", file=sys.stderr)
        return 1
    print(f"wasm_bindgen_match: crate and CLI are both {crate}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
