"""wait, the act the World is done with when its seconds have passed."""

from conftest import STANDS, Sand, life, said, settle, tags
from furb import engine
from furb.engine import WORLD


async def test_a_wait_the_world_says_it_is_done_when_its_seconds_have_passed() -> None:
  """A wait: the World says it is done when its seconds have passed, and it is over then."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.wait(0.1, on=root)
  await settle()
  assert act not in engine.outcomes
  assert await act is None
  assert [a[2] for a in said(log, "done") if a[1] == act] == [WORLD]


async def test_a_wait_stands_in_no_turns() -> None:
  """A wait stands in no turns, since a wait is no work of a model."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  was = tags(engine.turns(on=root))
  act = engine.wait(0, on=root)
  assert await act is None
  assert tags(engine.turns(on=root)) == was
  assert [tag for tag in was if ("id", act) in tag[1]] == []


async def test_it_tells_nothing_and_answers_nothing() -> None:
  """It tells nothing and answers nothing, since a wait is no work of a model."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.wait(0, on=root)
  assert await act is None
  assert [tag for tag in tags(engine.turns(on=root)) if ("id", act) in tag[1]] == []
  assert engine.read(f"{act}/stdout", on=root) is None
  assert engine.peek(act, on=root) is None
