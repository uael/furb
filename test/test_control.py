"""control, the one way a pause, a wake, a cancel or a close is said over an act."""

from asyncio import CancelledError

from conftest import born, heads, said, settle, slow
from furb import engine
from furb.engine import OPERATOR


async def test_a_control_said_over_the_act_it_names_with_a_header_of_its_name() -> None:
  """A control said over the act it names, with a header of its name headed with that act, which is how pause, wake, cancel and close say theirs."""
  _, log, _, act = slow()
  await settle()
  engine.control("pause", "paused", act)
  engine.wake(act)
  await settle()
  assert [(one[1], one[3]) for one in said(log, "pause")] == [(act, ["#bash1 paused"])]
  assert [(one[1], one[3]) for one in said(log, "wake")] == [(act, ["#bash1 woke"])]


async def test_the_call_gives_the_control_as_the_life_made_it_whole() -> None:
  """The call gives the control as the life made it whole, so a close that named no act reads which act it is over."""
  _, log, root = born("b = bash('slow')\nk = control('pause', 'paused', b)\nclose(3)", auto=False)
  act = engine.prompt(int, "count", on=root)
  assert await act == 3
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.module(root)["k"] == ("pause", "bash1", step, ["#bash1 paused"])
  assert [(one[1], one[3]) for one in said(log, "close")] == [(act, 3)]


async def test_a_control_is_said_while_it_is_over_an_act_that_is_not_done() -> None:
  """A control is said while the act it names is not done, a wake also while that act is paused, and a control over an act that is done also while an act that it covers is not done, which it finds after that act on its chain and on each chain that it meets there, so a control that reaches nothing says nothing."""
  sand, log, root = born("x = bash('slow')\nclose(7)", auto=False)
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
  step = engine.rung("y = bash('slow')", on=root)
  await step
  running = said(log, "bash")[-1][1]
  engine.cancel(step)
  await settle()
  assert isinstance(engine.peek(running), CancelledError)
  assert said(log, "cancel")[-1] == ("cancel", step, OPERATOR, [f"#{step} cancelled"])
  idle = engine.rung("chain('idle')", on=root)
  await idle
  engine.cancel(idle)
  await settle()
  assert [a for a in said(log, "cancel") if a[1] == idle] == []
  lane = engine.rung("prompt(None, 'work', on=chain('lane'))", on=root)
  await lane
  (far,) = [a[1] for a in said(log, "prompt") if engine.under(a[1], lane)]
  assert engine.peek(far, ...) is ...
  engine.cancel(lane)
  await settle()
  assert isinstance(engine.peek(far), CancelledError)
  assert said(log, "cancel")[-1] == ("cancel", lane, OPERATOR, [f"#{lane} cancelled"])
