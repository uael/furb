"""lives, which carries a fact into a body and says what the body yields."""

from collections.abc import Generator

from conftest import born, keeping
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
