"""wait, the act the World is done with when its seconds have passed."""

import pytest

from conftest import WORLD, born, paragraphs, said, settle
from furb import engine
from furb.engine import Refused


async def test_a_wait_the_world_takes_it_and_says_it_is_done_when_its_seconds_have_passed() -> None:
  """A wait: the World takes it and says it is done when its seconds have passed, and it is over then."""
  _, log, root = born()
  act = engine.wait(0.1, on=root)
  await settle()
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


async def test_it_tells_nothing_and_answers_nothing() -> None:
  """It tells nothing and answers nothing, since a wait is no work of a model."""
  _, log, root = born()
  act = engine.wait(0, on=root)
  assert await act is None
  assert [one for one in said(log, "tell") if one[1] == act] == []
  assert [one for one in paragraphs(engine.turns(on=root)) if act in one] == []
  with pytest.raises(Refused, match="nothing takes read"):
    engine.read(f"{act}/stdout", on=root)
  assert engine.peek(act) is None
