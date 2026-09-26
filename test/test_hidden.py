"""HIDDEN, the span of no line."""

from conftest import STANDS, Sand, acts, heads, life, said, settle
from furb import engine


async def test_hidden_is_the_span_of_no_line_which_an_act_takes_to_tell_nothing_of_itself() -> None:
  """HIDDEN is the span of no line, which an act takes to tell nothing of itself but its header and its binding."""
  assert engine.HIDDEN(["one", "two", "three"]) == []
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["q = bash('quiet', show=HIDDEN)\nawait q\nread('a.txt', HIDDEN)\nclose(1)"]
  assert await engine.prompt(int, "quietly", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  told = [a[3] for a in said(log, "tell") if a[1] == command or a[3][0].startswith("#read")]
  assert told == [[f"#{command}", f"{command}: Act[Exit] = Act({command!r})"]]
  assert [one for one in heads(engine.turns(on=root)) if one.startswith((f"#{command}", "#read"))] == [f"#{command}"]
  assert [a[4] for a in acts(log).values() if a[0] == "read"] == ["a.txt"]
