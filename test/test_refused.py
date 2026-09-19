"""Refused, what a call the engine will not make raises."""

import pytest

from conftest import STANDS, Dead, Sand, life, sown
from furb import engine
from furb.engine import Refused


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_what_a_call_the_engine_will_not_make_raises_in_the_one_that_made_it() -> None:
  """What a call the engine will not make raises in the one that made it."""
  sand = sown()
  life(sand)
  with pytest.raises(Refused, match="names no chain"):
    engine.bash("nowhere")
  with pytest.raises(Refused, match="names no chain"):
    engine.chain("twin", source="chain://operator.9")
  with pytest.raises(Refused, match="none outside one"):
    engine.debug(t"{1}")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_refused_call_raises_refused_in_the_caller() -> None:
  """A refused call raises Refused in the caller."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("BAD = 1", on=root)
  with pytest.raises(Refused):
    await act
  assert isinstance(engine.outcomes[act], Refused)
  dead = Dead(stands=STANDS)
  _, other = life(dead)
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=other) == "a dead World answers no read"
