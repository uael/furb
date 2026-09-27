"""lives, which carries a fact into a body and says what the body yields."""

from collections.abc import Generator

from conftest import born, keeping
from furb import engine
from furb.engine import OPERATOR


def once() -> Generator[tuple | None, tuple | None]:
  """A body that says one fact of its own and is over."""
  yield "done", "none://one", None


async def test_lives_carries_a_fact_into_an_ear_and_says_everything_the_ear_yields() -> None:
  """lives carries a fact into an ear, says everything the ear yields, and gives whether the ear lives on."""
  _, log, root = born()
  heard: list[tuple] = []
  body = keeping(heard)
  assert engine.lives(body, None) is True and heard == []
  told = ("tell", root, OPERATOR, [("noted", [], None)])
  assert engine.lives(body, told) is True and heard == [told]
  mark = len(log)
  assert engine.lives(once(), None) is False
  assert log[mark:] == [("done", "none://one", OPERATOR, None)]
