"""LARGEST, the most bytes that a read tells."""

import pytest

from conftest import born, of
from furb import engine
from furb.engine import LARGEST, Refused


async def test_largest_is_the_largest_number_of_bytes_that_a_read_tells_of_the_lines_that_its_show_picks() -> None:
  """LARGEST is the largest number of bytes that a read tells of the lines that its show picks."""
  assert LARGEST == 524288
  most = "\n".join(["é" * 500] * 523 + ["é" * 382 + "x"])
  past = most + "x"
  assert len(most.encode()) == LARGEST
  _, _, root = born("read('most.txt')\nclose(1)", files={"/w/most.txt": most + "\n", "/w/past.txt": past + "\n"})
  assert await engine.prompt(int, "read the most", on=root) == 1
  assert of(engine.turns(on=root), "read") == [
    f"#read1\nread1_path = 'most.txt'\n<s:read1_text>\n{most}</s:read1_text>"
  ]
  with pytest.raises(Refused, match=f"^past.txt shows {LARGEST + 1} bytes, over {LARGEST}$"):
    await engine.rung("read('past.txt')", on=root)
