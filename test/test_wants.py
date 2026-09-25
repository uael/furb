"""Wants, the act a run waits for."""

from conftest import STANDS, Sand, dones, life, said, settle
from furb import engine


async def test_a_wants_is_the_act_that_a_run_makes_when_its_word_waits() -> None:
  """A wants is the act that a run makes when its word waits for an act that is not done: the rung takes it, and answers it with what that act came to when the rung hears its done, so a pause over the rung holds the word."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.rung("out = await bash('slow')\nclose(out.code)", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  (run,) = [a[1] for a in said(log, "run") if a[4] == act]
  (wants,) = said(log, "wants")
  assert (wants[2], wants[3], wants[4]) == (run, root, command)
  assert [a[2] for a in said(log, "started") if a[1] == wants[1]] == [act]
  engine.pause(act)
  sand.exits(command, 3)
  await settle()
  assert dones(log, "wants") == [] and engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert [(a[2], a[3]) for a in dones(log, "wants")] == [(act, engine.peek(command))] and (await act) == 3
