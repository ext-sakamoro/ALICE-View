#!/usr/bin/env python3
"""The semver-checks job's two decisions, kept out of the workflow so they can be tested.

`baseline` picks the commit the API is compared with: the state before the change, not the
parent of the last commit (a push of several commits whose first one breaks the API would
otherwise be compared with an already broken parent and pass).

    push to main         the commit main pointed to before the push (`github.event.before`);
                         the parent when that is all zeros, not in the history, or not an ancestor
                         of HEAD (main was rewritten); the reason is printed
    pull request         the merge base with the branch it targets
    workflow_dispatch    the merge base with origin/main; on main itself, the last release tag
    schedule             the last release tag reachable from HEAD; the parent when there is none

`decide` reads cargo-semver-checks' output (colour codes removed) and its exit code. The tool is
level-aware itself: check-release reads both Cargo.toml versions ("Checking x vA -> vB (minor
change)") and exits 0 only when the bump covers every lint (a 0.y.z crate follows Cargo's rule:
0.y -> 0.y+1 is major, 0.y.z -> 0.y.z+1 is minor). Measured with a scratch crate:

    minor lint, 1.2.0 -> 1.2.1   exit 100 "requires new minor version"
    minor lint, 1.2.0 -> 1.3.0   exit 0
    major lint, 1.2.0 -> 1.3.0   exit 100 "requires new major version"
    major lint, 0.3.0 -> 0.3.1   exit 100 "requires new major version"
    any lint,   0.3.0 -> 0.4.0   exit 0, 0 checks (the tool runs none across a major bump)

A green run across a major bump does not mean the API is compatible: the tool compares nothing
there, so a removal nobody intended is not found either. The major bump itself declares that the
API may break; read the change list of that release, not this check.

So a non-zero exit always means the bump was not enough, and a version comparison here could
only turn a correct red into green:

    no "Checking ... (<change>)" line                        fail (the tool did not get that far)
    the version goes down (SemVer precedence)                fail (the tool labels 1.2.0 -> 0.9.0 and
                                                             2.0.0 -> 2.0.0-alpha.1 a "major change"
                                                             and checks nothing)
    0 checks after a "(major change)", exit 0                pass (a major release, said in the summary)
    0 checks otherwise                                       fail (it compared nothing)
    exit 0                                                   pass
    "requires new major / minor version"                     fail (the bump is lower than needed)
    any other non-zero exit                                  fail (the tool did not run to the end)

    semver_gate.py baseline <event> <before-sha> <base-ref>   prints the baseline commit
    semver_gate.py decide <log> <exit-code>
"""

from __future__ import annotations

import re
import subprocess
import sys

ZERO = "0" * 40
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
CHECKED_RE = re.compile(r"Checked \[[^\]]*\] +([0-9]+) checks")
CHANGE_RE = re.compile(r"Checking \S+ v(\S+) -> v(\S+) \(([^)]*)\)")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout.strip()


def ok(*args: str) -> bool:
    return subprocess.run(["git", *args], capture_output=True).returncode == 0


def last_tag() -> str | None:
    r = subprocess.run(["git", "describe", "--tags", "--abbrev=0", "HEAD"], capture_output=True, text=True)
    return r.stdout.strip() or None if r.returncode == 0 else None


def baseline_reason(event: str, before: str) -> str:
    """why a push falls back to the parent ("" when `before` is used)"""
    if event != "push":
        return ""
    if not before or before == ZERO:
        return "no commit before the push"
    if not ok("cat-file", "-e", f"{before}^{{commit}}"):
        return f"the commit before the push ({before[:12]}) is not in the history"
    if not ok("merge-base", "--is-ancestor", before, "HEAD"):
        return f"the commit before the push ({before[:12]}) is not an ancestor of HEAD (main was rewritten)"
    if git("rev-parse", before) == git("rev-parse", "HEAD"):
        return "the commit before the push is HEAD itself"
    return ""


def baseline(event: str, before: str, base_ref: str) -> str:
    head = git("rev-parse", "HEAD")
    if event == "push":
        if not baseline_reason(event, before):
            return git("rev-parse", before)
        return git("rev-parse", "HEAD~1")
    if event in ("pull_request", "workflow_dispatch"):
        target = git("rev-parse", base_ref)
        if target != head:
            return git("merge-base", "HEAD", target)
        if event == "pull_request":
            return git("rev-parse", "HEAD~1")
    tag = last_tag()
    if tag:
        commit = git("rev-parse", f"{tag}^{{commit}}")
        if commit != head:
            return commit
    return git("rev-parse", "HEAD~1")


def precedence(version: str) -> tuple:
    """a sort key in SemVer precedence: major.minor.patch, then a release above its pre-releases,
    pre-release identifiers compared one by one (numeric ones as numbers, below alphanumeric ones);
    build metadata is ignored"""
    version = version.split("+", 1)[0]
    core, _, pre = version.partition("-")
    key = tuple(int(x) for x in core.split("."))
    if not pre:
        return key, (1,)
    ids = tuple((0, int(x), "") if x.isdigit() else (1, 0, x) for x in pre.split("."))
    return key, (0, ids)


def decide(log: str, rc: int) -> tuple[bool, str]:
    text = ANSI_RE.sub("", log)
    c = CHANGE_RE.search(text)
    if not c:
        return False, f"cargo-semver-checks did not reach the comparison (exit {rc})"
    change = f"{c.group(1)} -> {c.group(2)} ({c.group(3)})"
    if precedence(c.group(2)) < precedence(c.group(1)):
        return False, f"{change}: the version goes down"
    m = CHECKED_RE.search(text)
    n = int(m.group(1)) if m else 0
    if n == 0:
        if rc == 0 and c.group(3) == "major change":
            return True, (f"{change}: major bump: the tool runs no checks across a major version; "
                          "API break is intended by the version change")
        return False, f"{change}: cargo-semver-checks compared nothing (exit {rc})"
    if rc == 0:
        return True, f"{change}: {n} checks, the version covers the change"
    for kind in ("major", "minor"):
        if f"requires new {kind} version" in text:
            return False, f"{change}: {n} checks, the change requires a new {kind} version"
    return False, f"{change}: cargo-semver-checks failed (exit {rc}) after {n} checks"


def main(argv: list[str]) -> int:
    if len(argv) == 4 and argv[0] == "baseline":
        reason = baseline_reason(argv[1], argv[2])
        if reason:
            print(f"semver baseline: the parent of HEAD, because {reason}", file=sys.stderr)
        print(baseline(argv[1], argv[2], argv[3]))
        return 0
    if len(argv) == 3 and argv[0] == "decide":
        with open(argv[1], encoding="utf-8", errors="replace") as f:
            passed, message = decide(f.read(), int(argv[2]))
        print(f"{'::notice::' if passed else '::error::'}alice-view {message}")
        return 0 if passed else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
