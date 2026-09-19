"""differs, the show of the lines that differ, which is what a write shows."""

import pytest

from conftest import STANDS, Sand, attr, life, settle, shown, tags
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_differs_lines_is_the_show_of_the_lines_that_differ_from_the_lines_it_holds() -> None:
  """differs(lines) is the show of the lines that differ from the lines it holds, which is what a write shows of what came back."""
  assert engine.differs(["one", "two"])(["one", "new", "two"]) == [2, 3]
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["read(__name__)\nwrite(Text(__name__, 'k = 4'))\nclose(4)"]
  assert await engine.prompt(int, "write to my own program", on=root) == 4
  await settle()
  wrote = tags(engine.turns(on=root), "write")
  assert [(attr(one, "known"), one[2]) for one in shown(wrote[0])] == [(4, "5 k = 4")]
