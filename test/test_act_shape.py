"""Act, the name of an act."""

from asyncio import CancelledError

import pytest

from conftest import paragraphs, posed, said, settle, slow
from furb import engine
from furb.engine import OPERATOR, Act, Refused


async def test_the_name_of_an_act() -> None:
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to."""
  _, log, root, one = await posed(int, "hi")
  assert isinstance(one, Act) and isinstance(one, str) and one == "prompt1"
  assert engine.get(one)[:4] == ("prompt", one, OPERATOR, root)
  engine.pause(one)
  engine.wake(one)
  engine.close(21, one)
  await settle()
  assert engine.peek(one) == 21 and (await one) == 21
  two = engine.bash("echo hi", on=root)
  engine.cancel(two)
  await settle()
  assert isinstance(engine.peek(two), CancelledError)
  controls = [(a[0], a[1]) for a in log if a[0] in ("pause", "wake", "cancel", "close")]
  assert controls == [("pause", one), ("wake", one), ("close", one), ("cancel", two)]


async def test_an_act_is_over_when_its_done_stands_and_lives_until_then() -> None:
  """An act is over when its done stands and lives until then; there is no other state, and a control over an act that is over tells nothing and reaches only the acts that it covers and that are not over, as control says, but a wake also reaches the ear of a command that a pause holds."""
  sand, log, root, one = await posed(int, "hi")
  assert engine.peek(one) is None
  engine.close(21, one)
  await settle()
  assert (await one) == 21
  was = engine.turns(on=root)
  engine.close(99, one)
  engine.cancel(one)
  engine.pause(one)
  engine.wake(one)
  await settle()
  assert engine.peek(one) == 21 and (await one) == 21
  controls = [(a[0], a[1]) for a in log if a[0] in ("pause", "wake", "cancel", "close")]
  assert controls == [("close", one), ("close", one), ("cancel", one), ("pause", one), ("wake", one)]
  assert engine.turns(on=root) == was
  sand.auto = False
  step = engine.rung("x = bash('slow')", on=root)
  await step
  command = said(log, "bash")[-1][1]
  engine.cancel(step)
  await settle()
  assert isinstance(engine.peek(command), CancelledError)
  assert [(a[0], a[1]) for a in log if a[0] == "cancel"] == [("cancel", one), ("cancel", step)]


async def test_the_outcome_of_a_cancelled_act_is_the_cancellederror_it_completed_with() -> None:
  """The outcome of a cancelled act is the CancelledError it completed with."""
  _, _, _, one = slow()
  await settle()
  assert engine.peek(one) is None
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  with pytest.raises(CancelledError):
    await one


async def test_an_act_is_dropped_at_its_done_and_only_the_ear_of_a_command_lives_on_until_it_hears_that_done() -> None:
  """An act is dropped at its done, and so is its ear, but a lasting ear, which lives on until it returns, as the ear of a command does once it heard that done and told the exit."""
  _, _, root, one = await posed(int, "hi")
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(one), one)
  engine.close(1, one)
  await settle()
  engine.drive(engine.idle(one), one)
  two = engine.bash("echo hi", on=root)
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(two), two)
  assert (await two).code == 0
  await settle()
  engine.drive(engine.idle(two), two)
  assert f"#{two} exited 0\n{two}_stdout = 'ran echo hi'" in paragraphs(engine.turns(on=root))
