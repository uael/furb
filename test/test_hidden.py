"""HIDDEN, the span of no line."""

import pytest

from conftest import STANDS, Sand, life, said, settle, tags
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_hidden_is_the_span_of_no_line_which_an_act_takes_to_tell_nothing_of_itself() -> None:
  """HIDDEN is the span of no line, which an act takes to tell nothing of itself."""
  assert engine.HIDDEN(["one", "two", "three"]) == []
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["q = bash('quiet', show=HIDDEN)\nawait q\nread('a.txt', HIDDEN)\nclose(1)"]
  assert await engine.prompt(int, "quietly", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  told = tags(engine.turns(on=root))
  assert [tag for tag in told if tag[0] == "read"] == []
  assert [tag for tag in told if ("id", command) in tag[1]] == []
