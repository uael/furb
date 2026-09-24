"""Read, the question of the text at a path."""

from conftest import FILES, STANDS, Sand, life, texted, verb
from furb import engine
from furb.engine import OPERATOR


async def test_a_read_is_the_question_of_the_text_at_a_path() -> None:
  """A read is the question of the text at a path."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n", "/w/b.txt": "three\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert texted(verb("read", root)("a.txt")) == ("/w/a.txt", "one\ntwo\n")
  assert texted(verb("read", root)("b.txt")) == ("/w/b.txt", "three\n")
  asked = [a for a in engine.asked.values() if a[0] == "read"]
  assert [a[2:] for a in asked] == [(OPERATOR, root, "a.txt"), (OPERATOR, root, "b.txt")]
  assert [engine.outcomes[a[1]] for a in asked] == [
    {"path": "/w/a.txt", "content": "one\ntwo\n"},
    {"path": "/w/b.txt", "content": "three\n"},
  ]
