"""The hooks of the tests run what the files of a commit reach, and the gates, which give every file, run the whole set."""

import importlib.util
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("reach", ROOT / "script" / "hooks" / "reach.py")
assert SPEC and SPEC.loader
reach = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(reach)


def hooks() -> dict[str, dict]:
  loaded = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
  return {one["id"]: one for repo in loaded["repos"] for one in repo["hooks"]}


def test_the_hooks_of_the_tests_give_all_the_files_of_the_commit_to_the_reach_in_one_call() -> None:
  for name, kind in (("pytest", "pytest"), ("cargo-test", "cargo")):
    hook = hooks()[name]
    assert hook["entry"] == f"python3 script/hooks/reach.py {kind}"
    assert hook.get("pass_filenames", True) and hook["require_serial"]


def test_a_file_of_the_engine_of_the_crate_or_of_a_manifest_reaches_the_whole_suite() -> None:
  for path in ("src/furb/engine.py", "src/world/files.rs", "pyproject.toml", "Cargo.lock", "test/conftest.py"):
    assert reach.suite([path]) is None


def test_a_script_reaches_the_tests_that_name_it_and_no_other() -> None:
  assert "test/outside/test_title_script.py" in reach.suite(["script/title.py"])
  assert "test/test_boot.py" not in reach.suite(["script/title.py"])
  assert "test/outside/test_scope.py" in reach.suite(["script/scope.py"])


def test_a_file_of_an_extension_reaches_its_tests_and_the_hygiene_laws() -> None:
  assert reach.suite([".agents/skills/memory/memory.py"]) == [".agents/skills/memory/test", "test/test_hygiene.py"]


def test_a_file_of_a_crate_reaches_that_crate_and_a_manifest_the_workspace() -> None:
  assert reach.crates(["host/src/daemon.rs"]) == ["furb-host"]
  assert reach.crates(["src/engine.rs", "cli/src/main.rs"]) == ["furb", "furb-cli"]
  assert reach.crates(["src/engine.rs", "Cargo.toml"]) is None


def test_each_file_of_the_harness_is_a_file_that_git_holds_and_reaches_the_whole_suite() -> None:
  for path in reach.HARNESS:
    assert (ROOT / path).is_file()
    assert reach.suite([path]) is None


def test_every_file_of_the_tree_reaches_the_whole_set_as_the_gates_give_it() -> None:
  every = ["script/scope.py", "src/furb/engine.py", "Cargo.toml", "host/src/daemon.rs"]
  assert reach.suite(every) is None and reach.crates(every) is None


def test_a_selection_of_tests_runs_with_no_coverage_and_the_whole_suite_keeps_it() -> None:
  got = reach.command("pytest", ["script/title.py"])
  assert got[:5] == ["uv", "run", "pytest", "-q", "--no-cov"]
  assert "test/outside/test_title_script.py" in got[5:]
  assert reach.command("pytest", ["src/furb/engine.py"]) == ["uv", "run", "pytest", "-q"]
  assert reach.command("pytest", ["tui/src/app.ts"]) is None


def test_a_contract_test_an_official_extension_test_and_a_project_contract_reach_the_hygiene_laws() -> None:
  laws = "test/test_hygiene.py"
  assert laws in reach.suite(["test/test_peek.py"])
  assert laws in reach.suite([".agents/skills/memory/test/test_memory.py"])
  assert laws in reach.suite([".furb/skills/pr/pr.py"])
  assert laws in reach.suite([".furb/skills/pr/test/test_commit.py"])
  assert laws not in reach.suite(["test/outside/test_scope.py"])
