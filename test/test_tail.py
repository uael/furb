"""TAIL, the span that the stdout of a command without a show is told as."""

from conftest import MANY, STANDS, Sand, life, paragraphs, said, settle, world_says
from furb import engine
from furb.engine import TAIL, span


async def test_tail_is_the_span_of_the_last_250_lines() -> None:
  """TAIL is the span of the last 250 lines, which the stdout of a command without a show is told as."""
  assert TAIL(MANY.splitlines()) == span(-250, -1)(MANY.splitlines()) == list(range(51, 301))
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("many", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  world_says("out", command, MANY, "stdout")
  sand.exits(command, 0)
  assert (await act).code == 0
  told = "\n".join(
    [f"#{command} exited 0", f"# {command}/stdout, 0 known", *[f"# {i} line {i}" for i in range(51, 301)]]
  )
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{command} exited")] == [told]
