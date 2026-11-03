"""A comment line that a change adds to a Rust file goes no further than 100 characters."""

import importlib.util
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
  "rustwidth", Path(__file__).resolve().parents[2] / "script" / "rustwidth.py"
)
assert SPEC and SPEC.loader
rustwidth = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(rustwidth)

LINE = "// " + "x" * 97
LONG = "// " + "x" * 98


def test_a_comment_line_of_100_characters_passes() -> None:
  assert len(LINE) == 100
  assert rustwidth.failures({"src/a.rs": f"fn a() {{}}\n{LINE}\n"}, {"src/a.rs": {2}}) == []


def test_an_added_comment_line_of_101_characters_fails_with_its_file_and_its_line() -> None:
  assert len(LONG) == 101
  assert rustwidth.failures({"src/a.rs": f"fn a() {{}}\n  {LONG}\n"}, {"src/a.rs": {2}}) == [
    "src/a.rs:2: a comment line that this change adds goes past 100 characters"
  ]


def test_a_long_comment_line_that_the_change_does_not_add_passes() -> None:
  assert rustwidth.failures({"src/a.rs": f"{LONG}\n"}, {"src/a.rs": {5}}) == []


def test_a_doc_comment_is_a_comment_and_a_line_of_code_is_not() -> None:
  text = f'/// {LONG}\n//! {LONG}\nconst A: &str = "{LONG}";\n'
  found = rustwidth.failures({"src/a.rs": text}, {"src/a.rs": {1, 2, 3}})
  assert [x[:10] for x in found] == ["src/a.rs:1", "src/a.rs:2"]


def test_a_line_inside_a_string_or_a_block_comment_is_no_comment() -> None:
  text = f'const A: &str = "\n{LONG}\n";\nconst B: &str = r#"\n{LONG}\n"#;\nconst C: &str = "a \\\n{LONG}";\n/*\n{LONG}\n*/\n'
  assert rustwidth.failures({"src/a.rs": text}, {"src/a.rs": set(range(1, 12))}) == []


def test_a_diff_gives_the_lines_that_it_adds() -> None:
  diff = "+++ b/src/a.rs\n@@ -3,2 +3,3 @@ x\n+a\n@@ -9 +10 @@\n+++ /dev/null\n@@ -1 +0,0 @@\n"
  assert rustwidth.parse(diff) == {"src/a.rs": {3, 4, 5, 10}}
