"""Read, the question of the text at a path."""

from conftest import STANDS, Sand, life
from furb import engine
from furb.engine import OPERATOR, Text


async def test_a_read_is_the_question_of_the_text_at_a_path() -> None:
  """A read is the question of the text at a path."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n", "/w/b.txt": "three\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert engine.read("b.txt", on=root) == Text("/w/b.txt", "three\n")
  asked = [a for a in engine.asked.values() if a[0] == "read"]
  assert asked == [
    ("read", "read@operator.2", OPERATOR, root, "a.txt"),
    ("read", "read@operator.3", OPERATOR, root, "b.txt"),
  ]
  assert [engine.outcomes[a[1]] for a in asked] == [Text("/w/a.txt", "one\ntwo\n"), Text("/w/b.txt", "three\n")]
