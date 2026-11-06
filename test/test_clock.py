"""clock, one reading of the wall clock of the World."""

from conftest import born
from furb import engine
from furb.engine import OPERATOR


async def test_clock_gives_one_reading_of_the_wall_clock_of_the_world() -> None:
  """clock gives one reading of the wall clock of the World."""
  sand, _, root = born()
  assert engine.clock(on=root) == 1001.0
  assert engine.clock(on=root) == 1002.0
  assert [one[0] for one in sand.calls if one[0] == "clock"] == ["clock", "clock"]


async def test_clock_asks_the_world_for_a_reading_of_the_wall_clock() -> None:
  """clock asks the World for a reading of the wall clock, and the journal keeps what it answered."""
  sand, _, root = born()
  assert engine.clock(on=root) == 1001.0
  word = [one for one in sand.calls if one[0] == "clock"][-1]
  assert (word[0], word[1], word[2], word[3]) == ("clock", "clock1", OPERATOR, root)
  assert ((word,), (("done", word[1], "world", 1001.0),)) == tuple(e for e in sand.record if e[0][1] == word[1])


async def test_clock_refuses_a_reading_to_a_word_when_the_last_fact_of_its_chain_is_of_a_reading_since_a_word_that_reads_the_clock_again_with_nothing_between_polls_it_and_each_reading_is_a_fact_of_the_record() -> (
  None
):
  """clock refuses a reading to a word when the last fact of its chain is of a reading, since a word that reads the clock again with nothing between polls it, and each reading is a fact of the record."""
  _, log, root = born(
    "t = clock()\ntry:\n  while clock() < t + 60:\n    pass\nexcept Refused as no:\n  close(str(no))",
    "t = clock()\nx = bash('true')\nclose(clock() >= t)",
  )
  assert "a poll of the clock" in await engine.prompt(str, "poll", on=root)
  assert len([a for a in log if a[0] == "clock"]) == 1
  assert await engine.prompt(bool, "read twice", on=root) is True
