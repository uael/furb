"""HEAD, the span a read without a show is told as."""

from conftest import born, settle
from furb import engine

LONG = "".join(f"line {i}\n" for i in range(1, 2003))
"""A text of two thousand and two lines, which is two more than HEAD tells."""


async def test_head_is_the_span_of_the_first_2000_lines_which_a_read_without_a_show_is_told_as() -> None:
  """HEAD is the span of the first 2000 lines, which a read without a show is told as."""
  assert engine.HEAD(LONG.splitlines()) == list(range(1, 2001))
  _, _, root = born("read('long.txt')\nclose(1)", files={"/w/long.txt": LONG})
  assert await engine.thread(int, "read it", on=root) == 1
  await settle()
  told = "\n".join(["#read1", "read1_path = 'long.txt'", "<s:read1_text>", *[f"line {i}" for i in range(1, 2001)]])
  assert engine.turns(on=root)[-1][1] == f"{told}</s:read1_text>\n\n#thread1 closed\nthread1_value = 1"
