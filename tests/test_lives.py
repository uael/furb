"""lives, which carries a fact into a body and says what the body yields."""

from collections.abc import Generator

from conftest import Sand, born, keeping, life, settle
from furb import engine
from furb.engine import OPERATOR


def once() -> Generator[tuple | None, tuple | None]:
  """A body that says one fact of its own and is over."""
  yield "done", "none://one", None


def waiting(name: str) -> Generator[set[str], tuple | None]:
  """A body that waits for the facts about one name."""
  while True:
    yield {name}


async def test_lives_carries_a_fact_into_an_ear_and_says_everything_the_ear_yields() -> None:
  """lives carries a fact into an ear, says everything the ear yields, and gives what the ear waits for, or False once the ear is over."""
  _, log, root = born()
  heard: list[tuple] = []
  body = keeping(heard)
  assert engine.lives(body, None) is None and heard == []
  told = ("tell", root, OPERATOR, [("noted", [], None)])
  assert engine.lives(body, told) is None and heard == [told]
  assert engine.lives(waiting(root), None) == {root}
  mark = len(log)
  assert engine.lives(once(), None) is False
  assert log[mark:] == [("done", "none://one", OPERATOR, None)]


async def test_boot_defines_lives_again_at_each_life() -> None:
  """boot defines lives again at each life, so each ear of the outside hears a fact that the journal says again under the name journal."""
  sand, _, _ = born()
  first, heard = engine.lives, list[tuple]()
  life(Sand(), list(sand.record), keeper=keeping(heard))
  await settle()
  assert engine.lives is not first
  done = [a for a in heard if a[:2] == ("done", "stand1")]
  assert done == [("done", "stand1", "journal", *done[0][3:])] and sand.record[2][0][:3] == ("done", "stand1", "world")
