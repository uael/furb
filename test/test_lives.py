"""lives, which carries a fact into an ear and gives whether it lives on."""

from collections.abc import Generator

from conftest import STANDS, Sand, keeping, life
from furb import engine
from furb.engine import OPERATOR


def once() -> Generator[None, tuple | None]:
  """An inner ear that says one fact of its own at its birth and is over."""
  engine.say("done", "none://one", None)
  yield


def wrapping(heard: list[tuple]) -> Generator[None, tuple | None]:
  """An ear made of two inner ears, which gives each what it hears for as long as it lives."""
  inner, a = [keeping(heard), once()], None
  while True:
    inner = [g for g in inner if engine.lives(g, a)]
    a = yield


async def test_lives_carries_a_fact_into_an_ear_and_gives_whether_the_ear_lives_on() -> None:
  """lives carries a fact into an ear and gives whether the ear lives on, which is how an ear made of an ear gives its inner ear what it hears."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  heard: list[tuple] = []
  body = keeping(heard)
  assert engine.lives(body, None) is True and heard == []
  told = ("tell", root, OPERATOR, [("noted", [], None)])
  assert engine.lives(body, told) is True and heard == [told]
  over = once()
  assert engine.lives(over, None) is True and engine.lives(over, told) is False
  mark = len(log)
  inside: list[tuple] = []
  engine.drive(wrapping(inside), "wrapper")
  assert log[mark:] == [("done", "none://one", "wrapper", None)] == inside
