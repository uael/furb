"""Feed, what a write to the stdin door of a command hands the World."""

from conftest import born, said
from furb import engine
from furb.engine import Text


async def test_a_write_to_the_stdin_door_of_a_command_with_a_stdin_of_none_hands_the_world_a_feed_with_the_text() -> (
  None
):
  """A write to the stdin door of a command with a stdin of None hands the World a feed with the text and gives that text, and a write of nothing hands it a feed of None, which closes the stdin."""
  sand, log, root = born(auto=False)
  act = engine.bash("run", stdin=None, on=root)
  assert engine.write(Text(f"{act}/stdin", "go"), on=root) == Text(f"{act}/stdin", "go")
  assert engine.write(Text(f"{act}/stdin"), on=root) == Text(f"{act}/stdin")
  assert [(word[1], word[2], word[3]) for word in said(log, "feed")] == [(act, root, "go"), (act, root, None)]
  assert sand.fed == ["go", None] and [one for one in sand.calls if one[0] == "feed"] == said(log, "feed")
