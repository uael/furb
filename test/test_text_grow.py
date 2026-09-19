"""grow, the text as more of it is told."""

import pytest

from conftest import Sand, life, said, settle
from furb import engine
from furb.engine import WORLD, Text


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
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
  engine.send("out", command, "half\n", "stdout", by=WORLD)
  engine.send("out", command, "rest\n", "stdout", by=WORLD)
  await settle()
  assert engine.read(f"{command}/stdout", on=root) == Text(f"{command}/stdout", "half\nrest\n")
  engine.send("exited", command, 0, by=WORLD)
  assert (await act).stdout.content == "half\nrest\n"
