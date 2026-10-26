"""The environment that the DeepSWE rig gives a checkout."""

import importlib
import io
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

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
