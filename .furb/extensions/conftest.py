"""The conftest of the suites of the extensions of the project: no test sees a variable of git that a commit sets."""

import os

import pytest


@pytest.fixture(autouse=True)
def no_git_variables(monkeypatch: pytest.MonkeyPatch) -> None:
  """A commit through the hook sets GIT_INDEX_FILE and its kin, which each git of a test would inherit."""
  for name in [name for name in os.environ if name.startswith("GIT_")]:
    monkeypatch.delenv(name)
