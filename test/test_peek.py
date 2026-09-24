"""peek, what the act it is at came to."""

from asyncio import CancelledError
from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD, Refused


def halfway(id: str) -> Generator[tuple | None, tuple]:
  """The ear of an act of an extension that answers a peek at itself with what it holds so far."""
  while True:
    match (yield):
      case ("peek", qid, _, _, at) if at == id:
        yield "done", qid, "half"


async def test_what_the_act_it_is_at_came_to_as_the_record_stands_where_the_call_is_made() -> None:
  """What the act it is at came to, as the record stands where the call is made, which it gives and never raises."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.act("note", root, engine.ending(halfway))
  assert engine.peek(act, on=root) == "half"
  engine.send("done", act, "whole", by=WORLD)
  assert await act == engine.peek(act, on=root) == "whole"


async def test_the_operator_reads_the_results_of_its_own_acts_in_the_transcript() -> None:
  """The operator reads the results of its own acts in the transcript."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(0, on=root)
  await act
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  ends = [one for one in held if one[0] == "done" and one[1] == act]
  assert [one[3] for one in ends] == [engine.peek(act, on=root)]


async def test_peek_gives_the_value_or_the_exception_itself() -> None:
  """peek gives the value, or the exception itself."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  good = engine.rung("close(21)", on=root)
  await good
  assert engine.peek(good, on=root) == 21
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  hurt = said(log, "rung")[1][1]
  got = engine.peek(hurt, on=root)
  assert isinstance(got, ValueError) and str(got) == "boom"


async def test_peek_never_raises() -> None:
  """peek never raises."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  gone = engine.wait(100, on=root)
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.peek(gone, on=root), CancelledError)
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.outcomes[ghost], Refused)
  assert isinstance(engine.peek(said(log, "prompt")[0][1], on=root), Refused)


async def test_the_done_of_that_act_filled_its_outcome_and_the_life_holds_every_outcome_by_name() -> None:
  """The done of that act filled its outcome, and the life holds every outcome by name, so the life answers a peek at an act that is over, though its generator is gone; an act that is not over answers for itself, as its ear likes."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.act("note", root, engine.ending(halfway))
  assert engine.peek(act, on=root) == "half"
  step = engine.rung("close(21)", on=root)
  await step
  assert engine.outcomes[step] == 21 == engine.peek(step, on=root)
  engine.send("done", act, "whole", by=WORLD)
  await settle()
  assert engine.peek(act, on=root) == engine.outcomes[act] == "whole"


async def test_a_peek_at_a_chain_gives_none_since_a_chain_never_comes_to_anything() -> None:
  """A peek at a chain gives None, since a chain never comes to anything."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  await settle()
  assert engine.peek(root, on=root) is None
  assert engine.peek(two, on=two) is None


async def test_a_peek_at_a_name_of_no_question_gives_none() -> None:
  """A peek at a name of no question gives None."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.peek("wait://nobody", on=root) is None
  assert engine.peek("", on=root) is None
