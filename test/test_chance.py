"""chance, a number the World draws."""

import pytest

from conftest import Dead, Sand, born, life, relived, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_chance_gives_a_number_that_is_at_least_zero_and_under_one() -> None:
  """chance gives a number that is at least zero and under one."""
  _, _, root = born()
  drawn = [engine.chance(on=root) for _ in range(7)]
  assert drawn == [1 / 7, 2 / 7, 3 / 7, 4 / 7, 5 / 7, 6 / 7, 0.0]
  assert all(0 <= one < 1 for one in drawn)


async def test_a_number_the_world_draws_at_least_zero_and_under_one() -> None:
  """A number the World draws, at least zero and under one."""
  sand, _, root = born()
  assert engine.chance(on=root) == 1 / 7
  assert [(one[0], one[2], one[3]) for one in sand.calls if one[0] == "chance"] == [("chance", OPERATOR, root)]
  _, over = life(Dead())
  with pytest.raises(Refused, match="no chance"):
    engine.chance(on=over)


async def test_chance_is_a_question_the_world_answers_and_the_journal_keeps_what_it_answered() -> None:
  """chance is a question the World answers, and the journal keeps what it answered, as it keeps every answer of the World."""
  sand, _, root = born("close(chance())")
  assert await engine.prompt(float, "draw", on=root) == 1 / 7
  await settle()
  drawn = [e[0] for e in sand.record if e[0][0] == "chance"]
  assert [a[3] for a in drawn] == [root] and (("done", drawn[0][1], "world", 1 / 7),) in sand.record
  later = Sand()
  _, over = await relived(later, list(sand.record))
  assert over == root and [one for one in later.calls if one[0] == "chance"] == []
