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
    semver_gate.py relax <baseline-dir>   drops `path` from path dependencies the sibling no longer satisfies
"""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

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


# version requirements: caret / tilde / exact, or a comparison range such as `>=0.4.1, <0.6`
SIMPLE_REQ = re.compile(r"^(\^|~|=)?(\d+)(?:\.(\d+))?(?:\.(\d+))?$")
CMP_REQ = re.compile(r"^(>=|>|<=|<)\s*(\d+)(?:\.(\d+))?(?:\.(\d+))?$")


def _triple(m: re.Match) -> list[int]:
    return [int(m.group(i)) if m.group(i) else 0 for i in (2, 3, 4)]


def parse_range(req: str) -> list[tuple[str, list[int]]] | None:
    """`>=a.b.c, <x.y.z` 形式の範囲を (演算子, 3 桁) の列にする 範囲でなければ None"""
    out = []
    for p in (p.strip() for p in req.split(",")):
        m = CMP_REQ.match(p)
        if not m:
            return None
        out.append((m.group(1), _triple(m)))
    return out
DEP_SECTION = re.compile(r"(?:^|\.)(?:dependencies|dev-dependencies|build-dependencies)$")


def satisfies(version: str, req: str) -> bool:
    """`version` が要求 `req` を満たすか (caret / tilde / exact のみ)"""
    rng = parse_range(req.strip())
    if rng is not None:
        got = [int(x) for x in re.split(r"[.\-+]", version)[:3]]
        got += [0] * (3 - len(got))
        ops = {">=": lambda a, b: a >= b, ">": lambda a, b: a > b,
               "<=": lambda a, b: a <= b, "<": lambda a, b: a < b}
        return all(ops[op](got, t) for op, t in rng)
    m = SIMPLE_REQ.match(req.strip())
    if not m:
        return True  # 下限を出す側で loud fail 済
    op = m.group(1) or "^"
    want = [int(m.group(i)) if m.group(i) else None for i in (2, 3, 4)]
    got = [int(x) for x in re.split(r"[.\-+]", version)[:3]]
    got += [0] * (3 - len(got))
    lo = [want[i] or 0 for i in range(3)]
    if got < lo:
        return False
    if op == "=":
        return got[: len([w for w in want if w is not None])] == [
            w for w in want if w is not None]
    if op == "~":
        # `~1.1` は >=1.1.0, <1.2.0 / `~1` は >=1.0.0, <2.0.0
        if want[1] is None:
            return got[0] == want[0]
        return got[0] == want[0] and got[1] == want[1]
    # caret 0 は左端の非 0 成分で上限が決まる
    if want[0] != 0:
        return got[0] == want[0]
    if (want[1] or 0) != 0:
        return got[0] == 0 and got[1] == want[1]
    return got[0] == 0 and got[1] == 0



PATH_DEP_RE = re.compile(r'^(\s*[A-Za-z0-9_-]+\s*=\s*\{[^}\n]*?)\bpath\s*=\s*"([^"]+)"\s*,?\s*([^}\n]*\}.*)$')
VERSION_RE = re.compile(r'\bversion\s*=\s*"([^"]+)"')


def relax_baseline(base_dir: Path) -> list[str]:
    """Make the baseline resolve from what its own Cargo.toml requires.

    The baseline is an older commit, from when the ALICE crates were path dependencies on sibling
    checkouts. This repository now takes them by git, so no sibling is checked out next to the
    baseline: a path dependency whose sibling is absent loses its `path` and is resolved from the
    registry at the requirement the baseline wrote. So does one whose sibling is present but has moved
    past that requirement (alice-db "0.4" against a sibling at 0.5.0). A version the registry does not
    have still fails the run (the job then reports it compared nothing). A dependency whose sibling is
    present and satisfies the requirement keeps its path. Returns one line per dependency changed."""
    toml = base_dir / "Cargo.toml"
    out, changed = [], []
    for line in toml.read_text(encoding="utf-8").splitlines(keepends=True):
        m = PATH_DEP_RE.match(line)
        v = VERSION_RE.search(line) if m else None
        if m and v:
            sib = (base_dir / m.group(2) / "Cargo.toml")
            reason = None
            if not sib.is_file():
                reason = "no sibling checkout"
            else:
                sib_ver = tomllib.loads(sib.read_text(encoding="utf-8"))["package"]["version"]
                if not satisfies(sib_ver, v.group(1)):
                    reason = f"the sibling is {sib_ver}"
            if reason is not None:
                name = line.split("=", 1)[0].strip()
                line = m.group(1) + m.group(3).lstrip() + "\n"
                line = re.sub(r",\s*,", ",", line).replace("{ ,", "{ ").replace(", }", " }")
                changed.append(f"{name} {v.group(1)}: {reason}, resolved from the registry")
        out.append(line)
    toml.write_text("".join(out), encoding="utf-8")
    return changed


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[0] == "relax":
        for note in relax_baseline(Path(argv[1])):
            print(f"semver baseline: {note}")
        return 0
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
