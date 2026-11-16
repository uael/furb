"""The job of the crate runs each test program of the table of the build, and fails when its selection hides one."""

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]
BASH = shutil.which("bash") or "bash"

pytestmark = pytest.mark.skipif(sys.platform == "win32", reason="the step runs in the bash of the runner")


def step() -> str:
  """The step of the job of the crate that runs the test programs."""
  jobs = yaml.safe_load((ROOT / ".github" / "workflows" / "system.yml").read_text(encoding="utf-8"))["jobs"]
  [run] = [one["run"] for one in jobs["crate"]["steps"] if "programs.tsv" in one.get("run", "")]
  return run


def ran(tmp_path: Path, rows: list[tuple[str, str, str]], *, empty: str = "") -> subprocess.CompletedProcess[str]:
  """The step on macOS, on a table of test programs that each note their folder and their filter and say that they
  run a test, but for the program `empty`, which runs none."""
  table = []
  for folder, name, program in rows:
    path = tmp_path / program
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0 if program == empty else 1
    path.write_text(f'#!/bin/sh\necho "{folder} $*" >> "{tmp_path}/ran"\necho "running {count} tests"\n')
    (tmp_path / folder).mkdir(parents=True, exist_ok=True)
    table.append(f"{folder}\ttest\t{name}\t{program}\n")
  (tmp_path / "programs.tsv").write_text("".join(table))
  env = {**os.environ, "RUNNER_OS": "macOS", "GITHUB_WORKSPACE": str(tmp_path)}
  return subprocess.run(  # noqa: S603
    [BASH, "-e", "-o", "pipefail", "-c", step()], cwd=tmp_path, env=env, capture_output=True, text=True, check=False
  )


ROWS = [
  (".", "suite", "target/suite-a"),
  ("crates/window", "suite", "target/suite-b"),
  ("crates/cli", "suite", "target/suite-c"),
  ("crates/host", "suite", "target/suite-d"),
]


def test_on_macos_the_tests_of_the_crate_and_of_the_window_run_and_the_kills_of_the_command_line_alone(
  tmp_path: Path,
) -> None:
  got = ran(tmp_path, ROWS)
  assert got.returncode == 0, got.stdout
  assert (tmp_path / "ran").read_text().splitlines() == [". ", "crates/window ", "crates/cli kills::"]


def test_a_folder_of_the_selection_that_no_program_of_the_table_holds_fails_the_job(tmp_path: Path) -> None:
  # The folders before the move of the crates, which the selection knows no more.
  got = ran(tmp_path, [(".", "suite", "target/suite-a"), ("window", "suite", "target/suite-b")])
  assert got.returncode == 1
  assert "::error::No test program of the folder crates/window ran." in got.stdout
  assert "::error::No test program of the folder crates/cli ran." in got.stdout


def test_a_filter_that_selects_no_test_fails_the_job(tmp_path: Path) -> None:
  got = ran(tmp_path, ROWS, empty="target/suite-c")
  assert got.returncode == 1
  assert "::error::The filter kills:: selects no test of crates/cli suite." in got.stdout
