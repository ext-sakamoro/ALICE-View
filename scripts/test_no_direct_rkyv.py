#!/usr/bin/env python3
"""Tests for scripts/no_direct_rkyv.py: the repository passes, a direct rkyv
dependency in every table form and a use of rkyv in Rust code are reported by
path:line and rule, comments and strings are not findings, and a tree with no
manifest or no .rs file compares nothing and fails."""
import contextlib
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import no_direct_rkyv as g  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

MANIFEST = """[package]
name = "demo"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
"""
LIB = "pub fn f() -> u32 { 1 }\n"


def tree(files: dict[str, str]) -> Path:
    root = Path(tempfile.mkdtemp(prefix="no-direct-rkyv-"))
    for rel, text in files.items():
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8")
    return root


def clean(extra: dict[str, str] | None = None) -> dict[str, str]:
    files = {"Cargo.toml": MANIFEST, "src/lib.rs": LIB}
    files.update(extra or {})
    return files


def run(root: Path) -> tuple[int, str, str]:
    with contextlib.redirect_stdout(io.StringIO()) as out, contextlib.redirect_stderr(
        io.StringIO()
    ) as err:
        code = g.main([str(root)])
    return code, out.getvalue(), err.getvalue()


class Repository(unittest.TestCase):
    def test_repository_is_clean_and_compares_something(self):
        problems, manifests, sources = g.check(ROOT)
        self.assertEqual(problems, [])
        self.assertGreater(manifests, 0)
        self.assertGreater(sources, 0)


class Manifests(unittest.TestCase):
    def manifest_problems(self, manifest: str) -> list[str]:
        problems, _, _ = g.check(tree({"Cargo.toml": manifest, "src/lib.rs": LIB}))
        return problems

    def assert_one(self, extra: str, line: int, kind: str):
        problems = self.manifest_problems(MANIFEST + extra)
        self.assertEqual(problems, [f"Cargo.toml:{line}: rkyv-dependency ({kind})"])

    def test_clean_tree_passes(self):
        root = tree(clean())
        self.assertEqual(run(root)[0], 0)

    def test_plain_dependency(self):
        self.assert_one('rkyv = "0.7"\n', 8, "dependencies")

    def test_inline_table_dependency(self):
        self.assert_one('rkyv = { version = "0.7", optional = true }\n', 8, "dependencies")

    def test_dotted_key_dependency(self):
        self.assert_one('rkyv.version = "0.7"\nrkyv.optional = true\n', 8, "dependencies")

    def test_dependency_table_header(self):
        self.assert_one('\n[dependencies.rkyv]\nversion = "0.7"\n', 9, "dependencies")

    def test_dev_dependency(self):
        self.assert_one('\n[dev-dependencies]\nrkyv = "0.7"\n', 10, "dev-dependencies")

    def test_build_dependency(self):
        self.assert_one('\n[build-dependencies]\nrkyv = "0.7"\n', 10, "build-dependencies")

    def test_target_specific_dependency(self):
        self.assert_one(
            "\n[target.'cfg(unix)'.dependencies]\nrkyv = \"0.7\"\n",
            10,
            "target-dependencies",
        )

    def test_target_specific_dev_dependency_header(self):
        self.assert_one(
            '\n[target."cfg(windows)".dev-dependencies.rkyv]\nversion = "0.7"\n',
            9,
            "target-dev-dependencies",
        )

    def test_renamed_inline(self):
        self.assert_one(
            'archive = { package = "rkyv", version = "0.7" }\n', 8, "dependencies renamed"
        )

    def test_renamed_table(self):
        self.assert_one(
            '\n[dependencies.archive]\npackage = "rkyv"\nversion = "0.7"\n',
            10,
            "dependencies renamed",
        )

    def test_renamed_dev_dependency(self):
        self.assert_one(
            "\n[dev-dependencies]\nzc = { version = \"0.7\", package = 'rkyv' }\n",
            10,
            "dev-dependencies renamed",
        )

    def test_workspace_dependency(self):
        self.assert_one(
            '\n[workspace.dependencies]\nrkyv = "0.7"\n', 10, "workspace-dependencies"
        )

    def test_workspace_inherited_dependency(self):
        self.assert_one("rkyv.workspace = true\n", 8, "dependencies")

    def test_patch(self):
        self.assert_one(
            '\n[patch.crates-io]\nrkyv = { git = "https://example.invalid/r" }\n',
            10,
            "patch",
        )

    def test_member_manifest_is_read(self):
        files = clean()
        files["members/inner/Cargo.toml"] = MANIFEST + 'rkyv = "0.7"\n'
        problems, manifests, _ = g.check(tree(files))
        self.assertEqual(manifests, 2)
        self.assertEqual(problems, ["members/inner/Cargo.toml:8: rkyv-dependency (dependencies)"])

    def test_comment_and_other_names_do_not_trigger(self):
        extra = (
            '# rkyv = "0.7" was removed\n'
            'rkyv_dyn_helper = "1"\n'
            'my-rkyv = "1"\n'
            'description_note = "not rkyv"\n'
            "\n[features]\n"
            'archive = []\n'
        )
        self.assertEqual(self.manifest_problems(MANIFEST + extra), [])

    def test_multiline_string_mentioning_rkyv_does_not_trigger(self):
        m = MANIFEST.replace(
            'edition = "2021"',
            'edition = "2021"\ndescription = """\nrkyv = "0.7"\n"""',
        )
        self.assertEqual(self.manifest_problems(m), [])

    def test_unparsable_manifest_fails(self):
        self.assertEqual(
            self.manifest_problems("[package\n"), ["Cargo.toml:0: unparsable-manifest"]
        )


class Rust(unittest.TestCase):
    def rs(self, text: str, rel: str = "src/lib.rs") -> list[str]:
        return g.check_rust(text, rel)

    def test_use_in_src_fails(self):
        root = tree(clean({"src/lib.rs": "use rkyv::Archive;\n" + LIB}))
        code, out, err = run(root)
        self.assertEqual(code, 1)
        self.assertIn("src/lib.rs:1: use-rkyv", err)
        self.assertNotIn("Archive", err)  # no matched text beyond path:line: rule
        self.assertIn("1 problems", out)

    def test_derive_in_tests_fails(self):
        text = (
            "#[derive(\n"
            "    Debug,\n"
            "    rkyv::Archive,\n"
            ")]\n"
            "struct S { x: u32 }\n"
        )
        problems, _, _ = g.check(tree(clean({"tests/t.rs": text})))
        self.assertEqual(
            problems, ["tests/t.rs:3: derive-archive", "tests/t.rs:3: rkyv-path"]
        )

    def test_bare_archive_derive(self):
        self.assertEqual(
            self.rs("#[derive(Debug, Archive)]\nstruct S;\n"), ["src/lib.rs:1: derive-archive"]
        )

    def test_rules(self):
        self.assertEqual(self.rs("use ::rkyv as r;\n"), ["src/lib.rs:1: use-rkyv"])
        self.assertEqual(self.rs("extern crate rkyv;\n"), ["src/lib.rs:1: extern-crate-rkyv"])
        self.assertEqual(
            self.rs("fn f(b: &[u8]) { let _ = rkyv::check_archived_root::<u8>(b); }\n"),
            ["src/lib.rs:1: rkyv-path"],
        )
        self.assertEqual(self.rs("#[rkyv(compare(PartialEq))]\nstruct S;\n"), ["src/lib.rs:1: rkyv-attr"])
        self.assertEqual(self.rs("pub use other::rkyv;\n"), ["src/lib.rs:1: rkyv-ident"])
        self.assertEqual(
            self.rs("#[archive(check_bytes)]\n#[archive_attr(derive(Debug))]\nstruct S;\n"),
            ["src/lib.rs:1: archive-attr", "src/lib.rs:2: archive-attr"],
        )
        self.assertEqual(
            self.rs("fn f(x: &Archived<u32>) {}\n"), ["src/lib.rs:1: archived-type"]
        )

    def test_raw_identifier(self):
        self.assertEqual(self.rs("use r#rkyv::Archive;\n"), ["src/lib.rs:1: use-rkyv"])
        self.assertEqual(self.rs("use ::r#rkyv as r;\n"), ["src/lib.rs:1: use-rkyv"])
        self.assertEqual(self.rs("extern crate r#rkyv;\n"), ["src/lib.rs:1: extern-crate-rkyv"])
        self.assertEqual(
            self.rs("fn f(b: &[u8]) { let _ = r#rkyv::from_bytes::<u8>(b); }\n"),
            ["src/lib.rs:1: rkyv-path"],
        )
        self.assertEqual(self.rs("#[r#rkyv(derive(Debug))]\nstruct S;\n"), ["src/lib.rs:1: rkyv-attr"])
        self.assertEqual(self.rs("pub use other::r#rkyv;\n"), ["src/lib.rs:1: rkyv-ident"])

    def test_raw_identifier_attribute_names(self):
        self.assertEqual(
            self.rs("#[r#derive(Archive)]\nstruct S;\n"), ["src/lib.rs:1: derive-archive"]
        )
        self.assertEqual(
            self.rs("#[r#archive(check_bytes)]\nstruct S;\n"), ["src/lib.rs:1: archive-attr"]
        )

    def test_spaced_path_and_group_import(self):
        self.assertEqual(self.rs("fn f() { rkyv :: to_bytes(); }\n"), ["src/lib.rs:1: rkyv-path"])
        self.assertEqual(self.rs("use {rkyv::Archive};\n"), ["src/lib.rs:1: rkyv-path"])
        self.assertEqual(self.rs("use ::{rkyv::Archive};\n"), ["src/lib.rs:1: rkyv-path"])

    def test_cfg_attr_derive(self):
        self.assertEqual(
            self.rs('#[cfg_attr(feature = "x", derive(Archive))]\nstruct S;\n'),
            ["src/lib.rs:1: derive-archive"],
        )

    def test_cfg_attr_derive_multi_line(self):
        text = (
            "#[cfg_attr(\n"
            '    feature = "x",\n'
            "    derive(\n"
            "        Debug,\n"
            "        Archive,\n"
            "    )\n"
            ")]\n"
            "struct S;\n"
        )
        self.assertEqual(self.rs(text), ["src/lib.rs:5: derive-archive"])

    def test_cfg_attr_nested(self):
        self.assertEqual(
            self.rs('#[cfg_attr(unix, cfg_attr(feature = "x", derive(Debug, Archive)))]\nstruct S;\n'),
            ["src/lib.rs:1: derive-archive"],
        )
        self.assertEqual(
            self.rs("#[r#cfg_attr(a, derive(Debug), r#derive(Archive))]\nstruct S;\n"),
            ["src/lib.rs:1: derive-archive"],
        )

    def test_cfg_attr_rkyv_path_derive(self):
        self.assertEqual(
            self.rs("#[cfg_attr(test, derive(rkyv::Archive))]\nstruct S;\n"),
            ["src/lib.rs:1: derive-archive", "src/lib.rs:1: rkyv-path"],
        )

    def test_cfg_attr_archive_attr(self):
        self.assertEqual(
            self.rs(
                "#[cfg_attr(x, archive(check_bytes))]\n"
                "#[cfg_attr(x, archive_attr(derive(Debug)))]\n"
                "struct S;\n"
            ),
            ["src/lib.rs:1: archive-attr", "src/lib.rs:2: archive-attr"],
        )
        self.assertEqual(
            self.rs("#![cfg_attr(\n    x,\n    archive(check_bytes)\n)]\n"),
            ["src/lib.rs:3: archive-attr"],
        )

    def test_similar_forms_do_not_trigger(self):
        text = (
            "fn f(r#type: u8) -> u8 { r#type }\n"
            "#[cfg_attr(test, derive(Debug))]\n"
            '#[cfg_attr(all(feature = "a", not(test)), derive(Debug, Clone))]\n'
            "#[cfg_attr(archive, derive(Debug))]\n"
            "#[cfg_attr(x, archived_note)]\n"
            "struct S;\n"
            "// #[cfg_attr(feature = \"x\", derive(Archive))]\n"
            "/* #[cfg_attr(x, archive(check_bytes))] */\n"
            'const A: &str = "#[cfg_attr(x, derive(Archive))]";\n'
            'const B: &str = r#"#[cfg_attr(x, archive_attr(derive(Debug)))]"#;\n'
        )
        self.assertEqual(self.rs(text), [])

    def test_benches_and_examples_are_read(self):
        files = clean({"benches/b.rs": "use rkyv;\n", "examples/e.rs": "type A = Archived<u8>;\n"})
        problems, _, sources = g.check(tree(files))
        self.assertEqual(sources, 3)
        self.assertEqual(
            problems, ["benches/b.rs:1: use-rkyv", "examples/e.rs:1: archived-type"]
        )

    def test_comments_and_strings_do_not_trigger(self):
        text = (
            "// use rkyv::Archive;\n"
            "/// rkyv::Archived<T> is not used here\n"
            "/* outer /* nested rkyv:: */ #[derive(Archive)] */\n"
            "const NAME: &str = \"seg_1.rkyv\";\n"
            "const RAW: &str = r#\"use rkyv::Archive; \"quoted\" Archived<u8>\"#;\n"
            "const B: &[u8] = b\"rkyv::\";\n"
            "const C: char = '\"';\n"
            "fn f<'a>(x: &'a str) -> &'a str { x }\n"
            "const D: &str = \"after the char literal: rkyv::x\";\n"
            "struct Archiver; struct ArchivedNote; fn rkyv_free() {}\n"
        )
        self.assertEqual(self.rs(text), [])

    def test_code_after_literals_is_still_scanned(self):
        text = "const C: char = '\\'';\nconst S: &str = \"a\\\"b\";\nuse rkyv;\n"
        self.assertEqual(self.rs(text), ["src/lib.rs:3: use-rkyv"])

    def test_skipped_directories(self):
        files = clean({"target/debug/x.rs": "use rkyv;\n"})
        files[".hidden/y.rs"] = "use rkyv;\n"
        files["target/Cargo.toml"] = '[dependencies]\nrkyv = "0.7"\n'
        problems, manifests, sources = g.check(tree(files))
        self.assertEqual((problems, manifests, sources), ([], 1, 1))


class ComparesNothing(unittest.TestCase):
    def test_empty_tree_fails(self):
        code, out, err = run(tree({"README.md": "x\n"}))
        self.assertEqual(code, 2)
        self.assertIn("0 manifests, 0 .rs files read", out)
        self.assertIn("compared nothing", err)

    def test_manifest_without_rust_fails(self):
        self.assertEqual(run(tree({"Cargo.toml": MANIFEST}))[0], 2)

    def test_rust_without_manifest_fails(self):
        self.assertEqual(run(tree({"src/lib.rs": LIB}))[0], 2)


if __name__ == "__main__":
    unittest.main()
