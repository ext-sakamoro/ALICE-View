#!/usr/bin/env python3
""".github/check_sibling_pins.py の試験 (違反を置いた入力で、検出の歯を確かめる)"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import check_sibling_pins as c  # noqa: E402

SHA = "a" * 40
ACTION = f"""
      env:
        REV_ALICE_SDF: {SHA}
        REV_ALICE_LOL: {SHA}
      run: |
            ALICE-SDF) echo "https://github.com/ext-sakamoro/ALICE-SDF.git" ;;
            ALICE-LOL) echo "https://github.com/ext-sakamoro/ALICE-LOL.git" ;;
"""
GOOD = """
      - uses: actions/checkout@v5
        with:
          path: ALICE-View
      - uses: ./ALICE-View/.github/actions/siblings
        with:
          repos: ALICE-SDF ALICE-LOL
"""


class CheckSiblingPins(unittest.TestCase):
    def test_good_passes(self):
        errors, revs, wf, ref = c.check(ACTION, {"ci.yml": GOOD})
        self.assertEqual(errors, [])
        self.assertEqual((revs, wf, ref), (2, 1, 2))

    def test_direct_checkout_fails(self):
        bad = GOOD + "      - uses: actions/checkout@v5\n        with:\n          repository: ext-sakamoro/ALICE-Zip\n"
        errors, *_ = c.check(ACTION, {"ci.yml": bad})
        self.assertEqual(len(errors), 1)
        self.assertIn("checks out a sibling directly", errors[0])

    def test_direct_checkout_with_ref_still_fails(self):
        bad = GOOD + "        with:\n          repository: ext-sakamoro/ALICE-SDF\n          ref: " + SHA + "\n"
        errors, *_ = c.check(ACTION, {"ci.yml": bad})
        self.assertEqual(len(errors), 1)

    def test_variable_clone_loop_fails(self):
        bad = GOOD + ('      - run: |\n'
                      '          for repo in $SIBLING_REPOS; do\n'
                      '            git clone --depth 1 "https://github.com/ext-sakamoro/${repo}.git" "../${repo}" || true\n'
                      '          done\n')
        errors, *_ = c.check(ACTION, {"release.yml": bad})
        self.assertEqual(len(errors), 2)  # the SIBLING_REPOS loop line and the clone line

    def test_fetch_forms_fail_and_comments_pass(self):
        for line in ('git -C ../x fetch --depth 1 origin "$rev"', 'gh repo clone "$r"',
                     'git -c protocol.version=2 clone "$url"', 'env:\n  SIBLING_REPOS: >-'):
            errors, *_ = c.check(ACTION, {"w.yml": GOOD + "      - run: " + line + "\n"})
            self.assertEqual(len(errors), 1, line)
        ok = GOOD + "      # git clone https://github.com/ext-sakamoro/ALICE-SDF.git (comment)\n" \
                    "      - run: git worktree add --detach ../baseline \"$base\"\n" \
                    "      - run: gh release upload \"$TAG\" a.zip --clobber\n"
        self.assertEqual(c.check(ACTION, {"w.yml": ok})[0], [])

    def test_unpinned_repo_fails(self):
        bad = GOOD.replace("repos: ALICE-SDF ALICE-LOL", "repos: ALICE-SDF ALICE-DB")
        errors, *_ = c.check(ACTION, {"ci.yml": bad})
        self.assertEqual(len(errors), 2)  # no REV and no URL for ALICE-DB

    def test_short_sha_fails(self):
        errors, *_ = c.check(ACTION.replace(SHA, "abc1234", 1), {"ci.yml": GOOD})
        self.assertEqual(len(errors), 1)
        self.assertIn("40-character", errors[0])

    def test_nothing_to_check_fails(self):
        self.assertTrue(c.check("", {"ci.yml": GOOD})[0])
        self.assertTrue(c.check(ACTION, {})[0])
        self.assertTrue(c.check(ACTION, {"ci.yml": "jobs: {}\n"})[0])


if __name__ == "__main__":
    unittest.main()
