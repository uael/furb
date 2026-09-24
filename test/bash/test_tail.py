"""TAIL, the span that the stdout of a command without a show is told as."""

from conftest import BASH, MANY, STANDS, Sand, exited, life, paragraphs, said, settle, verb
from furb import engine
from furb.engine import WORLD, Act


async def test_tail_is_the_span_of_the_last_250_lines() -> None:
  """TAIL is the span of the last 250 lines, which the stdout of a command without a show is told as."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  log, root = life(sand)
  lines = MANY.splitlines()
  assert await engine.rung(f"close(TAIL({lines!r}) == span(-250, -1)({lines!r}) == list(range(51, 301)))", on=root)
  act = verb("bash", root)("many")
  assert isinstance(act, Act)
  await settle()
  command = said(log, "bash")[0][1]
  engine.send("out", command, MANY, "stdout", by=WORLD)
  engine.send("exited", command, 0, by=WORLD)
  assert exited(await act)[0] == 0
  told = "\n".join(
    [f"#{command} exited 0", f"# {command}/stdout, 0 known", *[f"# {i} line {i}" for i in range(51, 301)]]
  )
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{command} exited")] == [told]
