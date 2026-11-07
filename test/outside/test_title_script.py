# ruff: noqa: S603, S607
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HOOKS = ROOT / "script" / "hooks"
TITLE = ROOT / "script" / "title.py"
# A commit hook runs this suite with GIT_DIR and more set, and PRE_COMMIT=1, which the hook pre-commit reads as a command.
CLEAN = {key: value for key, value in os.environ.items() if not key.startswith("GIT_") and key != "PRE_COMMIT"}


def commit(tmp_path: Path, title: str) -> tuple[subprocess.CompletedProcess[str], bool]:
  """A commit with the hooks of the repository on, with a stub for pre-commit, and whether a commit stands after it."""
  repo = tmp_path / "repo"
  subprocess.run(["git", "init", "-q", str(repo)], check=True, env=CLEAN)
  config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", f"core.hooksPath={HOOKS.as_posix()}"]
  got = subprocess.run(
    ["git", *config, "commit", "-q", "--allow-empty", "-m", title],
    cwd=repo,
    env={**CLEAN, "PRE_COMMIT": "true"},
    capture_output=True,
    text=True,
    check=False,
  )
  head = subprocess.run(
    ["git", "rev-parse", "-q", "--verify", "HEAD"], cwd=repo, env=CLEAN, capture_output=True, check=False
  )
  return got, head.returncode == 0


def test_a_commit_with_a_title_that_the_rules_accept_is_made(tmp_path: Path) -> None:
  got, made = commit(tmp_path, "fix(ci): Refuse a bad title")
  assert (got.returncode, got.stdout, got.stderr, made) == (0, "", "", True)


def test_a_commit_with_a_title_over_70_characters_is_refused_and_makes_no_commit(tmp_path: Path) -> None:
  got, made = commit(tmp_path, "fix(ci): " + "A" * 62)
  assert got.returncode != 0
  assert "The title is wrong" in got.stderr
  assert not made


def test_a_message_file_gives_its_first_line_that_is_no_comment_and_lets_a_merge_pass(tmp_path: Path) -> None:
  def check(text: str) -> int:
    message = tmp_path / "message"
    message.write_text(text)
    return subprocess.run(
      [sys.executable, str(TITLE), "--file", str(message)], capture_output=True, check=False
    ).returncode

  assert check("# a note\nfix(ci): Say it\n\nA body that no rule reads.\n") == 0
  assert check("Merge remote-tracking branch 'origin/w/converge' into c/ci\n") == 0
  assert check("fix: No area\n") == 1
