"""Act, the name of an act."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR, Act, Text


async def test_the_name_of_an_act() -> None:
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  assert isinstance(one, Act) and isinstance(one, str) and one.startswith("prompt://")
  await settle()
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


async def test_an_act_is_over_when_its_done_stands_and_lives_until_then() -> None:
  """An act is over when its done stands and lives until then; there is no other state, and a control over an act that is over reaches nothing."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  await settle()
  assert engine.peek(one) is None
  engine.close(21, one)
  await settle()
  assert (await one) == 21
  engine.close(99, one)
  engine.cancel(one)
  await settle()
  assert engine.peek(one) == 21 and (await one) == 21


async def test_the_outcome_of_a_cancelled_act_is_the_cancellederror_it_completed_with() -> None:
  """The outcome of a cancelled act is the CancelledError it completed with."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  one = engine.bash("slow", on=root)
  await settle()
  assert engine.peek(one) == engine.Exit(None, Text(f"{one}/stdout"), Text(f"{one}/stderr"))
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  with pytest.raises(CancelledError):
    await one


async def test_only_a_command_lives_past_its_done_and_every_other_act_is_dropped_at_its_done() -> None:
  """Only a command lives past its done, and every other act is dropped at its done."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  await settle()
  assert engine.write(Text(one, "k = 1"), on=root) is not None
  engine.close(None, one)
  await settle()
  assert engine.write(Text(one, "k = 2"), on=root) is None
  two = engine.bash("echo hi", on=root)
  assert (await two).code == 0
  assert engine.read(f"{two}/stdout", on=root).content == "ran echo hi\n"
