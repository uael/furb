"""TAIL, the span that the stdout of a command without a show is told as."""

from conftest import MANY, STANDS, Sand, attr, life, said, settle, shown, tags
from furb import engine
from furb.engine import TAIL, WORLD, span


async def test_tail_is_the_span_of_the_last_250_lines() -> None:
  """TAIL is the span of the last 250 lines, which the stdout of a command without a show is told as."""
  assert TAIL(MANY.splitlines()) == span(-250, -1)(MANY.splitlines()) == list(range(51, 301))
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("many", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  engine.send("out", command, MANY, "stdout", by=WORLD)
  engine.send("exited", command, 0, by=WORLD)
  assert (await act).code == 0
  closed = [tag for tag in tags(engine.turns(on=root), "closed") if attr(tag, "id") == command]
  told = shown(closed[0])[0]
  assert attr(told, "path") == f"{command}/stdout"
  assert told[2].splitlines()[0] == "51 line 51"
  assert told[2].splitlines()[-1] == "300 line 300"
  assert len(told[2].splitlines()) == 250
