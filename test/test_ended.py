"""ended, what an act a control is over is done with."""

from asyncio import CancelledError

from conftest import STANDS, Sand, life, said, settle
from furb import engine


async def test_what_an_act_a_control_is_over_is_done_with() -> None:
  """What an act a control is over is done with: the value a close carries for the act it names, and a CancelledError for every other."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  engine.close(21, act)
  await settle()
  shut = said(log, "close")[0]
  assert engine.ended(shut, act) == 21
  assert isinstance(engine.ended(shut, step), CancelledError)
  assert engine.peek(act) == 21 and isinstance(engine.peek(step), CancelledError)
