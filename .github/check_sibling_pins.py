#!/usr/bin/env python3
"""workflow が sibling を固定 commit 以外の方法で取っていないことの検査

- `.github/workflows/*.yml` に `repository: ext-sakamoro/...` の checkout があれば fail
  (sibling は .github/actions/siblings を通して、そこに固定した commit から取る)
- workflow の comment 以外の行に `git` / `gh` の clone / fetch や `SIBLING_REPOS` があれば fail
  (変数で組んだ URL の取得は literal の repository 行を持たないので、形で止める)
- そこで `repos:` に挙げた repo ごとに、action に `REV_<repo>` (40 桁の sha) と URL が
  無ければ fail
- action の固定値が 0 件、workflow が 0 件、`repos:` の参照が 0 件でも fail
  (検査の空振りを green にしない)

試験は .github/test_check_sibling_pins.py
"""

import pathlib
import re
import sys

DIRECT = re.compile(r"^\s*repository:\s*['\"]?ext-sakamoro/", re.M)
FETCH = re.compile(r"\b(?:git|gh)\b(?:\s+(?:-[cC]\s+\S+|--[\w-]+(?:=\S+)?|-\w+))*\s+(?:repo\s+)?(?:clone|fetch)\b"
                   r"|\bSIBLING_REPOS\b")
REPOS = re.compile(r"^\s*repos:\s*(.+?)\s*$", re.M)
REV = re.compile(r"^\s*REV_(ALICE_[A-Za-z_]+):\s*([0-9a-f]+)\s*$", re.M)
URL = re.compile(r"(ALICE-[A-Za-z-]+)\)\s*echo\s*\"https://github\.com/ext-sakamoro/\1\.git\"")


def check(action_text, workflows):
    """workflows = {path: text} 違反の list と、数えた件数を返す"""
    errors = []
    revs = dict(REV.findall(action_text))
    urls = set(URL.findall(action_text))
    for name, sha in revs.items():
        if len(sha) != 40:
            errors.append(f"action: REV_{name} is not a full 40-character sha")
    referenced = 0
    for path, text in workflows.items():
        for m in DIRECT.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            errors.append(f"{path}:{line}: checks out a sibling directly (use .github/actions/siblings)")
        for no, line in enumerate(text.splitlines(), 1):
            if not line.lstrip().startswith("#") and FETCH.search(line):
                errors.append(f"{path}:{no}: fetches a repository outside .github/actions/siblings")
        for m in REPOS.finditer(text):
            for repo in m.group(1).split():
                referenced += 1
                if repo.replace("-", "_") not in revs:
                    errors.append(f"{path}: {repo} has no REV_{repo.replace('-', '_')} in the action")
                if repo not in urls:
                    errors.append(f"{path}: {repo} has no URL in the action")
    if not revs:
        errors.append("action: no pinned commits found")
    if not workflows:
        errors.append("no workflows found")
    if referenced == 0:
        errors.append("no `repos:` references found in the workflows")
    return errors, len(revs), len(workflows), referenced


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    action = (root / ".github/actions/siblings/action.yml").read_text(encoding="utf-8")
    workflows = {str(p.relative_to(root)): p.read_text(encoding="utf-8")
                 for p in sorted((root / ".github/workflows").glob("*.yml"))}
    errors, n_revs, n_wf, n_ref = check(action, workflows)
    for e in errors:
        print(f"FAIL: {e}")
    print(f"checked pins {n_revs} / workflows {n_wf} / repos references {n_ref}, violations {len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
