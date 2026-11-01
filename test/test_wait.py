"""wait, the act the World is done with when its seconds have passed."""

import pytest

from conftest import WORLD, born, paragraphs, said, settle
from furb import engine
from furb.engine import Refused


async def test_a_wait_the_world_takes_it_and_says_it_is_done_when_its_seconds_have_passed() -> None:
  """A wait: the World takes it and says it is done with nothing when its seconds have passed, and it is over then."""
  _, log, root = born()
  act = engine.wait(0.1, on=root)
  # The World takes the wait while it is made, so these hold before the loop runs, whatever the load of the machine.
  assert [a[2] for a in said(log, "started") if a[1] == act] == [WORLD]
  assert engine.peek(act, ...) is ...
  assert await act is None
  assert [a[2] for a in said(log, "done") if a[1] == act] == [WORLD]


async def test_a_wait_stands_in_no_turns() -> None:
  """A wait stands in no turns, since a wait is no work of a model."""
  _, _, root = born()
  was = engine.turns(on=root)
  act = engine.wait(0, on=root)
  assert await act is None
  assert engine.turns(on=root) == was
  assert [one for one in paragraphs(was) if act in one] == []


async def test_it_tells_nothing() -> None:
  """It tells nothing, since a wait is no work of a model."""
  _, log, root = born()
  act = engine.wait(0, on=root)
  assert await act is None
  assert [one for one in said(log, "tell") if one[1] == act] == []
  assert [one for one in paragraphs(engine.turns(on=root)) if act in one] == []
  with pytest.raises(Refused, match="nothing takes read"):
    engine.read(f"{act}/stdout", on=root)
  assert engine.peek(act) is None


async def test_wait_seconds_acts_is_done_at_the_first_of_two_events() -> None:
  """wait(seconds, *acts) is done at the first of two events: with the name of the first of its acts that is done, or, of those that are done already when the wait is made, the first in the order it is given them, or with None when its seconds pass first."""
  sand, log, root = born(auto=False)
  one, two = engine.bash("slow", on=root), engine.bash("slow", on=root)
  first = engine.wait(30.0, one, two, on=root)
  await settle()
  assert engine.peek(first, ...) is ...
  sand.exits(two, 0)
  assert await first == two
  assert await engine.wait(30.0, one, two, on=root) == two
  sand.exits(one, 0)
  await settle()
  assert await engine.wait(30.0, one, two, on=root) == one
  assert [a[2] for a in said(log, "started") if a[1] in (first, *[a[1] for a in said(log, "wait")][1:])] == [WORLD]
  late = engine.bash("slow", on=root)
  assert await engine.wait(0.01, late, on=root) is None
  assert await engine.wait(0, late, on=root) is None
  word = engine.rung("x = bash('slow')\nw: str | None = await wait(0.01, x)\nclose(w)", on=root)
  assert await word is None


async def test_the_ear_of_a_wait_says_it_done_when_one_of_its_acts_is_done() -> None:
  """The ear of a wait says it done when one of its acts is done, and the World stops its timer when it hears that done, as when a control ends it."""
  sand, log, root = born(auto=False)
  command = engine.bash("slow", on=root)
  act = engine.wait(30.0, command, on=root)
  await settle()
  assert act in sand.timers
  sand.exits(command, 0)
  assert await act == command
  await settle()
  assert [a[2] for a in said(log, "done") if a[1] == act] == [act]
  assert act not in sand.timers
