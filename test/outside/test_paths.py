"""A script or a workflow of this repository names only paths that stand."""

import importlib.util
import subprocess
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location("paths", Path(__file__).resolve().parents[2] / "script" / "paths.py")
assert SPEC and SPEC.loader
paths = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(paths)

FILES = {"bind/typescript/test/programs/claude.ts", "docs/models.md", "script/real.py", "src/lib.rs", "README.md"}


def test_a_path_that_names_a_file_or_a_folder_of_git_stands() -> None:
  texts = {"script/a.sh": "cp docs/models.md out\nls bind/typescript/test/programs/\nbun build bind/typescript"}
  assert paths.stale(FILES, texts) == []


def test_a_path_that_a_move_left_behind_is_named_with_its_file_and_its_line() -> None:
  texts = {"script/prove-npm.sh": 'set -e\nbun build --compile "$root/bind/typescript/test/claude.ts" --outfile x\n'}
  assert paths.stale(FILES, texts) == [("script/prove-npm.sh", 2, "bind/typescript/test/claude.ts")]


def test_a_path_in_a_string_of_a_script_or_in_a_workflow_is_checked_as_well() -> None:
  texts = {
    "script/real.py": 'x = ROOT / "script/gone.py"  # see docs/old.md.',
    ".github/workflows/gates.yml": "      - run: python3 script/scope.py",
  }
  assert paths.stale(FILES, texts) == [
    (".github/workflows/gates.yml", 1, "script/scope.py"),
    ("script/real.py", 1, "script/gone.py"),
    ("script/real.py", 1, "docs/old.md"),
  ]


def test_a_path_with_a_variable_or_a_pattern_in_it_is_no_whole_path_and_is_not_checked() -> None:
  texts = {"script/a.sh": "cp bind/typescript/furb.$system.node x\nls docs/*.md\nrm bind/typescript/furb.${{ x }}.node"}
  assert paths.stale(FILES, texts) == []


def test_a_word_that_is_not_a_path_of_the_repository_is_not_checked() -> None:
  texts = {
    "script/a.sh": "cd /usr/share/docs/x\ncurl https://example.com/script/y.sh\nls traces/overnight/a.json\nmy-docs/x"
  }
  assert paths.stale(FILES, texts) == []


def test_a_path_that_a_build_makes_and_git_ignores_stands() -> None:
  texts = {"script/a.sh": "ls bind/typescript/dist\ncp docs/gone.md x"}
  made = paths.stale(FILES, texts, lambda found: {x for x in found if x.endswith("dist")})
  assert made == [("script/a.sh", 2, "docs/gone.md")]


def test_a_path_that_a_script_makes_outside_the_repository_stands_when_it_is_listed() -> None:
  assert paths.stale(FILES, {"script/b.sh": "cp demo.rs src/main.rs"}) == []
  assert paths.stale(FILES, {"script/b.sh": "cp demo.rs src/other.rs"}) == [("script/b.sh", 1, "src/other.rs")]


def test_a_folder_that_a_rule_of_git_ignores_as_a_folder_stands_before_a_build_makes_it(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)  # noqa: S607
  (tmp_path / ".gitignore").write_text("dist/\n")
  monkeypatch.setattr(paths, "ROOT", tmp_path)
  assert paths.ignored(["bind/dist", "bind/src"]) == {"bind/dist"}
