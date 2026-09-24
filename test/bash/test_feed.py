"""Feed, what a write to the stdin door of a command hands the World."""

from conftest import BASH, STANDS, Sand, life, made, said, texted, verb


async def test_a_write_to_the_stdin_door_of_a_fed_command_hands_the_world_a_feed_with_the_text() -> None:
  """A write to the stdin door of a fed command hands the World a feed with the text."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  act = verb("bash", root)("run", fed=True)
  assert texted(verb("write", root)(made(root, "Text", f"{act}/stdin", "go"))) == (f"{act}/stdin", "go")
  word = said(log, "feed")[0]
  assert (word[1], word[3], word[2]) == (act, "go", act)
  assert sand.fed == ["go"] and [one for one in sand.calls if one[0] == "feed"] == [word]
