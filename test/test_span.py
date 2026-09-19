"""span, the show of the lines lo through hi."""

import pytest

from conftest import STANDS, Sand, life, said, settle, tags
from furb import engine
from furb.engine import HIDDEN, span

LINES = [f"line {i}" for i in range(1, 31)]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_span_lo_hi_is_the_show_of_the_lines_lo_through_hi() -> None:
  """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty."""
  assert span(1, 20)(LINES) == list(range(1, 21))
  assert span(-20, -1)(LINES) == list(range(11, 31))
  assert span(-1, -1)(LINES) == [30]
  assert span(2, 3)(["a", "b", "c", "d"]) == [2, 3]
  assert span(2, 3)(["a"]) == []
  assert span(1, 9)([]) == []


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_span_that_holds_no_line_shows_none_which_hidden_is() -> None:
  """A span that holds no line shows none, which HIDDEN is, and what a hidden show shows stands in no turns at all, neither its open nor its close."""
  assert span(0, 0)(LINES) == HIDDEN(LINES) == []
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("quiet", show=HIDDEN, on=root)
  assert (await act).code == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  assert [tag for tag in tags(engine.turns(on=root)) if ("id", command) in tag[1]] == []
