"""The harness for the modules around the engine: the World, the Kernel and the door.

The engine's own laws are proved in test/, against engine.pyi, sentence for sentence. What is proved here is what
stands outside it: a World that answers from a script, a model of the provider of the crate that answers from one,
and a Kernel that runs words for a World under test.
"""

import os
from pathlib import Path

import pytest


@pytest.fixture
def yard(tmp_path: Path) -> Path:
  """A directory of its own for one test, with the file a word of the suite reads."""
  (tmp_path / "a.txt").write_text("one\ntwo\nthree\n", encoding="utf-8")
  return tmp_path


@pytest.fixture(autouse=True)
def no_git_variables(monkeypatch: pytest.MonkeyPatch) -> None:
  """A commit through the hook sets GIT_DIR and GIT_INDEX_FILE, which lead each git of a test to the clone."""
  for name in [name for name in os.environ if name.startswith("GIT_")]:
    monkeypatch.delenv(name)
