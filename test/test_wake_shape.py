"""Wake, the fact that ends a pause."""

from conftest import STANDS, Sand, job, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_a_wake_ends_the_pause_over_the_same_act_and_what_waited_is_heard() -> None:
  """A wake ends the pause over the same act, and what waited is heard."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = job(root)
  await settle()
  engine.pause(root)
  engine.send("finished", act, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert said(log, "wake") == [("wake", root, OPERATOR, [f"#{root} woke"])]
  assert act in engine.outcomes and (await act) == 0


async def test_a_wake_is_over_the_act_it_names_and_everything_under_it() -> None:
  """A wake is over the act it names and everything under it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  word = (
    "def fin(id):\n"
    "  while True:\n"
    "    match (yield):\n"
    "      case ('finished', about, _, value) if about == id:\n"
    "        yield 'done', id, value\n"
    "        return\n"
    "\n"
    "out = await act('job', '', pausing(fin))\n"
    "close(out)\n"
  )
  act = engine.rung(word, on=root)
  await settle()
  made = said(log, "job")[0][1]
  engine.pause(act)
  engine.send("finished", made, 0, by=WORLD)
  await settle()
  assert made not in engine.outcomes
  engine.wake(act)
  await settle()
  assert engine.outcomes[made] == 0
  assert await act == 0
