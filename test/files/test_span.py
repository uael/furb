"""span, the show of the lines lo through hi."""

from conftest import BASH, FILES, STANDS, Sand, heads, life, said, settle, worded
from furb import engine
from furb.engine import WORLD

LINES = [f"line {i}" for i in range(1, 31)]
"""The lines of a text of thirty lines."""


async def test_span_lo_hi_is_the_show_of_the_lines_lo_through_hi() -> None:
  """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty."""
  word = (
    f"lines = {LINES!r}\n"
    "close([span(1, 20)(lines), span(-20, -1)(lines), span(-1, -1)(lines), span(2, 3)(['a', 'b', 'c', 'd']), "
    "span(2, 3)(['a']), span(1, 9)([])])"
  )
  assert await worded(word) == [list(range(1, 21)), list(range(11, 31)), [30], [2, 3], [], []]


async def test_a_span_that_holds_no_line_shows_none_which_hidden_is() -> None:
  """A span that holds no line shows none, which HIDDEN is, so what a hidden show shows stands in no turns: an act that takes one tells its header and its binding alone, and a query that takes one tells nothing."""
  sand = Sand(files={"/w/n.txt": "one\n"}, stands=STANDS, words=BASH)
  log, root = life(sand)
  assert await engine.rung(f"close([span(0, 0)({LINES!r}), HIDDEN({LINES!r})])", on=root) == [[], []]
  act = await engine.rung("close(bash('quiet', show=HIDDEN))", on=root)
  await settle()
  assert await engine.rung("read('n.txt', HIDDEN)", on=root) is None
  await settle()
  assert said(log, "bash")[0][1] == act and said(log, "exited") == [("exited", act, WORLD, 0)]
  assert [a[3] for a in said(log, "tell") if a[1] == act] == [[f"#{act}", f"{act}: Act[Exit] = Act({act!r})"]]
  assert [one for one in heads(engine.turns(on=root)) if one.startswith(("#bash", "#read"))] == [f"#{act}"]


async def test_span_grep_and_differs_make_the_shows_of_the_file() -> None:
  """span, grep and differs make the shows of the file."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\nthree\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  word = "close([f(['one', 'two', 'three']) for f in (span(2, 3), grep('^t'), differs(['one', 'TWO', 'three']))])"
  assert await engine.rung(word, on=root) == [[2, 3], [2, 3], [2]]
  word = "read('n.txt', span(1, 1))\nread('n.txt', grep('^th'))\nwrite(Text('n.txt', 'one\\ntwo\\nthree\\n'))"
  assert await engine.rung(word, on=root) is None
  assert [one for one in heads(engine.turns(on=root)) if one.startswith(("#read", "#write"))] == ["#read n.txt"] * 2
