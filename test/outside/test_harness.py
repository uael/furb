"""The harness of the suites, and what keeps a test from the machine that runs it."""

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]


def test_a_test_of_an_extension_sees_no_skill_of_a_folder_above_the_temporary_folder(tmp_path: Path) -> None:
  """A test of an extension sees only the skills that it gives, when the temporary folder of the system lies under a
  home that holds a skill."""
  skills = tmp_path / "home" / ".claude" / "skills" / "stray"
  skills.mkdir(parents=True)
  (skills / "SKILL.md").write_text("---\ndescription: Of the machine.\n---\n", encoding="utf-8")
  (temp := tmp_path / "home" / "tmp").mkdir()
  env = {key: value for key, value in os.environ.items() if not key.startswith(("PYTEST_", "COV_CORE_"))}
  ran = subprocess.run(
    [sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider", "-n", "0", "--no-cov", "extensions/skills/test"],
    env={**env, "TMPDIR": str(temp)},
    cwd=ROOT,
    capture_output=True,
    text=True,
    check=False,
  )
  assert ran.returncode == 0, ran.stdout[-4000:]


def test_a_conftest_of_another_folder_does_not_shadow_the_harness() -> None:
  """A conftest.py in a test folder under .furb/extensions leaves each test importing the one harness module."""
  folder = ROOT / ".furb" / "extensions" / "zz_shadow" / "test"
  folder.mkdir(parents=True)
  try:
    (folder / "conftest.py").write_text('"""Another conftest."""\n', encoding="utf-8")
    (folder / "test_same.py").write_text("def test_same() -> None:\n  assert True\n", encoding="utf-8")
    env = {key: value for key, value in os.environ.items() if not key.startswith(("PYTEST_", "COV_CORE_"))}
    ran = subprocess.run(  # noqa: S603
      [
        sys.executable,
        *("-m", "pytest", "-q", "-p", "no:cacheprovider", "-n", "0", "--no-cov"),
        *("test/test_write.py", "test/test_writing.py", ".furb/extensions", "--ignore=.furb/extensions/conductor"),
      ],
      env=env,
      cwd=ROOT,
      capture_output=True,
      text=True,
      check=False,
    )
  finally:
    shutil.rmtree(ROOT / ".furb" / "extensions" / "zz_shadow")
  assert ran.returncode == 0, ran.stdout[-4000:]
