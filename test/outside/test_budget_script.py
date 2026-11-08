# ruff: noqa: S603
import os
import subprocess
import sys
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "script" / "budget.py"
# A command that uses about two seconds of CPU, and one that waits for two seconds with none.
BUSY = [sys.executable, "-c", "import time\nt = time.process_time()\nwhile time.process_time() - t < 2:\n  pass"]
WAIT = [sys.executable, "-c", "import time; time.sleep(2)"]


def budget(
  tmp: Path, part: str, wall: int | None, *command: str, cpu: int | None = None, load: str = "0"
) -> subprocess.CompletedProcess[str]:
  """The run of a command as a part, under a home that gives the part these limits and a machine of this load."""
  table = "".join(f"{key} = {value}\n" for key, value in ((part, wall), (part + "_cpu", cpu)) if value)
  (tmp / "budgets.toml").write_text(f"[local]\n{table}[ci]\n{table}", encoding="utf-8")
  env = {n: v for n, v in os.environ.items() if n != "CI"} | {"FURB_BUDGETS": str(tmp / "budgets.toml")}
  env["FURB_BUDGET_LOAD"] = load
  args = [sys.executable, str(SCRIPT), part, "--", *command]
  return subprocess.run(args, env=env, cwd=tmp, capture_output=True, text=True, timeout=120, check=False)


def pytest_of(tmp: Path, body: str) -> list[str]:
  (tmp / "test_part.py").write_text(body, encoding="utf-8")
  (tmp / "pytest.ini").write_text("[pytest]\n", encoding="utf-8")
  args = ["-q", "-c", "pytest.ini", "-p", "no:cacheprovider", "--durations=5", "test_part.py"]
  return [sys.executable, "-m", "pytest", *args]


def test_a_run_that_passes_its_limit_fails_and_names_its_slowest_test(tmp_path: Path) -> None:
  body = "import time\n\n\ndef test_sleeps_past_the_bound():\n  time.sleep(2)\n"
  got = budget(tmp_path, "pytest", 1, *pytest_of(tmp_path, body))
  assert got.returncode == 1
  assert "passed its wall limit" in got.stdout
  assert "test_part.py::test_sleeps_past_the_bound" in got.stdout.split("Its slowest tests:")[1]


def test_the_same_run_with_no_slow_test_passes_under_its_limit(tmp_path: Path) -> None:
  got = budget(tmp_path, "pytest", 30, *pytest_of(tmp_path, "def test_quick():\n  pass\n"))
  assert got.returncode == 0
  assert "passed its" not in got.stdout and "of 30 s" in got.stdout


def test_a_command_that_fails_gives_its_own_code(tmp_path: Path) -> None:
  assert budget(tmp_path, "pytest", 30, sys.executable, "-c", "raise SystemExit(3)").returncode == 3


def test_a_part_with_no_limit_reports_its_time_and_fails_never(tmp_path: Path) -> None:
  got = budget(tmp_path, "cargo", None, *WAIT)
  assert got.returncode == 0 and "with no limit yet" in got.stdout


def test_a_loaded_machine_judges_a_run_past_its_wall_limit_by_its_cpu_time(tmp_path: Path) -> None:
  got = budget(tmp_path, "pytest", 1, *WAIT, cpu=30, load="999")
  assert got.returncode == 0
  assert "while the load was 999" in got.stdout and "its CPU time fits" in got.stdout


def test_a_load_just_over_the_cores_already_judges_a_run_by_its_cpu_time(tmp_path: Path) -> None:
  busy = str((os.cpu_count() or 1) + 2)
  got = budget(tmp_path, "pytest", 1, *WAIT, cpu=30, load=busy)
  assert got.returncode == 0
  assert "its CPU time fits" in got.stdout


def test_a_quiet_machine_judges_the_same_run_by_its_wall_time(tmp_path: Path) -> None:
  got = budget(tmp_path, "pytest", 1, *WAIT, cpu=30, load="0")
  assert got.returncode == 1 and "passed its wall limit" in got.stdout


def test_a_run_past_its_cpu_limit_fails_whatever_the_load(tmp_path: Path) -> None:
  for load in ("0", "999"):
    got = budget(tmp_path, "pytest", 100, *BUSY, cpu=1, load=load)
    assert got.returncode == 1, load
    assert "of CPU, " in got.stdout and "over its limit" in got.stdout, load


def test_the_hooks_and_the_gates_run_pytest_through_the_one_home_of_its_limit() -> None:
  limits = tomllib.loads((ROOT / "script" / "budgets.toml").read_text(encoding="utf-8"))
  assert 0 < limits["local"]["pytest"] < limits["ci"]["pytest"]
  assert limits["local"]["pytest_cpu"] > 0
  loaded = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
  hook = next(one for repo in loaded["repos"] for one in repo["hooks"] if one["id"] == "pytest")
  assert hook["entry"].startswith("python3 script/budget.py pytest -- ")
  gates = yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))
  runs = [step.get("run", "") for step in gates["jobs"]["suite"]["steps"]]
  assert any("script/budget.py pytest -- " in run and "uv run pytest" in run for run in runs)
