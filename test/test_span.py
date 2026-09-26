"""span, the show of the lines lo through hi."""

from conftest import WORLD, Sand, dones, heads, life, said, settle
from furb import engine
from furb.engine import HIDDEN, Text, span

LINES = Text("/w/n.txt", "".join(f"line {i}\n" for i in range(1, 31))).lines
"""The lines of a text of thirty lines."""


def test_span_lo_hi_is_the_show_of_the_lines_lo_through_hi() -> None:
  """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty."""
  assert span(1, 20)(LINES) == list(range(1, 21))
  assert span(-20, -1)(LINES) == list(range(11, 31))
  assert span(-1, -1)(LINES) == [30]
  assert span(2, 3)(["a", "b", "c", "d"]) == [2, 3]
  assert span(2, 3)(["a"]) == []
  assert span(1, 9)([]) == []


async def test_a_span_that_holds_no_line_shows_none_which_hidden_is() -> None:
  """A span that holds no line shows none, which HIDDEN is, so what a hidden show shows stands in no turns: a command that takes one tells its header and its binding alone, and a read that takes one tells nothing."""
  assert span(0, 0)(LINES) == HIDDEN(LINES) == []
  sand = Sand(files={"/w/n.txt": "one\n"})
  log, root = life(sand)
  act = engine.bash("quiet", show=HIDDEN, on=root)
  assert (await act).code == 0
  assert await engine.rung("read('n.txt', HIDDEN)", on=root) is None
  await settle()
  assert said(log, "bash")[0][1] == act and [(a[1], a[2]) for a in dones(log, "bash")] == [(act, WORLD)]
  assert [a[3] for a in said(log, "tell") if a[1] == act] == [[f"#{act}", f"{act}: Act[Exit] = Act({act!r})"]]
  assert [one for one in heads(engine.turns(on=root)) if one.startswith(("#bash", "#read"))] == [f"#{act}"]
