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


def test_a_change_to_the_tui_alone_runs_the_bindings_on_each_system_and_the_suite_on_linux() -> None:
  got = scope.scope(["tui/src/app.ts", "tui/test/cancel.test.ts"])
  assert got == {"suite_os": LINUX, "ts_os": EVERY, "crate": False, "gui": False, "code": True}


def test_a_change_to_the_app_runs_its_checks_on_macos() -> None:
  got = scope.scope(["gui/src/main.ts"])
  assert got == {"suite_os": LINUX, "ts_os": EVERY, "crate": False, "gui": True, "code": True}


def test_a_change_to_the_tests_of_python_alone_runs_the_suite_on_each_system_and_the_bindings_on_linux() -> None:
  got = scope.scope(["test/test_act.py", "pyproject.toml"])
  assert got == {"suite_os": EVERY, "ts_os": LINUX, "crate": False, "gui": False, "code": True}


def test_a_change_to_the_crate_or_to_the_engine_runs_every_job() -> None:
  every = {"suite_os": EVERY, "ts_os": EVERY, "crate": True, "gui": True, "code": True}
  assert scope.scope(["src/furb/engine.py"]) == every
  assert scope.scope(["extensions/web/web.py"]) == every
  assert scope.scope(["Cargo.lock"]) == every


def test_a_change_to_an_extension_of_the_project_runs_the_suite_on_linux() -> None:
  got = scope.scope([".furb/extensions/conductor/conductor.py", ".furb/config.json"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate": False, "gui": False, "code": True}


def test_a_change_to_prose_or_to_the_tools_runs_the_jobs_on_linux_alone() -> None:
  got = scope.scope(["AGENTS.md", "docs/tui.md", "script/budgets.py", ".furb/tests/x.json", "LICENSE"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate": False, "gui": False, "code": False}


def test_a_change_to_the_gates_or_to_a_path_that_no_rule_names_runs_every_job() -> None:
  every = {"suite_os": EVERY, "ts_os": EVERY, "crate": True, "gui": True, "code": True}
  assert scope.scope([".github/workflows/gates.yml"]) == every
  assert scope.scope([".github/actions/setup/action.yml"]) == every
  assert scope.scope(["something-new.cfg"]) == every


def test_the_workflows_with_their_own_proof_run_no_job_of_the_gates() -> None:
  got = scope.scope([".github/workflows/publish.yml", ".github/workflows/titles.yml", ".github/budgets.json"])
  assert got == {"suite_os": LINUX, "ts_os": LINUX, "crate": False, "gui": False, "code": False}


def test_a_change_with_no_path_runs_the_jobs_on_linux_alone() -> None:
  assert scope.scope([]) == {"suite_os": LINUX, "ts_os": LINUX, "crate": False, "gui": False, "code": False}


def test_the_answer_is_the_lines_of_the_outputs_of_a_step() -> None:
  lines = scope.lines(scope.scope(["tui/src/app.ts"])).splitlines()
  assert [line.split("=", 1)[0] for line in lines] == ["suite_os", "ts_os", "crate", "gui", "code"]
  assert json.loads(lines[0].split("=", 1)[1]) == LINUX
  assert lines[2] == "crate=false"
