"""Write, the question of putting a text at its path."""

from conftest import Sand, acts, life
from furb import engine
from furb.engine import OPERATOR, Text


async def test_a_write_is_the_question_of_putting_a_text_at_its_path() -> None:
  """A write is the question of putting a text at its path."""
  sand = Sand()
  log, root = life(sand)
  assert engine.write(Text("b.txt", "one\ntwo\n"), on=root) == Text("/w/b.txt", "one\ntwo\n")
  assert sand.files == {"/w/b.txt": "one\ntwo\n"}
  asked = [a for a in acts(log).values() if a[0] == "write"]
  assert asked == [("write", "write1", OPERATOR, root, Text("b.txt", "one\ntwo\n"))]
  assert engine.peek("write1") == Text("/w/b.txt", "one\ntwo\n")
