"""raised, the exception that the last rung of a chain raised."""

import pytest

from conftest import Sand, life
from furb import engine
from furb.engine import Refused


async def test_raised_is_the_exception_object_that_the_last_rung_raised_rebound_at_each_raise() -> None:
  """raised is the exception object that the last rung raised, rebound at each raise."""
  sand = Sand()
  _, root = life(sand)
  assert engine.module(root)["raised"] is None
  with pytest.raises(ValueError, match="one"):
    await engine.rung("raise ValueError('one')", on=root)
  first = engine.module(root)["raised"]
  assert isinstance(first, ValueError) and str(first) == "one"
  with pytest.raises(KeyError, match="two"):
    await engine.rung("raise KeyError('two')", on=root)
  second = engine.module(root)["raised"]
  assert isinstance(second, KeyError) and second is not first
  with pytest.raises(Refused):
    await engine.rung("k = BAD", on=root)
  assert engine.module(root)["raised"] is second
  assert await engine.rung("close(str(raised))", on=root) == "'two'"


async def test_raised_is_none_at_the_birth_of_the_module_of_a_chain() -> None:
  """raised is None at the birth of the module of a chain, so a word reads it before any rung raised."""
  sand = Sand()
  _, root = life(sand)
  assert engine.module(root)["raised"] is None
  assert await engine.rung("close(raised)", on=root) is None
  with pytest.raises(ValueError, match="once"):
    await engine.rung("raise ValueError('once')", on=root)
  fresh = engine.chain("fresh", on=root)
  assert engine.module(fresh)["raised"] is None
