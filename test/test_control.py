"""control, the one way a pause, a wake, a cancel or a close is said over an act."""

from asyncio import CancelledError

from conftest import born, heads, said, settle, slow
from furb import engine
from furb.engine import OPERATOR


async def test_a_control_said_over_the_act_it_names_with_a_header_of_its_name() -> None:
  """A control said over the act it names, after a tell of a header of its name headed with that act, which is how pause, wake, cancel and close say theirs."""
  _, log, _, act = slow()
  await settle()
  engine.control("pause", "paused", act)
  engine.wake(act)
  await settle()
  assert [one[1:] for one in said(log, "pause")] == [(act, OPERATOR)]
  assert [one[1:] for one in said(log, "wake")] == [(act, OPERATOR)]
  told = [(one[0], one[1], one[-1]) for one in log if one[0] in ("tell", "pause", "wake") and one[1] == act]
  assert told[-4:] == [
    ("tell", act, ["#bash1 paused"]),
    ("pause", act, OPERATOR),
    ("tell", act, ["#bash1 woke"]),
    ("wake", act, OPERATOR),
  ]


async def test_the_call_gives_the_control_as_the_life_made_it_whole() -> None:
  """The call gives the control as the life made it whole, so a close that named no act reads which act it is over."""
  _, log, root = born("b = bash('slow')\nk = control('pause', 'paused', b)\nclose(3)", auto=False)
  act = engine.prompt(int, "count", on=root)
  assert await act == 3
  await settle()
  step = said(log, "rung")[0][1]
  assert engine.module(root)["k"] == ("pause", "bash1", step)
  assert [(one[1], one[3]) for one in said(log, "close")] == [(act, 3)]


async def test_a_control_is_said_over_any_act_and_tells_its_header_only_while_the_act_it_names_is_not_over() -> None:
  """A control is said over any act of the life, and it tells its header only while the act it names is not over, so a control over an act that is over tells nothing and still reaches each act under it that is not over."""
  sand, log, root = born("x = bash('slow')\nclose(7)", auto=False)
  act = engine.prompt(int, "go", on=root)
  assert await act == 7
  command = said(log, "bash")[0][1]
  sand.exits(command, 0)
  await settle()
  told = len(said(log, "tell"))
  engine.close(1, act)
  engine.cancel(command)
  await settle()
  assert [(a[0], a[1]) for a in log if a[0] in ("pause", "wake", "cancel", "close")][-2:] == [
    ("close", act),
    ("cancel", command),
  ]
  assert said(log, "tell")[told:] == [] and engine.peek(act) == 7
  first = engine.rung("lane = chain('far')", on=root)
  await first
  second = engine.rung("p = prompt(None, 'work', on=lane)", on=root)
  await second
  (far,) = [a[1] for a in said(log, "prompt") if engine.under(a[1], second)]
  assert engine.peek(far, ...) is ...
  engine.cancel(second)
  await settle()
  assert isinstance(engine.peek(far), CancelledError)
  assert said(log, "cancel")[-1] == ("cancel", second, OPERATOR)
  assert f"#{second} cancelled" not in heads(engine.turns(on=root))
