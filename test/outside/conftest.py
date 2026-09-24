"""The harness for the four modules around the engine: the World, the Kernel, the provider and the command line.

The engine's own laws are proved in test/, against engine.pyi, sentence for sentence. What is proved here is what
stands outside it: a claude that answers from a script, a model of pydantic_ai that answers from one, a World that
answers from one, and a Kernel that runs words for a World under test.
"""

import os
import sys
from collections.abc import AsyncGenerator
from pathlib import Path

import pytest

from furb.provider.claude import CLI, cool


@pytest.fixture(autouse=True)
def placed(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
  """No config and no cache of the extensions of the developer reach a test: each is an empty directory of the test."""
  monkeypatch.setenv("FURB_CONFIG_DIR", str(tmp_path / "config"))
  monkeypatch.setenv("FURB_CACHE_DIR", str(tmp_path / "cache"))


@pytest.fixture(autouse=True)
async def cooled() -> AsyncGenerator[None]:
  """No claude outlives the test that spawned it, and no pool outlives it either."""
  yield
  await cool()
  CLI.clear()


@pytest.fixture
def fake(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
  """A claude on the path that answers from a script, and the yard it records what it was asked in."""
  home = tmp_path / "fake"
  home.mkdir()
  script = tmp_path / ("claude.cmd" if os.name == "nt" else "claude")
  said = Path(__file__).parent / "fake_claude.py"
  if os.name == "nt":
    script.write_text(f'@"{sys.executable}" "{said}" %*\n')
  else:
    script.write_text(f'#!/bin/sh\nexec "{sys.executable}" "{said}" "$@"\n')
    script.chmod(0o755)
  monkeypatch.setenv("FURB_CLAUDE_BIN", str(script))
  monkeypatch.setenv("FURB_FAKE_HOME", str(home))
  return home


@pytest.fixture
def yard(tmp_path: Path) -> Path:
  """A directory of its own for one test, with the file a word of the suite reads."""
  (tmp_path / "a.txt").write_text("one\ntwo\nthree\n", encoding="utf-8")
  return tmp_path
