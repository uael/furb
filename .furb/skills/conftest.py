"""The conftest of the suites of the extensions of the project: no test sees a variable of git that a commit sets, and a
test runs on the engine that engine_of names, and pulling is a Bench that plays pr in a folder with the template, title.py
and the config of release-please of the repository."""

import os
import shutil
from pathlib import Path

import pytest

from conftest import engine_of  # noqa: F401
from skills.conftest import Bench
from furb.engine import OPERATOR


@pytest.fixture(autouse=True)
def no_git_variables(monkeypatch: pytest.MonkeyPatch) -> None:
  """A commit through the hook sets GIT_INDEX_FILE and its kin, which each git of a test would inherit."""
  for name in [name for name in os.environ if name.startswith("GIT_")]:
    monkeypatch.delenv(name)


REPO = Path(__file__).resolve().parents[2]


@pytest.fixture
async def pulling(tmp_path: Path) -> Bench:
  (tmp_path / "script").mkdir()
  shutil.copy(REPO / "script" / "title.py", tmp_path / "script")
  shutil.copy(REPO / "release-please-config.json", tmp_path)
  made = Bench(tmp_path, [REPO / ".furb" / "skills" / "pr" / "pr.py"], [[OPERATOR, [], 200000]], OPERATOR)
  made.sand.files[f"{tmp_path}/.github/pull_request_template.md"] = (REPO / ".github" / "pull_request_template.md").read_text(encoding="utf-8")
  await made.load()
  return made
