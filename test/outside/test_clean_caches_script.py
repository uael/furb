# ruff: noqa: S603
import subprocess
import sys
from pathlib import Path

import pytest

fcntl = pytest.importorskip("fcntl")

SCRIPT = Path(__file__).resolve().parents[2] / "script" / "clean-caches.py"
MARK = ".furb-scratch"


def test_the_cleaner_removes_the_scratch_directory_that_holds_only_its_mark_and_no_life_holds(tmp_path: Path) -> None:
  root = tmp_path / "scratch"
  for name in ("a" * 16, "b" * 16, "c" * 16, "d" * 16 + ".new", "notes"):
    (root / name).mkdir(parents=True)
    (root / name / MARK).write_text("", encoding="utf-8")
  (root / ("c" * 16) / "plan.md").write_text("kept", encoding="utf-8")
  (root / ".reaped").write_text("", encoding="utf-8")
  with (root / ("b" * 16) / MARK).open("rb") as life:
    fcntl.flock(life, fcntl.LOCK_SH)
    done = subprocess.run(
      [sys.executable, "-I", str(SCRIPT), "--scratch", str(root), "--uv", str(tmp_path / "uv")],
      capture_output=True,
      text=True,
      check=False,
    )
  assert done.returncode == 0, done.stderr
  assert sorted(one.name for one in root.iterdir()) == [".reaped", "b" * 16, "c" * 16, "notes"]
  assert (root / ("c" * 16) / "plan.md").read_text(encoding="utf-8") == "kept"
  assert "2 scratch directories removed, of 4" in done.stdout
