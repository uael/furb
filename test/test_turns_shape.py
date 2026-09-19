"""Turns, the question of the turns of a chain."""

from conftest import STANDS, Sand, life, said, tags
from furb import engine


async def test_a_turns_is_the_question_of_the_turns_of_a_chain() -> None:
  """A turns is the question of the turns of a chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  asked = said(held, "turns")[0]
  assert (asked[1], asked[3]) == ("turns://operator.3", root)
  assert engine.outcomes[asked[1]] == got
  assert [role for role, *_ in got] == ["user"]
  assert [name for name, *_ in tags(got)] == ["opened", "opened", "opened", "closed"]
