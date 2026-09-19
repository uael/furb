"""Ran, what the word of a rung gave."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Sand, life, said
from furb import engine


async def test_a_ran_says_what_the_word_of_a_rung_gave() -> None:
  """A ran says what the word of a rung gave."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  assert await engine.rung("close(21)", on=root) == 21
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  gave = said(log, "ran")
  assert gave[0][3] is None and isinstance(gave[1][3], CancelledError)
  assert isinstance(gave[2][3], ValueError) and str(gave[2][3]) == "boom"
  assert [a[1] for a in gave] == [a[1] for a in said(log, "rung")]
