"""chance, a number the World draws."""

import pytest

from conftest import STANDS, Dead, Sand, attr, life, relived, settle, tags
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_chance_gives_a_number_that_is_at_least_zero_and_under_one() -> None:
  """chance gives a number that is at least zero and under one."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  drawn = [engine.chance(on=root) for _ in range(7)]
  assert drawn == [1 / 7, 2 / 7, 3 / 7, 4 / 7, 5 / 7, 6 / 7, 0.0]
  assert all(0 <= one < 1 for one in drawn)


async def test_a_number_the_world_draws_at_least_zero_and_under_one() -> None:
  """A number the World draws, at least zero and under one."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.chance(on=root) == 1 / 7
  assert [(one[0], one[2], one[3]) for one in sand.calls if one[0] == "chance"] == [("chance", OPERATOR, root)]
  _, over = life(Dead(stands=STANDS))
  with pytest.raises(Refused, match="no chance"):
    engine.chance(on=over)


async def test_chance_is_a_question_the_world_answers_and_it_enters_the_record() -> None:
  """chance is a question the World answers, and it enters the record as any question of a run does."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(chance())"]
  assert await engine.prompt(float, "draw", on=root) == 1 / 7
  await settle()
  drawn = [e for e in sand.record if e[1][0] == "chance"]
  assert [(e[1][3], e[2]) for e in drawn] == [(root, 1 / 7)]
  later = Sand(stands=STANDS)
  _, over = await relived(later, list(sand.record))
  assert over == root and [one for one in later.calls if one[0] == "chance"] == []


async def test_chance_tells_the_number_it_drew() -> None:
  """chance tells the number it drew."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["chance()\nclose(1)"]
  assert await engine.prompt(int, "draw", on=root) == 1
  await settle()
  assert [attr(tag, "drew") for tag in tags(engine.turns(on=root), "chance")] == [1 / 7]
