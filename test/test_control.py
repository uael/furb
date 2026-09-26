"""control, the one way a pause, a wake, a cancel or a close is said over an act."""

from conftest import STANDS, Sand, heads, life, said, settle
from furb import engine


async def test_a_control_said_over_the_act_it_names_with_a_header_of_its_name() -> None:
  """A control said over the act it names, with a header of its name headed with that act, which is how pause, wake, cancel and close say theirs."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  engine.control("pause", "paused", act)
  engine.wake(act)
  await settle()
  assert [(one[1], one[3]) for one in said(log, "pause")] == [(act, ["#bash1 paused"])]
  assert [(one[1], one[3]) for one in said(log, "wake")] == [(act, ["#bash1 woke"])]


async def test_the_call_gives_the_control_as_the_life_made_it_whole() -> None:
  """The call gives the control as the life made it whole, so a close that named no act reads which act it is over."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["b = bash('slow')\nk = control('pause', 'paused', b)\nclose(3)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 3
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.module(root)["k"] == ("pause", "bash1", step, ["#bash1 paused"])
  assert [(one[1], one[3]) for one in said(log, "close")] == [(act, 3)]


async def test_a_control_is_said_while_it_is_over_an_act_that_is_not_done() -> None:
  """A control is said while the act it names is not done, and a wake while it is paused too, so a control that reaches nothing says nothing."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(7)"]
  act = engine.prompt(int, "go", on=root)
  assert await act == 7
  command = said(log, "bash")[0][1]
  engine.close(1, act)
  engine.pause(command)
  sand.exits(command, 0)
  await settle()
  engine.cancel(command)
  engine.wake(command)
  engine.cancel(act)
  engine.pause("nothing9")
  engine.close(1, "bogus9")
  await settle()
  controls = [(a[0], a[1]) for a in log if a[0] in ("pause", "wake", "cancel", "close")]
  assert controls == [("close", act), ("pause", command), ("wake", command)]
  assert heads(engine.turns(on=root))[-1] == f"#{command} exited 0"
