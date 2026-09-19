"""raised, the exception that the last rung of a chain raised."""

import pytest

from conftest import STANDS, Sand, life
from furb import engine


async def test_raised_is_the_exception_object_that_the_last_rung_raised_rebound_at_each_raise() -> None:
  """raised is the exception object that the last rung raised, rebound at each raise."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert "raised" not in engine.modules[root]
  with pytest.raises(ValueError, match="one"):
    await engine.rung("raise ValueError('one')", on=root)
  first = engine.modules[root]["raised"]
  assert isinstance(first, ValueError) and str(first) == "one"
  with pytest.raises(KeyError, match="two"):
    await engine.rung("raise KeyError('two')", on=root)
  second = engine.modules[root]["raised"]
  assert isinstance(second, KeyError) and second is not first
  assert await engine.rung("close(str(raised))", on=root) == "'two'"
