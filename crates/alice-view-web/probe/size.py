#!/usr/bin/env python3
"""The size gate: what the crate's code comes to in a browser, gzip -9, against its share of a page's budget.

Built with feature `probe`, which calls every part of the API, so this is an upper bound on what a product links
(the Rust standard library's formatting is counted here though a product already has it). The page's arrival budget
is 500 KB before its first rich frame; this crate may take at most BUDGET of it. Fails when a file is missing or empty
(compared nothing).

    python3 probe/size.py            # builds probe/pkg first (probe/build.sh)

Author: Moroya Sakamoto
"""
import gzip
import pathlib
import subprocess
import sys

BUDGET = 50 * 1024
HERE = pathlib.Path(__file__).resolve().parent


def main() -> int:
    subprocess.run([str(HERE / "build.sh")], check=True)
    files = [HERE / "pkg" / "alice_view_web_bg.wasm", HERE / "pkg" / "alice_view_web.js"]
    total, compared = 0, 0
    for f in files:
        if not f.is_file() or f.stat().st_size == 0:
            print(f"FAIL {f.name}: missing or empty")
            return 1
        z = len(gzip.compress(f.read_bytes(), 9))
        total += z
        compared += 1
        print(f"{f.name}: {f.stat().st_size} bytes, {z} gzip -9")
    if compared == 0:
        print("FAIL compared nothing")
        return 1
    ok = total <= BUDGET
    print(f"{'ok  ' if ok else 'FAIL'} {total} bytes gzip -9 (budget {BUDGET}, of the page's 500 KB)")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
