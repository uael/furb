"""Act, the name of an act."""

from asyncio import CancelledError
from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR, Act, Refused


def lasting(id: str) -> Generator[tuple | None, tuple]:
  """The ear of an act of an extension that is done with one at its birth and lives on to answer a read of a door of
  its own."""
  yield "done", id, 1
  while True:
    match (yield):
      case ("read", qid, _, _, path) if path == f"{id}/kept":
        yield "done", qid, "kept"


async def test_the_name_of_an_act() -> None:
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  assert isinstance(one, Act) and isinstance(one, str) and one == "prompt1"
  await settle()
  assert engine.get(one)[:4] == ("prompt", one, OPERATOR, root)
  engine.pause(one)
  engine.wake(one)
  engine.close(21, one)
  await settle()
  assert engine.peek(one) == 21 and (await one) == 21
  two = engine.wait(100, on=root)
  engine.cancel(two)
  await settle()
  assert isinstance(engine.peek(two), CancelledError)


async def test_an_act_is_over_when_its_done_stands_and_lives_until_then() -> None:
  """An act is over when its done stands and lives until then; there is no other state, and a control over an act that is over reaches nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  await settle()
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
  assert [(a[0], a[1]) for a in log if a[0] in ("pause", "wake", "cancel", "close")] == [("close", one)]
  assert engine.turns(on=root) == was


async def test_the_outcome_of_a_cancelled_act_is_the_cancellederror_it_completed_with() -> None:
  """The outcome of a cancelled act is the CancelledError it completed with."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.wait(100, on=root)
  await settle()
  assert one not in engine.outcomes
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(one), CancelledError)
  with pytest.raises(CancelledError):
    await one


async def test_an_act_of_the_file_is_dropped_at_its_done_and_an_act_of_an_extension_lives_past_it() -> None:
  """An act of the file is dropped at its done, and an act of an extension lives past its done while its ear lives."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(None, "hi", to=OPERATOR, on=root)
  await settle()
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(one), one)
  engine.close(None, one)
  await settle()
  engine.drive(engine.idle(one), one)
  two = engine.act("note", root, lasting)
  assert (await two) == 1
  with pytest.raises(Refused, match="hears"):
    engine.drive(engine.idle(two), two)
  assert engine.ask("read", root, f"{two}/kept")[1] == "kept"
