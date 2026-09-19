"""ending, the kind that ends an act at its done and at a control over it."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_close_from_outside_still_ends_what_a_pause_is_over() -> None:
  """A close from outside still ends what a pause is over."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "how many?", on=root)
  await settle()
  engine.pause(act)
  engine.close(21, act)
  await settle()
  assert act in engine.outcomes and (await act) == 21


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_it_is_over_as_it_says_a_done_of_its_own() -> None:
  """It is over as it says a done of its own, so it never hears that done and says nothing after it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("close(21)", on=root)
  await act
  ends = [one for one in said(log, "done") if one[1] == act]
  assert [(one[2], one[3]) for one in ends] == [(act, 21)]
  mark = len(log)
  engine.cancel(act)
  await settle()
  assert [one for one in log[mark:] if one[2] == act] == []
  assert engine.outcomes[act] == 21
