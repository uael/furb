"""Wake, the fact that ends a pause."""

from conftest import born, heads, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_wake_ends_the_pause_over_the_same_act_and_what_waited_is_heard() -> None:
  """A wake ends the pause over the same act, and what waited is heard."""
  sand, log, root = born(auto=False)
  act = engine.bash("slow", on=root)
  await settle()
  engine.pause(root)
  sand.exits(act, 0)
  await settle()
  assert f"#{act} exited 0" not in heads(engine.turns(on=root))
  engine.wake(root)
  await settle()
  assert said(log, "wake") == [("wake", root, OPERATOR, [f"#{root} woke"])]
  assert heads(engine.turns(on=root))[-1] == f"#{act} exited 0"


async def test_a_wake_is_over_the_act_it_names_and_everything_under_it() -> None:
  """A wake is over the act it names and everything under it."""
  sand, log, root = born(auto=False)
  act = engine.rung("out = await bash('slow')\nclose(out.code)", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(act)
  sand.exits(command, 0)
  await settle()
  assert f"#{command} exited 0" not in heads(engine.turns(on=root)) and engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert heads(engine.turns(on=root))[-2:] == [f"#{command} exited 0", f"#{act} closed 0"]
  assert await act == 0
