"""No test of the suite reaches the repository of the hook of git that runs it."""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLEAN = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_", "COVERAGE_PROCESS_"))}
GITS = [
  "test/outside/test_paths.py::test_a_folder_that_a_rule_of_git_ignores_as_a_folder_stands_before_a_build_makes_it"
]


def test_the_tests_that_run_git_under_the_hook_of_a_worktree_leave_its_clone_as_it_was(tmp_path: Path) -> None:
  def git(*args: str) -> None:
    who = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"]
    subprocess.run(["git", *who, *args], cwd=tmp_path, env=CLEAN, check=True, capture_output=True)  # noqa: S603, S607

  git("init", "-q", "-b", "main", "main")
  git("-C", "main", "commit", "-q", "--allow-empty", "-m", "base")
  git("-C", "main", "worktree", "add", "-q", "-b", "side", str(tmp_path / "side"))
  config = tmp_path / "main" / ".git" / "config"
  before = config.read_text(encoding="utf-8")
  side = tmp_path / "main" / ".git" / "worktrees" / "side"
  hook = {**CLEAN, "GIT_DIR": str(side), "GIT_INDEX_FILE": str(side / "index")}
  command = [sys.executable, "-m", "pytest", "-q", *GITS, "-p", "no:cacheprovider", "--no-cov", "-n0"]
  got = subprocess.run(command, cwd=ROOT, env=hook, capture_output=True, text=True, timeout=300, check=False)  # noqa: S603
  assert got.returncode == 0, got.stdout[-2000:]
  assert config.read_text(encoding="utf-8") == before
