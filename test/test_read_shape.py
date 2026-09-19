"""Read, the question of the text at a path."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import Text


async def test_a_read_is_the_question_of_the_text_at_a_path() -> None:
  """A read is the question of the text at a path."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n", "/w/b.txt": "three\n"}, stands=STANDS)
  log, root = life(sand)
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("b.txt", on=root) == Text("/w/b.txt", "three\n")
  asked = said(log, "read")
  assert [(a[1], a[3], a[4]) for a in asked] == [
    ("read://operator.2", root, "a.txt"),
    ("read://operator.3", root, "b.txt"),
  ]
