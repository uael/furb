"""What every test of the binding is given.

The World the tests drive, and the helpers that open a life on it, are in `yard`, which is a module and not this
file: the repository holds a `conftest` of its own for the suite of the engine, and one name is one thing.
"""

from pathlib import Path

import pytest


@pytest.fixture
def yard(tmp_path: Path) -> Path:
  """The directory the chains of a life stand in."""
  return tmp_path
