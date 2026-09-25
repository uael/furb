"""grow, the text as more of it is told."""

from conftest import Sand, life, said, settle, world_says
from furb import engine
from furb.engine import Text


async def test_the_text_as_more_of_it_is_told() -> None:
  """The text as more of it is told, which is how a stream of a command grows, and which comes from no text, since a stream that grows is no edit of one."""
  grown = Text("/x/stdout").grow("half\n").grow("rest\n")
  assert grown.content == "half\nrest\n" and grown.lines == ["half", "rest"]
  assert grown.before is None and grown.undo() is grown
  sand = Sand(auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  world_says("out", command, "half\n", "stdout")
  world_says("out", command, "rest\n", "stdout")
  await settle()
  assert engine.read(f"{command}/stdout", on=root) == Text(f"{command}/stdout", "half\nrest\n")
  sand.exits(command, 0)
  assert (await act).stdout.content == "half\nrest\n"
