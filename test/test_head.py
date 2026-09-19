"""HEAD, the span a read without a show is told as."""

from conftest import STANDS, Sand, life, settle, shown, tags
from furb import engine

LONG = "".join(f"line {i}\n" for i in range(1, 2003))
"""A text of two thousand and two lines, which is two more than HEAD tells."""


async def test_head_is_the_span_of_the_first_2000_lines_which_a_read_without_a_show_is_told_as() -> None:
  """HEAD is the span of the first 2000 lines, which a read without a show is told as."""
  assert engine.HEAD(LONG.splitlines()) == list(range(1, 2001))
  sand = Sand(files={"/w/long.txt": LONG}, stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["read('long.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  await settle()
  body = shown(tags(engine.turns(on=root), "read")[0])[0][2]
  assert isinstance(body, str)
  lines = body.splitlines()
  assert len(lines) == 2000 and lines[0] == "1 line 1" and lines[-1] == "2000 line 2000"
