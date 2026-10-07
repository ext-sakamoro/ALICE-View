#!/usr/bin/env python3
"""CI 用の sibling stub を `Cargo.toml` から導出して生成する。

## なぜ導出するのか

stub の version を workflow に literal で書くと **source of truth が 2 つ**になる。
`Cargo.toml` の path dep に `version = "4.0"` を足した瞬間、literal `1.7.4` の stub
が要求を外して **CI 全 job が resolve 段階で落ちる** (2026-09-29 に本 repo で実測)。
しかも local の `preflight.sh` は実 sibling を見るので、この経路を構造的に検出できない。

導出すれば drift が原理的に起きない。version も features も `Cargo.toml` が唯一の出所。

## どの dep を stub にしてよいか (これも導出する)

**判定軸は「その dep が既定 build で compile されるか」**であって、private かどうかでも
暗号かどうかでもない。compile される dep に空 lib を置くと `E0432 unresolved import`
で落ちるか、もっと悪い場合は「中身の無い test double」で CI が緑になる
(`alice-blockchain` を使う repo は `signature::{KeyPair, ...}` を無条件に使い、test が
実 Ed25519 の検証に依存するので、そこに stub を置くと暗号的に無意味な緑になる)。

この判定を手で書くと必ず腐るので、機械で出す:

- **非 optional** → 常に compile される → stub 不可
- **optional だが `default` feature から到達できる** → 既定 build で compile される → stub 不可
  (本 repo の `default = ["lol"]` → `lol = ["dep:alice-lol"]` がこれ。`optional = true`
   だけを見て stub にすると `alice_lol::` の参照が全部 E0432 になる)
- **それ以外** → 既定 build では manifest が読めれば足りるので空 lib で正しい

stub 不可の dep が実体として存在しない (= workflow に real checkout が無い) 場合は
**loud fail** する。黙って stub を置くのが一番危ない。

## 探索範囲

real checkout された sibling の `Cargo.toml` も辿る。sibling 自身が外部 path dep を
持つことがあり (ALICE-SDF が `alice-codec` / `libasp` / `alice-cache` を path dep に
戻した場合がこれ)、自 repo の manifest だけ見ていると取りこぼす。

## version の導出規則

要求 range の**下限**を出す。`^4.0`→`4.0.0` / `^0.1.1`→`0.1.1` / `~1.1`→`1.1.0` /
`=1.2.3`→`1.2.3` / prerelease はその文字列 / version 無しは `0.1.0`。
range (`>=1, <2`) と wildcard (`*`) は下限が一意でないので **loud fail** する
(黙って `0.1.0` を置くと、また version 不整合を silent に作るため)。
"""
from __future__ import annotations

import os
import pathlib
import re
import sys

INLINE_DEP = re.compile(r'^\s*([A-Za-z0-9_-]+)\s*=\s*\{(.+)\}\s*$')
SECTION = re.compile(r"^\s*\[([^\]]+)\]\s*$")
KV = re.compile(r'^\s*([A-Za-z0-9_-]+)\s*=\s*(.+?)\s*$')
FIELD = {k: re.compile(rf'\b{k}\s*=\s*"([^"]+)"') for k in ("version", "path")}
OPTIONAL_RE = re.compile(r"\boptional\s*=\s*true\b")
FEATURES_RE = re.compile(r"\bfeatures\s*=\s*\[([^\]]*)\]")
SIMPLE_REQ = re.compile(r"^(\^|~|=)?(\d+)(?:\.(\d+))?(?:\.(\d+))?$")
DEP_SECTION = re.compile(r"(?:^|\.)(?:dependencies|dev-dependencies|build-dependencies)$")


def lower_bound(req: str | None, who: str) -> str:
    """version 要求の下限を semver 3 桁で返す"""
    if req is None:
        return "0.1.0"
    req = req.strip()
    if re.match(r"^\d+\.\d+\.\d+-", req):  # prerelease はそのまま
        return req
    m = SIMPLE_REQ.match(req)
    if not m:
        sys.exit(
            f"make-stubs: {who} の version 要求 '{req}' の下限を一意に決められない\n"
            f"  range / wildcard は stub 化できない 対象の dep を実 clone にするか、\n"
            f"  要求を caret / tilde / exact に直すこと (黙って 0.1.0 を置くと\n"
            f"  version 不整合を silent に作り、CI が resolve 段階で落ちる)")
    return f"{m.group(2)}.{m.group(3) or '0'}.{m.group(4) or '0'}"


def satisfies(version: str, req: str) -> bool:
    """`version` が要求 `req` を満たすか (caret / tilde / exact のみ)"""
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


def choose_version(reqs: list[str | None], crate: str) -> str:
    """全要求を同時に満たす version を決める

    ⚠️ 最初に見つけた要求の下限を採ってはいけない 同じ crate を複数の manifest が
    違う要求で参照する (本 repo の `alice-physics` は自 repo が `1.4`、sibling の
    ALICE-LOL が `1.0`) 先勝ちで `1.0.0` を置くと `^1.4` を外して resolve が落ちる
    **最大下限**を採れば caret の上限が同じ限り両方を満たす
    """
    uniq = [r for r in dict.fromkeys(reqs)]
    cands = sorted(
        (lower_bound(r, crate) for r in uniq),
        key=lambda v: [int(x) for x in re.split(r"[.\-+]", v)[:3] if x.isdigit()])
    best = cands[-1]
    unmet = [r for r in uniq if r is not None and not satisfies(best, r)]
    if unmet:
        sys.exit(
            f"make-stubs: {crate} の要求 {uniq} を同時に満たす version が無い\n"
            f"  候補 {best} は {unmet} を満たさない\n"
            f"  参照側の version 要求を揃えるか、この dep を実 clone にすること")
    return best


def parse_manifest(toml: pathlib.Path) -> tuple[dict[str, dict], dict[str, list[str]]]:
    """(dep 名 -> 属性, feature 名 -> 有効化する項目) を返す

    inline table (`foo = { path = ".." }`) と section 形式
    (`[dependencies.foo]` / `[target.'cfg(..)'.dependencies.foo]`) の両方を読む。
    """
    deps: dict[str, dict] = {}
    feats: dict[str, list[str]] = {}
    section = ""
    sec_dep: str | None = None
    for raw in toml.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.split("#", 1)[0] if raw.lstrip().startswith("#") else raw
        if not line.strip():
            continue
        s = SECTION.match(line)
        if s:
            section = s.group(1)
            sec_dep = None
            # [dependencies.foo] / [target.'cfg(..)'.dev-dependencies.foo]
            head, _, tail = section.rpartition(".")
            if head and DEP_SECTION.search(head):
                sec_dep = tail.strip('"').strip("'")
                deps.setdefault(sec_dep, {"raw": "", "manifest": toml})
            continue
        if sec_dep is not None:
            deps[sec_dep]["raw"] += line + "\n"
            continue
        if section == "features":
            kv = KV.match(line)
            if kv and "[" in kv.group(2):
                items = re.findall(r'"([^"]+)"', kv.group(2))
                feats[kv.group(1)] = items
            continue
        if DEP_SECTION.search(section):
            m = INLINE_DEP.match(line)
            if m:
                deps[m.group(1)] = {"raw": m.group(2), "manifest": toml}
    return deps, feats


def default_enabled(deps: dict[str, dict], feats: dict[str, list[str]]) -> set[str]:
    """既定 feature set で compile される dep 名"""
    enabled = {n for n, d in deps.items() if not OPTIONAL_RE.search(d["raw"])}
    seen, stack = set(), list(feats.get("default", []))
    while stack:
        item = stack.pop()
        if item in seen:
            continue
        seen.add(item)
        if item.startswith("dep:"):
            enabled.add(item[4:])
        elif "/" in item:
            # `foo/bar` は foo を有効化する `foo?/bar` (weak) はしない
            head, _, _ = item.partition("/")
            if not head.endswith("?"):
                enabled.add(head)
        else:
            if item in deps:
                enabled.add(item)
            stack.extend(feats.get(item, []))
    return enabled


WORKDIR: pathlib.Path | None = None  # 各 repo が checkout される親 dir


def _checkout_root(p: pathlib.Path) -> pathlib.Path:
    """`p` を含む checkout dir (= WORKDIR の直下) を返す"""
    cur = p.resolve()
    while cur.parent != WORKDIR and cur.parent != cur:
        cur = cur.parent
    return cur


def collect(repo: pathlib.Path, wanted: dict[str, dict], scanned: set[pathlib.Path],
            own: bool = True) -> None:
    """repo の外を指す path dep を集める (real checkout された sibling は再帰)

    ⚠️ 走査範囲は cargo が実際に実体を要求する範囲に合わせる 2026-09-29 に本 repo で
    `Cargo.lock` を消して 1 つずつ外して実測した結果:

    - 自 repo の**直接** optional path dep (`alice-analytics` / `alice-db` /
      `alice-physics`) → **実体が無いと `failed to get <name> as a dependency` で落ちる**
    - 推移先の path dep が持つ optional path dep (ALICE-LOL の `alice-llm`) →
      **無くても resolve も build も通る**

    なので sibling 側は「その package の manifest の非 optional path dep」だけ辿る。
    全 manifest を rglob すると、graph に入らない workspace member の要求まで拾い
    (ALICE-LOL の他 member が `alice-physics = "1.0"`)、version 調停を無意味に
    失敗させる。sibling の optional dep が実際に活性化されている場合は cargo が
    同じ error で落ちるので、黙って壊れることはない。
    """
    repo = repo.resolve()
    if repo in scanned:
        return
    scanned.add(repo)
    # ⚠️ 「repo の外」の境界は package dir でなく **checkout dir** で測る
    # sibling が workspace の場合 `../alice-lol-macro` は package dir の外だが
    # 同じ checkout の中なので供給不要 (package dir で測ると別 repo 扱いになる)
    boundary = repo if own else _checkout_root(repo)
    manifests = sorted(repo.rglob("Cargo.toml")) if own else [repo / "Cargo.toml"]
    for toml in manifests:
        if "/target/" in toml.as_posix() or not toml.exists():
            continue
        deps, feats = parse_manifest(toml)
        default_set = default_enabled(deps, feats)
        for name, d in deps.items():
            path = FIELD["path"].search(d["raw"])
            if not path:
                continue
            if not own and OPTIONAL_RE.search(d["raw"]):
                continue  # 推移先の optional dep は実体不要 (docstring の実測)
            dest = (toml.parent / path.group(1)).resolve()
            try:
                dest.relative_to(boundary)
                continue  # 同じ checkout の中なので供給不要
            except ValueError:
                pass
            if (dest / "Cargo.toml").exists():
                # real checkout 済 その package 自身の非 optional path dep も要る
                collect(dest, wanted, scanned, own=False)
                continue
            ver = FIELD["version"].search(d["raw"])
            fs = FEATURES_RE.search(d["raw"])
            entry = wanted.setdefault(name, {
                "dest": dest,
                "reqs": [],
                "features": set(),
                "must_compile": [],
            })
            entry["reqs"].append(ver.group(1) if ver else None)
            if fs:
                entry["features"].update(
                    f.strip().strip('"') for f in fs.group(1).split(",") if f.strip())
            if name in default_set:
                entry["must_compile"].append(toml)


def main() -> int:
    global WORKDIR
    # ⚠️ stub は repo の **外** (checkout の親 dir) に書く local で誤実行すると
    # `$HOME/ALICE-Analytics` のような実 repo が無い場所に空 crate を作ってしまうので、
    # CI 以外では明示の opt-in を要求する (local は実 sibling を見る preflight が担当)
    if not os.environ.get("CI") and "--force" not in sys.argv:
        sys.exit("make-stubs: CI 外では --force が要る (repo の外に dir を作るため)")
    root = pathlib.Path(__file__).resolve().parents[1]
    WORKDIR = root.parent
    wanted: dict[str, dict] = {}
    collect(root, wanted, set())
    if not wanted:
        print("make-stubs: 供給が要る外部 path dep なし")
        return 0

    blocked = {n: i for n, i in wanted.items() if i["must_compile"]}
    if blocked:
        msg = ["make-stubs: 既定 build で compile される dep に stub は置けない"]
        for n, i in sorted(blocked.items()):
            where = ", ".join(str(m) for m in i["must_compile"])
            msg.append(f"  - {n}: 非 optional か default feature から到達 ({where})")
        msg.append("  workflow に real checkout (actions/checkout / git clone) を足すこと")
        msg.append("  空 lib を置くと E0432 で落ちるか、中身の無い test double で緑になる")
        sys.exit("\n".join(msg))

    for crate, info in sorted(wanted.items()):
        version = choose_version(info["reqs"], crate)
        # ⚠️ features も `Cargo.toml` から導出する 手で列挙すると、消費側が新しい
        # feature を要求した時に stub が追従せず `feature X does not exist` で落ちる
        feats = sorted(info["features"])
        dest = info["dest"]
        (dest / "src").mkdir(parents=True, exist_ok=True)
        lines = [
            "[package]",
            f'name = "{crate}"',
            f'version = "{version}"',
            'edition = "2021"',
            # cargo-deny の unlicensed reject 予防
            'license = "MIT OR Apache-2.0"',
            "",
            "[lib]",
            'path = "src/lib.rs"',
            "",
            "[features]",
            "default = []",
        ]
        lines += [f"{f} = []" for f in feats if f != "default"]
        (dest / "Cargo.toml").write_text("\n".join(lines) + "\n", encoding="utf-8")
        (dest / "src/lib.rs").write_text("", encoding="utf-8")
        req = ", ".join(r or "(指定なし)" for r in dict.fromkeys(info["reqs"]))
        print(f"  stub {crate} {version}  (要求 {req}, features {feats}) -> {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
