"""The paths of a change say which jobs of the gates run."""

import importlib.util
import json
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("scope", Path(__file__).resolve().parents[2] / "script" / "scope.py")
assert SPEC and SPEC.loader
scope = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scope)

LINUX = ["ubuntu-latest"]
EVERY = ["ubuntu-latest", "macos-latest", "windows-latest"]


def test_a_change_to_the_bindings_alone_runs_them_on_each_system_and_the_suite_on_linux() -> None:
  got = scope.scope(["bind/typescript/src/session.ts", "bind/typescript/test/engine.test.ts"])
  assert got == {"suite_os": LINUX, "ts_os": EVERY, "crate_os": LINUX, "build_os": EVERY, "code": True}


def test_a_change_to_the_tests_of_python_alone_runs_the_suite_on_each_system_and_the_bindings_on_linux() -> None:
  got = scope.scope(["test/test_act.py", "pyproject.toml"])
  assert got == {"suite_os": EVERY, "ts_os": LINUX, "crate_os": LINUX, "build_os": EVERY, "code": True}


def test_a_change_to_the_window_alone_runs_the_crate_on_each_system_and_the_other_jobs_on_linux() -> None:
  got = scope.scope(["window/src/model.rs", "window/tests/screens/start.txt"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate_os": EVERY, "build_os": EVERY, "code": True}


def test_a_change_to_the_crate_runs_every_job() -> None:
  every = {"suite_os": EVERY, "ts_os": EVERY, "crate_os": EVERY, "build_os": EVERY, "code": True}
  assert scope.scope(["src/furb/world.py"]) == every
  assert scope.scope([".agents/skills/web/web.py"]) == every
  assert scope.scope(["Cargo.lock"]) == every


def test_a_change_to_the_engine_or_to_the_corpus_alone_runs_every_job_on_linux() -> None:
  linux = {"suite_os": LINUX, "ts_os": LINUX, "crate_os": LINUX, "build_os": LINUX, "code": True}
  assert scope.scope(["src/furb/engine.py", "src/furb/engine.pyi", "src/furb/record.py", "src/furb/kernel.py"]) == linux
  assert scope.scope(["tests/corpus/next/work.jsonl"]) == linux


def test_a_change_to_the_engine_with_the_world_runs_every_job() -> None:
  got = scope.scope(["src/furb/engine.py", "src/world/mod.rs"])
  assert got == {"suite_os": EVERY, "ts_os": EVERY, "crate_os": EVERY, "build_os": EVERY, "code": True}


def test_a_change_to_a_skill_of_the_project_runs_the_suite_on_linux() -> None:
  got = scope.scope([".furb/skills/lane/lane.py", ".furb/config.json"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate_os": LINUX, "build_os": LINUX, "code": True}


def test_a_change_to_prose_or_to_the_tools_runs_the_jobs_on_linux_alone() -> None:
  got = scope.scope(["AGENTS.md", "docs/models.md", "script/budgets.py", ".furb/tests/x.json", "LICENSE"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate_os": LINUX, "build_os": LINUX, "code": False}


def test_a_change_to_the_gates_or_to_a_path_that_no_rule_names_runs_every_job() -> None:
  every = {"suite_os": EVERY, "ts_os": EVERY, "crate_os": EVERY, "build_os": EVERY, "code": True}
  assert scope.scope([".github/workflows/gates.yml"]) == every
  assert scope.scope([".github/actions/setup/action.yml"]) == every
  assert scope.scope(["something-new.cfg"]) == every


def test_the_workflows_with_their_own_proof_run_no_job_of_the_gates() -> None:
  got = scope.scope([".github/workflows/publish.yml", ".github/workflows/titles.yml", ".github/budgets.json"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate_os": LINUX, "build_os": LINUX, "code": False}


def test_a_change_with_no_path_runs_the_jobs_on_linux_alone() -> None:
  assert scope.scope([]) == {"suite_os": LINUX, "ts_os": LINUX, "crate_os": LINUX, "build_os": LINUX, "code": False}


def test_the_answer_is_the_lines_of_the_outputs_of_a_step() -> None:
  lines = scope.lines(scope.scope(["bind/typescript/src/session.ts"])).splitlines()
  assert [line.split("=", 1)[0] for line in lines] == ["suite_os", "ts_os", "crate_os", "build_os", "code"]
  assert json.loads(lines[0].split("=", 1)[1]) == LINUX
  assert json.loads(lines[2].split("=", 1)[1]) == LINUX
