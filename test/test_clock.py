"""clock, one reading of the wall clock of the World."""

from conftest import born
from furb import engine
from furb.engine import OPERATOR


async def test_clock_gives_one_reading_of_the_wall_clock_of_the_world() -> None:
  """clock gives one reading of the wall clock of the World."""
  sand, _, root = born()
  assert engine.clock(on=root) == 1002.0
  assert engine.clock(on=root) == 1003.0
  assert [one[0] for one in sand.calls if one[0] == "clock"] == ["clock", "clock", "clock"]


async def test_clock_asks_the_world_for_a_reading_of_the_wall_clock() -> None:
  """clock asks the World for a reading of the wall clock, and the journal keeps what it answered."""
  sand, _, root = born()
  assert engine.clock(on=root) == 1002.0
  word = [one for one in sand.calls if one[0] == "clock"][-1]
  assert (word[0], word[1], word[2], word[3]) == ("clock", "clock2", OPERATOR, root)
  assert ((word,), (("done", word[1], "world", 1002.0),)) == tuple(e for e in sand.record if e[0][1] == word[1])
