"""peek, what the act it is at came to."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Sand, life, relived, said, settle, world_says
from furb import engine
from furb.engine import Exit, Refused, Text


async def test_what_the_act_it_is_at_came_to_as_the_record_stands_where_the_call_is_made() -> None:
  """What the act it is at came to, as the record stands where the call is made, which it gives and never raises."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("run", on=root)
  world_says("out", act, "half\n", "stdout")
  await settle()
  assert engine.peek(act) is None and engine.read(f"{act}/stdout", on=root).content == "half\n"
  sand.exits(act, 0)
  assert await act == engine.peek(act)


async def test_the_operator_reads_the_results_of_its_own_acts_in_the_transcript() -> None:
  """The operator reads the results of its own acts in the transcript."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.bash("echo hi", on=root)
  await act
  held = engine.transcript(root)
  ends = [one for one in held if one[0] == "done" and one[1] == act]
  assert [one[3] for one in ends] == [engine.peek(act)]


async def test_peek_gives_the_value_or_the_exception_itself() -> None:
  """peek gives the value, or the exception itself."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  good = engine.rung("close(21)", on=root)
  await good
  assert engine.peek(good) == 21
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  hurt = said(log, "rung")[1][1]
  got = engine.peek(hurt)
  assert isinstance(got, ValueError) and str(got) == "boom"


async def test_peek_never_raises() -> None:
  """peek never raises."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  gone = engine.bash("slow", on=root)
  engine.cancel(gone)
  await settle()
  assert isinstance(engine.peek(gone), CancelledError)
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
  assert isinstance(engine.peek(said(log, "prompt")[0][1]), Refused)


async def test_the_done_of_that_act_filled_its_outcome_and_the_life_holds_every_outcome_by_name() -> None:
  """The done of that act filled its outcome, and the life holds every outcome by name, so a peek reads an act that is over, though its generator is gone, and gives what it was given to wait with for an act that is not over."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("run", on=root)
  world_says("out", act, "half\n", "stdout")
  await settle()
  assert engine.peek(act, "waiting") == "waiting" and engine.peek(act) is None
  step = engine.rung("close(21)", on=root)
  await step
  assert engine.peek(step) == 21
  sand.exits(act, 0)
  await settle()
  assert engine.peek(act) == Exit(0, Text(f"{act}/stdout", "half\n"), Text(f"{act}/stderr"))


async def test_a_peek_at_a_chain_gives_none_since_a_chain_never_comes_to_anything() -> None:
  """A peek at a chain gives None, since a chain never comes to anything."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  await settle()
  assert engine.peek(root) is None
  assert engine.peek(two) is None


async def test_a_peek_at_a_name_of_no_question_gives_none() -> None:
  """A peek at a name of no question gives None."""
  sand = Sand(stands=STANDS)
  life(sand)
  assert engine.peek("bash://nobody") is None
  assert engine.peek("") is None


async def test_the_outcome_is_no_slot_of_the_act() -> None:
  """The outcome is no slot of the act, so the plain form of an act holds none of it, and an act made again from the record waits until its done is said again."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  assert engine.peek(act, ...) is ... and len(said(log, "bash")[0]) == 7
  kept = list(sand.record)
  assert [len(e[0]) for e in kept if e[0][0] == "bash"] == [7]
  later = Sand(stands=STANDS, auto=False)
  _, over = await relived(later, kept)
  assert over == root and engine.get(act) is not None and engine.peek(act, ...) is ...
