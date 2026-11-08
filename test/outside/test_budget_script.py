# ruff: noqa: S603
import os
import subprocess
import sys
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "script" / "budget.py"
# A command that uses 1.2 s of CPU, and one that waits 1.2 s with none: each just over the limit of 1 s of the tests.
BUSY = [sys.executable, "-c", "import time\nt = time.process_time()\nwhile time.process_time() - t < 1.2:\n  pass"]
WAIT = [sys.executable, "-c", "import time; time.sleep(1.2)"]


def budget(
  tmp: Path, part: str, wall: int | None, *command: str, cpu: int | None = None, load: str = "0"
) -> subprocess.CompletedProcess[str]:
  """The run of a command as a part, under a home that gives the part these limits and a machine of this load."""
  table = "".join(f"{key} = {value}\n" for key, value in ((part, wall), (part + "_cpu", cpu)) if value)
  (tmp / "budgets.toml").write_text(f"[local]\n{table}[ci]\n{table}", encoding="utf-8")
  env = {n: v for n, v in os.environ.items() if n != "CI"} | {"FURB_BUDGETS": str(tmp / "budgets.toml")}
  env["FURB_BUDGET_LOAD"] = load
  env["FURB_WATCH_LOG"] = str(tmp / "watch.log")
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
  cargo = next(one for repo in loaded["repos"] for one in repo["hooks"] if one["id"] == "cargo-test")
  assert cargo["entry"] == "python3 script/hooks/reach.py cargo"
  assert '"budget.py"' in (ROOT / "script" / "hooks" / "reach.py").read_text(encoding="utf-8")


def killed_after(log: Path, when: str, program: str) -> list[str]:
  """A command that writes a line of the log of the guard of memory, at a time given as an expression, then ends by SIGKILL,
  as a test binary that the guard ended does."""
  line = f"f'{{{when}}} 4242 {program} --test-threads 4\\n'"
  body = f"import os, signal, time\nopen({str(log)!r}, 'a').write({line})\nos.kill(os.getpid(), signal.SIGKILL)"
  return [sys.executable, "-c", body]


def test_a_kill_that_the_log_of_the_guard_of_memory_holds_in_the_run_is_named_as_its(tmp_path: Path) -> None:
  program = str(tmp_path / "target" / "debug" / "deps" / "furb-3bfcea354560c7bc")
  got = budget(tmp_path, "cargo", 100, *killed_after(tmp_path / "watch.log", "int(time.time())", program))
  assert got.returncode != 0
  assert "the guard of memory, script/memwatch.sh, ended a test process of cargo" in got.stdout
  assert f"pid 4242: {program} --test-threads 4" in got.stdout


def test_a_kill_that_the_log_holds_not_in_the_run_or_of_its_folder_names_no_guard(tmp_path: Path) -> None:
  ours, other = (
    str(tmp_path / "target" / "debug" / "deps" / "furb-3bfcea354560c7bc"),
    "/elsewhere/target/debug/deps/x-1a",
  )
  for when, program in (("int(time.time()) - 3600", ours), ("int(time.time())", other)):
    got = budget(tmp_path, "cargo", 100, *killed_after(tmp_path / "watch.log", when, program))
    assert got.returncode != 0 and "guard of memory" not in got.stdout, (when, program)


def test_a_sigkill_with_no_line_of_the_guard_names_no_guard(tmp_path: Path) -> None:
  command = "import os, signal; print('(signal: 9, SIGKILL: kill)', flush=True); os.kill(os.getpid(), signal.SIGKILL)"
  got = budget(tmp_path, "cargo", 100, sys.executable, "-c", command)
  assert got.returncode != 0 and "guard of memory" not in got.stdout


def test_a_line_of_the_guard_whose_program_is_a_bare_name_that_the_run_prints_names_no_guard(tmp_path: Path) -> None:
  line = "f'{int(time.time())} 4242 bun test ./a.test.ts\\n'"
  body = (
    f"import os, signal, time\nopen({str(tmp_path / 'watch.log')!r}, 'a').write({line})\n"
    "print('bun test v1.3.14', flush=True)\nos.kill(os.getpid(), signal.SIGKILL)"
  )
  got = budget(tmp_path, "bun", 100, sys.executable, "-c", body)
  assert got.returncode != 0 and "bun test v1.3.14" in got.stdout and "guard of memory" not in got.stdout
