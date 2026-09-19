"""lives, which carries a fact into a body and says what the body yields."""

from collections.abc import Generator

import pytest

from conftest import STANDS, Sand, keeping, life
from furb import engine
from furb.engine import OPERATOR


def once() -> Generator[tuple | None, tuple | None]:
  """A body that says one fact of its own and is over."""
  yield "done", "none://one", None


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_lives_carries_a_fact_into_a_body_and_says_everything_the_body_yields() -> None:
  """lives carries a fact into a body, says everything the body yields, and gives whether the body lives on."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  body = keeping(heard)
  assert engine.lives(body, None) is True and heard == []
  told = ("tell", root, OPERATOR, [("noted", [], None)])
  assert engine.lives(body, told) is True and heard == [told]
  mark = len(log)
  assert engine.lives(once(), None) is False
  assert log[mark:] == [("done", "none://one", OPERATOR, None)]
