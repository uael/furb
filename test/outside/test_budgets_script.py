"""The budgets are set for the jobs that the gates name, and for no other."""

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("budgets", ROOT / "script" / "budgets.py")
assert SPEC and SPEC.loader
budgets = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(budgets)


def kept(name: str) -> bool:
  return any(x.fullmatch(name) for x in budgets.named(str(ROOT / ".github" / "workflows" / "gates.yml")))


def test_a_job_of_a_matrix_has_a_budget_for_each_system() -> None:
  assert kept("the suite on macos-latest") and kept("the bindings and the TUI on windows-latest")


def test_a_job_that_no_workflow_names_has_no_budget() -> None:
  assert not kept("the app on macos-latest")


def test_the_job_that_only_holds_the_check_has_no_budget() -> None:
  assert not kept("the gates")
