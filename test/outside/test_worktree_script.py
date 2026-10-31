# ruff: noqa: S603, S607
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

CLEAN = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
SCRIPT = Path(__file__).resolve().parents[2] / "script" / "worktree.sh"


def git(cwd: Path, *args: str) -> None:
  subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True, env=CLEAN)


def clone_with_origin(root: Path) -> Path:
  origin = root / "origin.git"
  subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True, env=CLEAN)
  main = root / "main"
  subprocess.run(["git", "clone", "-q", str(origin), str(main)], check=True, capture_output=True, env=CLEAN)
  (main / "script").mkdir()
  shutil.copy(SCRIPT, main / "script" / "worktree.sh")
  git(main, "add", ".")
  git(main, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "x")
  git(main, "push", "-q", "origin", "HEAD:refs/heads/main")
  git(main, "fetch", "-q", "origin")
  return main


def run(main: Path, path: Path, branch: str) -> subprocess.CompletedProcess[str]:
  env = {**CLEAN, "PATH": "/usr/bin:/bin:" + CLEAN["PATH"]}
  return subprocess.run(
    ["sh", str(main / "script" / "worktree.sh"), str(path), branch],
    capture_output=True,
    text=True,
    env=env,
    check=False,
  )


@pytest.mark.skipif(sys.platform == "win32", reason="the sh of Git on Windows names a path in a form of its own")
def test_failure_names_the_path_and_the_step(tmp_path: Path) -> None:
  main = clone_with_origin(tmp_path)
  git(main, "branch", "held")
  git(main, "worktree", "add", "-q", str(tmp_path / "first"), "held")
  got = run(main, tmp_path / "second", "held")
  assert got.returncode != 0
  last = got.stderr.strip().splitlines()[-1]
  assert last == f'worktree.sh failed at "make the worktree" with exit {got.returncode}: {tmp_path / "second"} on held'


@pytest.mark.skipif(sys.platform == "win32", reason="the sh of Git on Windows names a path in a form of its own")
def test_success_says_bun_run_build_suffices(tmp_path: Path) -> None:
  main = clone_with_origin(tmp_path)
  got = run(main, tmp_path / "new", "fresh")
  assert got.returncode == 0
  assert "bun run build there suffices" in got.stdout
  assert "failed" not in got.stderr


@pytest.mark.skipif(sys.platform == "win32", reason="the sh of Git on Windows names a path in a form of its own")
def test_fetch_runs_again_when_another_fetch_holds_a_ref_lock(tmp_path: Path) -> None:
  main = clone_with_origin(tmp_path)
  real = shutil.which("git")
  assert real
  shim = tmp_path / "shim"
  shim.mkdir()
  count = tmp_path / "count"
  (shim / "git").write_text(
    "#!/bin/sh\n"
    'case " $* " in\n'
    '  *" fetch "*) echo x >> "$COUNT"; if [ "$(wc -l < "$COUNT")" -eq 1 ]; then\n'
    '    echo "error: cannot lock ref refs/remotes/origin/main" >&2; exit 1; fi ;;\n'
    "esac\n"
    f'exec {real} "$@"\n'
  )
  (shim / "git").chmod(0o755)
  env = {**CLEAN, "COUNT": str(count), "PATH": f"{shim}:/usr/bin:/bin:" + CLEAN["PATH"]}
  got = subprocess.run(
    ["sh", str(main / "script" / "worktree.sh"), str(tmp_path / "new"), "fresh"],
    capture_output=True,
    text=True,
    env=env,
    check=False,
  )
  assert got.returncode == 0
  assert "runs again in 1 s: error: cannot lock ref refs/remotes/origin/main" in got.stderr
  assert (tmp_path / "new").is_dir()
