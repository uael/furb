"""Peek, the question of what an act came to."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_peek_is_at_an_act_and_gives_what_the_act_came_to_as_the_record_stands() -> None:
  """A peek is at an act, and gives what the act came to as the record stands."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, one)
  got = await one
  word, answer = engine.ask("peek", root, one)
  assert word == ("peek", "peek@operator.3", OPERATOR, root, one) and answer == got == 21
  sand.script[root] = [f"seen = peek({one!r})\nclose(seen)"]
  assert await engine.prompt(int, "look", on=root) == 21
  await settle()
  assert engine.modules[root]["seen"] == got
