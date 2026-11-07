"""The hooks of the tests run what the files of a commit reach, and the gates, which give every file, run the whole set."""

import importlib.util
import os
import shlex
import subprocess
import sys
from pathlib import Path

import pytest
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


@pytest.mark.skipif(sys.platform == "win32", reason="the shims run in sh")
def test_each_hook_of_the_tests_starts_at_once_while_the_memory_is_tight(tmp_path: Path) -> None:
  # The kernel says level 4, critical, and cargo and uv are fakes that note their call: a hook that waits on memory
  # outlives its time, and a hook that starts calls its tool.
  shim, calls = tmp_path / "shim", tmp_path / "calls"
  shim.mkdir()
  fakes = {
    "sysctl": "echo 4",
    "memory_pressure": "echo 'System-wide memory free percentage: 3%'",
    "cargo": f'echo "cargo $*" >> {calls}',
    "uv": f'echo "uv $*" >> {calls}',
  }
  for name, body in fakes.items():
    (shim / name).write_text(f"#!/bin/sh\n{body}\n", encoding="utf-8")
    (shim / name).chmod(0o755)
  env = {**os.environ, "PATH": f"{shim}{os.pathsep}{os.environ['PATH']}"}
  for name in ("pytest", "cargo-test", "clippy"):
    hook = hooks()[name]
    words = [*shlex.split(hook["entry"]), *(["src/furb/engine.py"] if hook.get("pass_filenames", True) else [])]
    got = subprocess.run(words, cwd=ROOT, env=env, timeout=20, capture_output=True, text=True, check=False)  # noqa: S603
    assert got.returncode == 0, name
  tools = [line.split()[:2] for line in calls.read_text(encoding="utf-8").splitlines()]
  assert tools == [["uv", "run"], ["cargo", "test"], ["cargo", "clippy"]]


def test_only_the_watcher_reads_the_memory() -> None:
  readers = [p.name for p in (ROOT / "script").rglob("*") if p.is_file() and b"memorystatus" in p.read_bytes()]
  assert readers == ["memwatch.sh"]


def test_a_file_of_the_engine_of_the_crate_or_of_a_manifest_reaches_the_whole_suite() -> None:
  for path in ("src/furb/engine.py", "src/world/files.rs", "pyproject.toml", "Cargo.lock", "test/conftest.py"):
    assert reach.suite([path]) is None


def test_a_script_reaches_the_tests_that_name_it_and_no_other() -> None:
  assert "test/outside/test_title_script.py" in reach.suite(["script/title.py"])
  assert "test/test_boot.py" not in reach.suite(["script/title.py"])
  assert "test/outside/test_scope.py" in reach.suite(["script/scope.py"])


def test_a_file_of_a_skill_reaches_its_tests_and_the_hygiene_laws() -> None:
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


def test_a_contract_test_an_official_skill_test_and_a_project_contract_reach_the_hygiene_laws() -> None:
  laws = "test/test_hygiene.py"
  assert laws in reach.suite(["test/test_peek.py"])
  assert laws in reach.suite([".agents/skills/memory/test/test_memory.py"])
  assert laws in reach.suite([".furb/skills/pr/pr.py"])
  assert laws in reach.suite([".furb/skills/pr/test/test_commit.py"])
  assert laws not in reach.suite(["test/outside/test_scope.py"])
