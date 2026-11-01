"""The harness of the suites, and what keeps a test from the machine that runs it."""

import os
import shutil
import subprocess
import sys
import tempfile
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
  """A conftest.py in a test folder outside the harness leaves each test importing the one harness module."""
  # The folder stands under the repo and not under the temporary folder. Pytest lists each folder from the common
  # ancestor of its paths down, and the temporary folder holds the folders that other tests make and delete, which
  # fail the listing when one goes between the list and the stat.
  base = ROOT / ".furb" / "tests"
  base.mkdir(parents=True, exist_ok=True)
  folder = Path(tempfile.mkdtemp(prefix="shadow-", dir=base))
  try:
    nested = folder / "extension" / "test"
    nested.mkdir(parents=True)
    (folder / "conftest.py").write_text('"""Another conftest."""\n', encoding="utf-8")
    (nested / "conftest.py").write_text('"""Another conftest."""\n', encoding="utf-8")
    (nested / "test_same.py").write_text("def test_same() -> None:\n  assert True\n", encoding="utf-8")
    env = {key: value for key, value in os.environ.items() if not key.startswith(("PYTEST_", "COV_CORE_"))}
    ran = subprocess.run(  # noqa: S603
      [
        sys.executable,
        *("-m", "pytest", "-q", "-p", "no:cacheprovider", "-n", "0", "--no-cov"),
        *("test/test_write.py::test_a_write_whoever_serves_the_path_of_the_text_takes_its_content", str(folder)),
      ],
      env=env,
      cwd=ROOT,
      capture_output=True,
      text=True,
      check=False,
    )
  finally:
    shutil.rmtree(folder, ignore_errors=True)
  assert ran.returncode == 0, ran.stdout[-4000:]
