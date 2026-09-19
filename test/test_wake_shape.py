"""Wake, the fact that ends a pause."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD, Exit


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_wake_ends_the_pause_over_the_same_act_and_what_waited_is_heard() -> None:
  """A wake ends the pause over the same act, and what waited is heard."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  engine.pause(root)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert [(a[1], a[3]) for a in said(log, "wake")] == [(root, [("woke", [("over", root)], None)])]
  assert act in engine.outcomes and (await act).code == 0


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_wake_is_over_the_act_it_names_and_everything_under_it() -> None:
  """A wake is over the act it names and everything under it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.rung("out = await bash('slow')\nclose(out.code)", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  engine.pause(act)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert command not in engine.outcomes
  engine.wake(act)
  await settle()
  got = engine.outcomes[command]
  assert isinstance(got, Exit) and got.code == 0
  assert await act == 0
