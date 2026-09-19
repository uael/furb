"""Wants, the act a run waits for."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_a_wants_says_the_act_a_run_waits_for() -> None:
  """A wants says the act a run waits for."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.rung("out = await bash('slow')\nclose(out.code)", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  assert [(a[1], a[3]) for a in said(log, "wants")] == [(act, command)]
  assert act not in engine.outcomes
  engine.send("exited", command, 3, by=WORLD)
  await settle()
  assert (await act) == 3
