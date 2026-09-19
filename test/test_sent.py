"""Sent, what an act a run waited for came to."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import Exit


async def test_a_sent_carries_to_a_run_what_the_act_it_waited_for_came_to() -> None:
  """A sent carries to a run what the act it waited for came to."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("out = await bash('echo hi')\nclose(out.code)", on=root)
  assert await act == 0
  _, command, *_ = said(log, "bash")[0]
  carried = said(log, "sent")
  assert [(a[1], a[2]) for a in carried] == [(act, act)]
  assert carried[0][3] is engine.peek(command, on=root)
  assert isinstance(carried[0][3], Exit) and carried[0][3].code == 0
