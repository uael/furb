"""Sent, what an act a run waited for came to."""

from conftest import STANDS, Sand, job, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_a_sent_carries_to_a_run_what_the_act_it_waited_for_came_to() -> None:
  """A sent carries to a run what the act it waited for came to."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = job(root)
  act = engine.rung(f"out = await Act({one!r})\nclose(out)", on=root)
  await settle()
  engine.send("finished", one, {"code": 0}, by=WORLD)
  assert await act == {"code": 0}
  carried = said(log, "sent")
  assert [(a[1], a[2]) for a in carried] == [(act, act)]
  assert carried[0][3] == engine.peek(one, on=root) == {"code": 0}
