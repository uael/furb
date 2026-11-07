# ruff: noqa: S603
import os
import subprocess
import sys
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "script" / "budget.py"


def budget(tmp: Path, part: str, limit: int, *command: str) -> subprocess.CompletedProcess[str]:
  """The run of a command as the part pytest, under a home of limits whose pytest limit is the given one."""
  (tmp / "budgets.toml").write_text(f"[local]\npytest = {limit}\n[ci]\npytest = {limit}\n", encoding="utf-8")
  env = {n: v for n, v in os.environ.items() if n != "CI"} | {"FURB_BUDGETS": str(tmp / "budgets.toml")}
  args = [sys.executable, str(SCRIPT), part, "--", *command]
  return subprocess.run(args, env=env, cwd=tmp, capture_output=True, text=True, timeout=120, check=False)


def pytest_of(tmp: Path, body: str) -> list[str]:
  (tmp / "test_part.py").write_text(body, encoding="utf-8")
  (tmp / "pytest.ini").write_text("[pytest]\n", encoding="utf-8")
  args = ["-q", "-c", "pytest.ini", "-p", "no:cacheprovider", "--durations=5", "test_part.py"]
  return [sys.executable, "-m", "pytest", *args]


def test_a_run_that_passes_its_limit_fails_and_names_its_slowest_test(tmp_path: Path) -> None:
  got = budget(
    tmp_path, "pytest", 1, *pytest_of(tmp_path, "import time\n\n\ndef test_sleeps_past_the_bound():\n  time.sleep(2)\n")
  )
  assert got.returncode == 1
  assert "passed its limit" in got.stdout
  assert "test_part.py::test_sleeps_past_the_bound" in got.stdout.split("its slowest tests:")[1]


def test_the_same_run_with_no_slow_test_passes_under_its_limit(tmp_path: Path) -> None:
  got = budget(tmp_path, "pytest", 30, *pytest_of(tmp_path, "def test_quick():\n  pass\n"))
  assert got.returncode == 0
  assert "passed its limit" not in got.stdout and "of 30 s" in got.stdout


def test_a_command_that_fails_gives_its_own_code(tmp_path: Path) -> None:
  assert budget(tmp_path, "pytest", 30, sys.executable, "-c", "raise SystemExit(3)").returncode == 3


def test_a_part_with_no_limit_reports_its_time_and_fails_never(tmp_path: Path) -> None:
  got = budget(tmp_path, "cargo", 1, sys.executable, "-c", "import time; time.sleep(2)")
  assert got.returncode == 0 and "with no limit yet" in got.stdout


def test_the_hooks_and_the_gates_run_pytest_through_the_one_home_of_its_limit() -> None:
  limits = tomllib.loads((ROOT / "script" / "budgets.toml").read_text(encoding="utf-8"))
  assert 0 < limits["local"]["pytest"] < limits["ci"]["pytest"]
  loaded = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
  hook = next(one for repo in loaded["repos"] for one in repo["hooks"] if one["id"] == "pytest")
  assert hook["entry"].startswith("python3 script/budget.py pytest -- ")
  gates = yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))
  runs = [step.get("run", "") for step in gates["jobs"]["suite"]["steps"]]
  assert any("script/budget.py pytest -- " in run and "uv run pytest" in run for run in runs)
