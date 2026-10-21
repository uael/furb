"""The harness of the suites, and what keeps a test from the machine that runs it."""

import os
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
