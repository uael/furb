"""control, the one way a pause, a wake, a cancel or a close is said over an act."""

from asyncio import CancelledError

from conftest import born, heads, said, settle, slow
from furb import engine
from furb.engine import OPERATOR


async def test_a_control_said_over_the_act_it_names_with_a_header_of_its_name() -> None:
  """A control said over the act it names, after a tell of a header of its name headed with that act when it comes from outside the chain of that act, which is how pause, wake, cancel and close say theirs."""
  _, log, _, act = slow()
  await settle()
  engine.control("pause", "paused", act)
  engine.wake(act)
  await settle()
  assert [one[1:] for one in said(log, "pause")] == [(act, OPERATOR)]
  assert [one[1:] for one in said(log, "wake")] == [(act, OPERATOR)]
  told = [(one[0], one[1], one[-1]) for one in log if one[0] in ("tell", "pause", "wake") and one[1] == act]
  assert told[-4:] == [
    ("tell", act, ["#bash1 paused by operator"]),
    ("pause", act, OPERATOR),
    ("tell", act, ["#bash1 woke by operator"]),
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
  assert said(log, "cancel")[-1] == ("cancel", second, OPERATOR, None)
  assert f"#{second} cancelled" not in heads(engine.turns(on=root))


async def test_the_header_of_a_control_names_the_one_that_said_it() -> None:
  """The header of a control names the one that said it, as #rung5 paused by operator, or #prompt4 closed by rung9 when rung9 stands on another chain, so a turn says who paused, woke, cancelled or closed the act."""
  _, log, root = born(auto=False)
  first = engine.bash("slow", on=root)
  await engine.rung("lane = chain('far')", on=root)
  lane = engine.module(root)["lane"]
  word = engine.rung(f"pause({first!r})\nwake({first!r})\ncancel({first!r})", on=lane)
  await word
  other = engine.bash("slow", on=root)
  engine.pause(other)
  engine.wake(other)
  engine.cancel(other)
  await settle()
  told = [a[3][0] for a in said(log, "tell") if a[1] in (first, other) and " by " in a[3][0]]
  assert told == [
    f"#{first} paused by {word}",
    f"#{first} woke by {word}",
    f"#{first} cancelled by {word}",
    f"#{other} paused by {OPERATOR}",
    f"#{other} woke by {OPERATOR}",
    f"#{other} cancelled by {OPERATOR}",
  ]


async def test_a_control_from_inside_the_chain_of_the_act_it_names_tells_nothing() -> None:
  """A control from inside the chain of the act it names, as one that a word or an ear of that chain says, tells nothing, since the chain shows already what its words and its ears did."""
  _, log, root = born("close(7)", auto=False)
  first = engine.bash("slow", on=root)
  word = engine.rung(f"pause({first!r})\nwake({first!r})\ncancel({first!r})", on=root)
  await word
  act = engine.prompt(int, "count", on=root)
  assert await act == 7
  await settle()
  controls = [(a[0], a[1], a[2]) for a in log if a[0] in ("pause", "wake", "cancel") and a[1] == first]
  assert controls == [("pause", first, word), ("wake", first, word), ("cancel", first, word)]
  assert [a[1] for a in said(log, "close")] == [act]
  assert [a for a in said(log, "tell") if a[1] in (first, act) and " by " in a[3][0]] == []
