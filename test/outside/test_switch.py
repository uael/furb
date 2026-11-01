"""The switch of the package: FURB_ENGINE picks the engine once, where the package is imported."""

import importlib
import sys

import pytest

import furb
import furb_monty.engine


def test_furb_engine_gives_the_engine_of_monty_when_furb_engine_is_monty(monkeypatch: pytest.MonkeyPatch) -> None:
  """`from furb import engine` gives the engine of monty when FURB_ENGINE is monty, the engine of this interpreter
  otherwise, and `furb.python` is the engine of this interpreter either way."""
  python = furb.python
  monkeypatch.setenv("FURB_ENGINE", "monty")
  try:
    importlib.reload(furb)
    assert furb.MONTY and furb.python is python
    assert sys.modules["furb.engine"] is furb_monty.engine
    assert vars(furb)["engine"] is furb_monty.engine
  finally:
    monkeypatch.delenv("FURB_ENGINE")
    importlib.reload(furb)
  assert not furb.MONTY and furb.python is python
  assert sys.modules["furb.engine"] is python
  assert vars(furb)["engine"] is python
