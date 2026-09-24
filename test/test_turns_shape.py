"""Turns, the question of the turns of a chain."""

from conftest import STANDS, Sand, heads, life, rows, said
from furb import engine
from furb.engine import OPERATOR


async def test_a_turns_is_the_question_of_the_turns_of_a_chain() -> None:
  """A turns is the question of the turns of a chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert said(held, "turns") == [("turns", "turns@operator.3", OPERATOR, root)]
  assert engine.outcomes["turns@operator.3"] == got
  assert [role for role, *_ in got] == ["user"]
  assert heads(got) == ["#chain1 root", rows("chain1")[0], "#rung1"]
