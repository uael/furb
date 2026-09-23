"""clock, one reading of the wall clock of the World."""

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR


async def test_clock_gives_one_reading_of_the_wall_clock_of_the_world() -> None:
  """clock gives one reading of the wall clock of the World."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.clock(on=root) == 1001.0
  assert engine.clock(on=root) == 1002.0
  assert [one[0] for one in sand.calls if one[0] == "clock"] == ["clock", "clock"]


async def test_the_driver_sends_clock_to_the_world_for_a_reading_of_the_wall_clock() -> None:
  """The driver sends clock to the World for a reading of the wall clock."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.clock(on=root) == 1001.0
  word = next(one for one in sand.calls if one[0] == "clock")
  assert (word[0], word[2], word[3]) == ("clock", OPERATOR, root)


async def test_clock_tells_the_reading_it_was_answered() -> None:
  """clock tells the reading it was answered, since the word that asked holds it and the turns after it would not."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["at = clock()\nclose(1)"]
  assert await engine.prompt(int, "read the clock", on=root) == 1
  await settle()
  assert engine.modules[root]["at"] == 1001.0
  assert engine.turns(on=root)[-1][1] == "#clock 1001.0\n\n#prompt1 closed 1"
