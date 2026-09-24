"""Write, the question of putting a content at a path."""

from conftest import FILES, STANDS, Sand, life
from furb import engine


async def test_a_write_is_the_question_of_putting_a_content_at_a_path() -> None:
  """A write is the question of putting a content at a path, which carries the path and the content."""
  sand = Sand(stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("close(write(Text('b.txt', 'one\\ntwo\\n')).content)", on=root) == "one\ntwo\n"
  assert sand.files == {"/w/b.txt": "one\ntwo\n"}
  (asked,) = [a for a in engine.asked.values() if a[0] == "write"]
  assert asked[3:] == (root, "b.txt", "one\ntwo\n") and asked[2] in engine.acts
  assert engine.outcomes[asked[1]] == {"path": "/w/b.txt", "content": "one\ntwo\n"}
