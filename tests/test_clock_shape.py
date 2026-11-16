"""Clock, the question of a reading of the wall clock."""

from conftest import WORLD, born, said
from furb import engine
from furb.engine import OPERATOR


async def test_a_clock_is_the_question_of_a_reading_of_the_wall_clock() -> None:
  """A clock is the question of a reading of the wall clock."""
  sand, log, root = born()
  assert engine.clock(on=root) == 1001.0
  word = [one for one in sand.calls if one[0] == "clock"][-1]
  assert (word[0], word[1], word[2], word[3]) == ("clock", "clock1", OPERATOR, root) and len(word) == 4
  answered = [one for one in said(log, "done") if one[1] == word[1]]
  assert [(one[2], one[3]) for one in answered] == [(WORLD, 1001.0)]
