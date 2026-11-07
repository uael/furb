"""The step of the scope in the gates gives a render of the gallery no job, on its pull request and on its push to main."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]
GIT = shutil.which("git") or "git"
BASH = shutil.which("bash") or "bash"


def step() -> str:
  workflow = yaml.safe_load((ROOT / ".github" / "workflows" / "gates.yml").read_text(encoding="utf-8"))
  [scope] = [one for one in workflow["jobs"]["scope"]["steps"] if one.get("id") == "scope"]
  return scope["run"]


def git(where: Path, *words: str) -> str:
  named = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
  clean = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
  command = [GIT, "-C", str(where), *words]
  return subprocess.run(command, check=True, capture_output=True, text=True, env={**clean, **named}).stdout.strip()  # noqa: S603


@pytest.mark.parametrize("event", ["pull_request", "push"])
def test_a_render_of_the_gallery_alone_runs_no_job_on_its_pull_request_and_on_its_push_to_main(
  tmp_path: Path, event: str
) -> None:
  origin, clone = tmp_path / "origin", tmp_path / "clone"
  (origin / "script").mkdir(parents=True)
  shutil.copy(ROOT / "script" / "scope.py", origin / "script" / "scope.py")
  (origin / "docs" / "screenshots").mkdir(parents=True)
  (origin / "docs" / "screenshots" / "tui.png").write_bytes(b"old")
  git(origin, "init", "-q", "-b", "main")
  git(origin, "add", "-A")
  git(origin, "commit", "-q", "-m", "base")
  before = git(origin, "rev-parse", "HEAD")
  (origin / "docs" / "screenshots" / "tui.png").write_bytes(b"new")
  git(origin, "commit", "-q", "-am", "render")
  git(tmp_path, "clone", "-q", str(origin), str(clone))
  output = tmp_path / "output"
  env = {**os.environ, "GITHUB_EVENT_NAME": event, "BEFORE": before, "GITHUB_OUTPUT": str(output)}
  subprocess.run([BASH, "-e", "-c", step()], cwd=clone, env=env, check=True, capture_output=True)  # noqa: S603
  got = dict(line.split("=", 1) for line in output.read_text().splitlines())
  assert got["code"] == "false" and got["crate"] == "false" and got["hooks"] == "false"
