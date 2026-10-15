#!/usr/bin/env python3
"""This repository neither depends on rkyv directly nor uses it in Rust code.

RUSTSEC-2026-0235 (rkyv 0.7 archive validation of Rc / Arc) is ignored in the
cargo audit / cargo-deny configuration of a crate that reaches rkyv only
through a dependency whose archived types hold no Rc / Arc. That ignore is
sound only while this repository itself stays out of rkyv: a direct
dependency, or an archived type defined here, would be a new exposure the
ignore silently covers. This script fails as soon as either appears. The file
is generic and is kept byte-identical across the repositories that carry the
same ignore.

Manifests: every Cargo.toml under the repository root (the root package, any
workspace member, and also nested unpublished crates such as fuzz/ or
generators under scripts/ -- they are not skipped, so the rule has no
exception to maintain). A manifest is a finding when it declares rkyv in

    [dependencies] [dev-dependencies] [build-dependencies]
    (and the legacy dev_dependencies / build_dependencies spellings)
    [target.<cfg>.<any of the above>]  [workspace.dependencies]
    [patch.<registry>]  [replace]

either under the key `rkyv` (plain, inline table, dotted `rkyv.version`, or a
`[dependencies.rkyv]` table) or renamed through `package = "rkyv"`. The
manifest is parsed with tomllib (the authority) and located line by line; if
the two disagree the manifest is reported as `unlocated` rather than passed.

Rust: every .rs file under the repository root (src/, tests/, benches/,
examples/, build.rs, fuzz targets, nested crates). Comments (line, block,
nested, doc) and string / raw-string / byte-string / char literals are blanked
first, so a comment or a string mentioning rkyv (a file name `seg_1.rkyv`) is
not a finding. Rules, one per occurrence:

    use-rkyv           `use rkyv...` / `use ::rkyv...`
    extern-crate-rkyv  `extern crate rkyv`
    rkyv-path          `rkyv::...`
    rkyv-attr          `#[rkyv(...)]` (rkyv 0.8 attribute)
    rkyv-ident         any other `rkyv` identifier in code (a re-export
                       `other::rkyv`, a field or method named rkyv)
    derive-archive     `Archive` inside `#[derive(...)]`
    archive-attr       `#[archive(...)]` / `#[archive_attr(...)]`
    archived-type      `Archived<`

Skipped directories: target/, node_modules/ and any directory whose name
starts with a dot (.git, .github, ...).

Output: `path:line: rule` per finding (no matched text), then the counts.
Exit 0 clean, 1 findings, 2 when no manifest or no .rs file was read.

    python3 scripts/no_direct_rkyv.py [ROOT]
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    sys.exit("no_direct_rkyv: needs Python 3.11+ (tomllib)")

ROOT = Path(__file__).resolve().parents[1]
CRATE = "rkyv"
SKIP_DIRS = {"target", "node_modules"}
DEP_TABLES = {
    "dependencies",
    "dev-dependencies",
    "dev_dependencies",
    "build-dependencies",
    "build_dependencies",
}


# --------------------------------------------------------------------------
# file walk
# --------------------------------------------------------------------------
def walk(root: Path, pred) -> list[Path]:
    out: list[Path] = []
    stack = [root]
    while stack:
        d = stack.pop()
        try:
            entries = sorted(d.iterdir())
        except OSError:
            continue
        for p in entries:
            if p.is_dir():
                if p.name in SKIP_DIRS or p.name.startswith("."):
                    continue
                stack.append(p)
            elif pred(p):
                out.append(p)
    return sorted(out)


# --------------------------------------------------------------------------
# manifests
# --------------------------------------------------------------------------
_KEY_PART = r'(?:"[^"]*"|\'[^\']*\'|[A-Za-z0-9_-]+)'
_KEY_PATH = _KEY_PART + r"(?:\s*\.\s*" + _KEY_PART + r")*"
_HEADER = re.compile(r"^\s*\[\[?\s*(" + _KEY_PATH + r")\s*\]\]?")
_KEYLINE = re.compile(r"^\s*(" + _KEY_PATH + r")\s*=\s*(.*)$")
_PACKAGE_INLINE = re.compile(r"\bpackage\s*=\s*([\"'])" + CRATE + r"\1")
_STRING_VALUE = re.compile(r"^([\"'])" + CRATE + r"\1")


def _parts(path: str) -> list[str]:
    return [p.strip("\"'") for p in re.findall(_KEY_PART, path)]


def _dep(path: list[str]):
    """(kind, key) when `path` reaches into one dependency entry, else None.

    key identifies the entry (table path + dependency name), rest is what
    follows the name.
    """
    if len(path) >= 2 and path[0] in DEP_TABLES:
        return path[0], tuple(path[:2]), path[2:]
    if len(path) >= 4 and path[0] == "target" and path[2] in DEP_TABLES:
        return "target-" + path[2], tuple(path[:4]), path[4:]
    if len(path) >= 3 and path[:2] == ["workspace", "dependencies"]:
        return "workspace-dependencies", tuple(path[:3]), path[3:]
    if len(path) >= 3 and path[0] == "patch":
        return "patch", tuple(path[:3]), path[3:]
    if len(path) >= 2 and path[0] == "replace":
        return "replace", tuple(path[:2]), path[2:]
    return None


def _name(key: tuple) -> str:
    return key[-1].split(":", 1)[0]  # [replace] keys are "name:version"


def _semantic(doc: dict) -> set[tuple]:
    """Dependency entries naming rkyv, from the parsed manifest."""
    hits: set[tuple] = set()

    def table(prefix: tuple, deps):
        if not isinstance(deps, dict):
            return
        for name, spec in deps.items():
            key = prefix + (name,)
            if _name(key) == CRATE:
                hits.add(key)
            elif isinstance(spec, dict) and spec.get("package") == CRATE:
                hits.add(key)

    for t in DEP_TABLES:
        table((t,), doc.get(t))
    for cfg, body in (doc.get("target") or {}).items():
        if isinstance(body, dict):
            for t in DEP_TABLES:
                table(("target", cfg, t), body.get(t))
    table(("workspace", "dependencies"), (doc.get("workspace") or {}).get("dependencies"))
    for reg, body in (doc.get("patch") or {}).items():
        table(("patch", reg), body)
    table(("replace",), doc.get("replace"))
    return hits


def _strip_toml_comment(line: str) -> str:
    quote = None
    for i, ch in enumerate(line):
        if quote:
            if ch == "\\" and quote == '"':
                continue
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
        elif ch == "#":
            return line[:i]
    return line


def _located(text: str) -> dict[tuple, tuple[int, str]]:
    """Dependency entries naming rkyv, with the first line and the kind."""
    found: dict[tuple, tuple[int, str]] = {}
    table: list[str] = []
    in_multiline = None
    for n, raw in enumerate(text.splitlines(), 1):
        if in_multiline:
            if raw.count(in_multiline) % 2 == 1:
                in_multiline = None
            continue
        line = _strip_toml_comment(raw)
        for delim in ('"""', "'''"):
            if line.count(delim) % 2 == 1:
                in_multiline = delim
        m = _HEADER.match(line)
        if m:
            table = _parts(m.group(1))
            d = _dep(table)
            if d and not d[2] and _name(d[1]) == CRATE:
                found.setdefault(d[1], (n, d[0]))
            continue
        m = _KEYLINE.match(line)
        if not m:
            continue
        full = table + _parts(m.group(1))
        value = m.group(2).strip()
        d = _dep(full)
        if not d:
            continue
        kind, key, rest = d
        if _name(key) == CRATE:
            found.setdefault(key, (n, kind))
        elif not rest and _PACKAGE_INLINE.search(value):
            found.setdefault(key, (n, kind + " renamed"))
        elif rest == ["package"] and _STRING_VALUE.match(value):
            found.setdefault(key, (n, kind + " renamed"))
    return found


def check_manifest(path: Path, rel: str) -> list[str]:
    text = path.read_text(encoding="utf-8")
    try:
        doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return [f"{rel}:0: unparsable-manifest"]
    want = _semantic(doc)
    got = _located(text)
    out = []
    for key in sorted(want & set(got), key=lambda k: got[k][0]):
        n, kind = got[key]
        out.append(f"{rel}:{n}: rkyv-dependency ({kind})")
    for _ in want - set(got):
        out.append(f"{rel}:0: rkyv-dependency (unlocated)")
    for key in set(got) - want:
        out.append(f"{rel}:{got[key][0]}: unlocated (line scan and parser disagree)")
    return out


# --------------------------------------------------------------------------
# Rust sources
# --------------------------------------------------------------------------
_RAW_OPEN = re.compile(r'(?:br|cr|r)(#*)"')


def blank(src: str) -> str:
    """Replace comments and literal contents by spaces, keeping newlines."""
    out = list(src)
    n = len(src)
    i = 0

    def wipe(a: int, b: int):
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    def ident_char(c: str) -> bool:
        return c.isalnum() or c == "_"

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            wipe(i, j)
            i = j
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            wipe(i, j)
            i = j
        elif c in "rbc" and (i == 0 or not ident_char(src[i - 1])) and (
            m := _RAW_OPEN.match(src, i)
        ):
            hashes = m.group(1)
            start = m.end()
            end = src.find('"' + hashes, start)
            end = n if end < 0 else end
            wipe(start, end)
            i = end + 1 + len(hashes)
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            wipe(i + 1, j)
            i = j + 1
        elif c == "'":
            if src.startswith("\\", i + 1):
                j = src.find("'", i + 3)
                j = n if j < 0 else j
                wipe(i + 1, j)
                i = j + 1
            elif i + 2 < n and src[i + 2] == "'":
                wipe(i + 1, i + 2)
                i += 3
            else:
                i += 1  # lifetime or label
        else:
            i += 1
    return "".join(out)


_RKYV = re.compile(r"(?<![A-Za-z0-9_#])" + CRATE + r"(?![A-Za-z0-9_])")
_DERIVE = re.compile(r"#\s*\[\s*derive\s*\(([^)]*)\)", re.S)
_ARCHIVE = re.compile(r"(?<![A-Za-z0-9_])Archive(?![A-Za-z0-9_])")
_ARCHIVE_ATTR = re.compile(r"#\s*!?\s*\[\s*(?:archive|archive_attr)(?![A-Za-z0-9_])")
_ARCHIVED = re.compile(r"(?<![A-Za-z0-9_])Archived\s*<")


def _classify(code: str, pos: int) -> str:
    before = code[max(0, pos - 64):pos]
    after = code[pos + len(CRATE):pos + len(CRATE) + 64]
    if re.search(r"(?<![A-Za-z0-9_])use\s+(?:::\s*)?$", before):
        return "use-rkyv"
    if re.search(r"(?<![A-Za-z0-9_])extern\s+crate\s+$", before):
        return "extern-crate-rkyv"
    if re.search(r"#\s*!?\s*\[\s*$", before):
        return "rkyv-attr"
    if re.match(r"\s*::", after):
        return "rkyv-path"
    return "rkyv-ident"


def check_rust(text: str, rel: str) -> list[str]:
    code = blank(text)

    def line(pos: int) -> int:
        return code.count("\n", 0, pos) + 1

    hits: list[tuple[int, str]] = []
    for m in _RKYV.finditer(code):
        hits.append((line(m.start()), _classify(code, m.start())))
    for m in _DERIVE.finditer(code):
        for a in _ARCHIVE.finditer(m.group(1)):
            hits.append((line(m.start(1) + a.start()), "derive-archive"))
    for m in _ARCHIVE_ATTR.finditer(code):
        hits.append((line(m.start()), "archive-attr"))
    for m in _ARCHIVED.finditer(code):
        hits.append((line(m.start()), "archived-type"))
    return [f"{rel}:{n}: {rule}" for n, rule in sorted(set(hits))]


# --------------------------------------------------------------------------
def check(root: Path) -> tuple[list[str], int, int]:
    manifests = walk(root, lambda p: p.name == "Cargo.toml")
    sources = walk(root, lambda p: p.suffix == ".rs")
    problems: list[str] = []
    for p in manifests:
        problems += check_manifest(p, p.relative_to(root).as_posix())
    for p in sources:
        problems += check_rust(p.read_text(encoding="utf-8"), p.relative_to(root).as_posix())
    return problems, len(manifests), len(sources)


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    root = Path(argv[0]).resolve() if argv else ROOT
    problems, manifests, sources = check(root)
    for p in problems:
        print(p, file=sys.stderr)
    print(
        f"no_direct_rkyv: {manifests} manifests, {sources} .rs files read, "
        f"{len(problems)} problems"
    )
    if manifests == 0 or sources == 0:
        print("no_direct_rkyv: compared nothing (0 manifests or 0 .rs files)", file=sys.stderr)
        return 2
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
