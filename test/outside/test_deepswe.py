"""The environment that the DeepSWE rig gives a checkout."""

import importlib
import io
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest


@pytest.mark.skipif(os.name == "nt", reason="the DeepSWE checkout uses POSIX command paths")
def test_a_copy_uses_the_image_environment_and_its_own_python_and_source(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  """A copied checkout uses the image environment and its own interpreter and source."""
  monkeypatch.syspath_prepend(str(Path(__file__).parents[2] / "script"))
  deepswe = importlib.import_module("deepswe")

  task = tmp_path / "task"
  task.mkdir()
  (task / "task.toml").write_text(f'[environment]\ndocker_image = "fixture/{tmp_path.name}:pin"\n')

  def registry(_named: str, path: str, _accept: str) -> io.BytesIO:
    data = (
      {"config": {"digest": "config"}}
      if path.startswith("manifests/")
      else {"config": {"Env": ["PYTHONPATH=/app/src", "PATH=/opt/venv/bin:/usr/bin", "PROBE_VALUE=image"]}}
    )
    return io.BytesIO(json.dumps(data).encode())

  monkeypatch.setattr(deepswe, "registry", registry)
  base, copy = tmp_path / "base", tmp_path / "copy"
  for repo, value in [(base, "base"), (copy, "copy")]:
    (repo / "src").mkdir(parents=True)
    (repo / "src" / "seed_payload.py").write_text(f'VALUE = "{value}"\n')
    (repo / ".venv" / "bin").mkdir(parents=True)
    (repo / ".venv" / "bin" / "python").symlink_to(sys.executable)
  env = {**os.environ, **deepswe.task_env(task, copy)}
  executable = shutil.which("python", path=env["PATH"])
  assert executable is not None
  got = subprocess.run(  # noqa: S603
    [
      executable,
      "-c",
      'import os,sys,seed_payload; print(seed_payload.VALUE, os.environ["PROBE_VALUE"]); print(sys.executable)',
    ],
    cwd=base,
    env=env,
    capture_output=True,
    text=True,
    check=True,
    timeout=10,
  )
  assert got.stdout.splitlines() == ["copy image", str(copy / ".venv" / "bin" / "python")]


def rig(monkeypatch: pytest.MonkeyPatch) -> ModuleType:
  for name in [name for name in os.environ if name.startswith("GIT_")]:
    monkeypatch.delenv(name)
  monkeypatch.syspath_prepend(str(Path(__file__).parents[2] / "script"))
  return importlib.import_module("deepswe")


def git(repo: Path, *args: str) -> str:
  command = ["git", "-c", "user.email=t@t", "-c", "user.name=t", *args]
  return subprocess.run(command, cwd=repo, capture_output=True, text=True, check=True).stdout.strip()  # noqa: S603


@pytest.mark.skipif(shutil.which("git") is None, reason="the DeepSWE rig runs git")
def test_a_checkout_is_changed_once_it_differs_from_the_base(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
  """A checkout is changed once a file differs from the base, edited, added or committed."""
  deepswe = rig(monkeypatch)
  repo = tmp_path / "repo"
  repo.mkdir()
  git(repo, "init", "-q")
  (repo / "a.txt").write_text("a\n")
  git(repo, "add", "-A")
  git(repo, "commit", "-q", "-m", "base")
  base = git(repo, "rev-parse", "HEAD")
  assert deepswe.changed(repo, base) is False
  (repo / "a.txt").write_text("b\n")
  assert deepswe.changed(repo, base) is True
  git(repo, "checkout", "--", "a.txt")
  (repo / "new.txt").write_text("n\n")
  assert deepswe.changed(repo, base) is True
  git(repo, "add", "-A")
  git(repo, "commit", "-q", "-m", "work")
  assert deepswe.changed(repo, base) is True


def test_the_dev_and_test_requirements_of_a_python_checkout_are_installed(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  """A python checkout gets each requirements file that names dev or test, and no other."""
  deepswe = rig(monkeypatch)
  for name in ("requirements.txt", "requirements-dev.txt", "requirements_test.txt"):
    (tmp_path / name).write_text("")
  calls: list[list[str]] = []

  def ran(args: list[str], **_: object) -> int:
    calls.append(list(args))
    return 0

  monkeypatch.setattr(deepswe, "ran", ran)
  deepswe.required(tmp_path, {}, deepswe.SAID)
  assert [call[2].split("-q -r ")[1] for call in calls] == ["requirements-dev.txt", "requirements_test.txt"]
