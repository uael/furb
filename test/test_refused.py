"""Refused, what a call the engine will not make raises."""

import pytest

from conftest import Dead, Sand, life, sown
from furb import engine
from furb.engine import Refused


async def test_what_a_call_the_engine_will_not_make_raises_in_the_one_that_made_it() -> None:
  """What a call the engine will not make raises in the one that made it."""
  sand = sown()
  life(sand)
  with pytest.raises(Refused, match="no chain"):
    engine.bash("nowhere")
  with pytest.raises(Refused, match="no chain"):
    engine.chain("twin", source="chain://operator.9")
  with pytest.raises(Refused, match="no act"):
    engine.debug(t"{1}")


async def test_a_refused_call_raises_refused_in_the_caller() -> None:
  """A refused call raises Refused in the caller."""
  sand = Sand()
  _, root = life(sand)
  act = engine.rung("k = BAD", on=root)
  with pytest.raises(Refused):
    await act
  assert isinstance(engine.peek(act), Refused)
  dead = Dead()
  _, other = life(dead)
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=other) == "a dead World answers no read"
