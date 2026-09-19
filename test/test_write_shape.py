"""Write, the question of putting a text at its path."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import Text


async def test_a_write_is_the_question_of_putting_a_text_at_its_path() -> None:
  """A write is the question of putting a text at its path."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert engine.write(Text("b.txt", "one\ntwo\n"), on=root) == Text("/w/b.txt", "one\ntwo\n")
  assert sand.files == {"/w/b.txt": "one\ntwo\n"}
  asked = said(log, "write")
  assert [(a[1], a[3], a[4]) for a in asked] == [("write://operator.2", root, Text("b.txt", "one\ntwo\n"))]
