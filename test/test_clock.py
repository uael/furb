"""clock, one reading of the wall clock of the World."""

from conftest import STANDS, Sand, life
from furb import engine
from furb.engine import OPERATOR


async def test_clock_gives_one_reading_of_the_wall_clock_of_the_world() -> None:
  """clock gives one reading of the wall clock of the World."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.clock(on=root) == 1001.0
  assert engine.clock(on=root) == 1002.0
  assert [one[0] for one in sand.calls if one[0] == "clock"] == ["clock", "clock"]


async def test_clock_asks_the_world_for_a_reading_of_the_wall_clock() -> None:
  """clock asks the World for a reading of the wall clock, and the record keeps what it answered."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.clock(on=root) == 1001.0
  word = next(one for one in sand.calls if one[0] == "clock")
  assert (word[0], word[2], word[3]) == ("clock", OPERATOR, root)
  assert ((word,), (("done", word[1], "world", 1001.0),)) == tuple(e for e in sand.record if e[0][1] == word[1])
