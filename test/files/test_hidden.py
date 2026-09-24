"""HIDDEN, the span of no line."""

from conftest import BASH, STANDS, Sand, heads, life, said, settle
from furb import engine


async def test_hidden_is_the_span_of_no_line_which_an_act_takes_to_tell_nothing_of_itself() -> None:
  """HIDDEN is the span of no line, which an act takes to tell nothing of itself but its header and its binding."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS, words=BASH)
  log, root = life(sand)
  assert await engine.rung("close(HIDDEN(['one', 'two', 'three']))", on=root) == []
  sand.script[root] = ["q = bash('quiet', show=HIDDEN)\nawait q\nread('a.txt', HIDDEN)\nclose(1)"]
  assert await engine.prompt(int, "quietly", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  told = [a[3] for a in said(log, "tell") if a[1] == command or a[3][0].startswith("#read")]
  assert told == [[f"#{command}", f"{command}: Act[Exit] = Act({command!r})"]]
  assert [one for one in heads(engine.turns(on=root)) if one.startswith((f"#{command}", "#read"))] == [f"#{command}"]
  assert [a[4] for a in engine.asked.values() if a[0] == "read"] == ["a.txt"]
