"""idle, the generator of the file that listens and says nothing."""

import pytest

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_idle_hears_every_fact_and_says_nothing_of_its_own() -> None:
  """idle hears every fact and says nothing of its own, which is the ear of a wait and the ear of the operator."""
  hears = engine.idle("wait://operator.1.1")
  assert next(hears) is None
  assert hears.send(("tell", "chain://one", OPERATOR, [])) is None
  assert hears.send(("done", "rung://operator.2.1", OPERATOR, 1)) is None
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.wait(0, on=root)
  assert (await act) is None
  assert [one for one in said(log, "tell") if one[1] == act] == []
  with pytest.raises(Refused, match="operator hears"):
    engine.drive(engine.idle(OPERATOR), OPERATOR)
