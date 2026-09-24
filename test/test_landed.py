"""landed, what a read and a write give of what they were answered."""

from conftest import STANDS, Sand, life
from furb import engine
from furb.engine import Text


async def test_what_a_read_and_a_write_give_of_what_they_were_answered() -> None:
  """What a read and a write give of what they were answered: the Text of a path and a content that came as plain data, since no Text crosses to the World or back, and any other answer as it is."""
  assert engine.landed({"path": "/w/a.txt", "content": "one\n"}) == Text("/w/a.txt", "one\n")
  assert engine.landed([1, 2]) == [1, 2]
  assert engine.landed({"path": "/w/a.txt"}) == {"path": "/w/a.txt"}
  one = Text("x", "y")
  assert engine.landed(one) is one
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\n")
  assert engine.write(Text("b.txt", "two\n"), on=root) == Text("/w/b.txt", "two\n")
  asked = [a[4:] for a in sand.calls if a[0] in ("read", "write")]
  assert asked == [("a.txt",), ("b.txt", "two\n")]
  assert not any(isinstance(x, Text) for a in sand.calls for x in a)
