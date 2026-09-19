"""Feed, what a write to the stdin door of a command hands the World."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import Text


async def test_a_write_to_the_stdin_door_of_a_fed_command_hands_the_world_a_feed_with_the_text() -> None:
  """A write to the stdin door of a fed command hands the World a feed with the text."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("run", fed=True, on=root)
  assert engine.write(Text(f"{act}/stdin", "go"), on=root) == Text(f"{act}/stdin", "go")
  word = said(log, "feed")[0]
  assert (word[1], word[3], word[2]) == (act, "go", act)
  assert sand.fed == ["go"] and [one for one in sand.calls if one[0] == "feed"] == [word]
