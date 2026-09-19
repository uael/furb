"""Chance, the question the World draws a number for."""

import pytest

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import OPERATOR, WORLD


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_chance_is_the_question_of_a_number_the_world_draws() -> None:
  """A chance is the question of a number the World draws."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert engine.chance(on=root) == 1 / 7
  word = next(one for one in sand.calls if one[0] == "chance")
  assert (word[0], word[2], word[3]) == ("chance", OPERATOR, root) and len(word) == 4
  answered = [one for one in said(log, "done") if one[1] == word[1]]
  assert [(one[2], one[3]) for one in answered] == [(WORLD, 1 / 7)]
