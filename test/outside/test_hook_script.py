# ruff: noqa: S603, S607
import os
import subprocess
from pathlib import Path

HOOK = Path(__file__).resolve().parents[2] / "script" / "hooks" / "pre-commit"


def run(tmp_path: Path, body: str) -> subprocess.CompletedProcess[str]:
  stub = tmp_path / "stub.sh"
  stub.write_text("#!/bin/sh\n" + body)
  stub.chmod(0o755)
  subprocess.run(["git", "init", "-q", str(tmp_path / "repo")], check=True)
  env = {**os.environ, "PRE_COMMIT": stub.as_posix(), "TMPDIR": tmp_path.as_posix()}
  return subprocess.run(
    ["sh", HOOK.as_posix()], cwd=tmp_path / "repo", env=env, capture_output=True, text=True, check=False
  )


def test_hook_prints_nothing_when_every_hook_passes(tmp_path: Path):
  got = run(tmp_path, "echo 'ruff-check...........Passed'\necho 'fmt...(no files to check)Skipped'\n")
  assert (got.returncode, got.stdout, got.stderr) == (0, "", "")


def test_hook_prints_only_what_fails(tmp_path: Path):
  got = run(
    tmp_path, "echo 'ruff-check...........Passed'\necho 'pytest...........Failed'\necho 'E assert 1 == 2'\nexit 1\n"
  )
  assert got.returncode == 1
  assert got.stdout == "pytest...........Failed\nE assert 1 == 2\n"
