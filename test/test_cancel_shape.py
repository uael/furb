"""Cancel, the control that ends everything it is over."""

from asyncio import CancelledError

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import WORLD


async def test_a_cancel_ends_everything_it_is_over() -> None:
  """A cancel ends everything it is over: each of them is done with CancelledError, and none of them says anything of its own again."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  one = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.cancel(one)
  await settle()
  assert [type(engine.peek(x)).__name__ for x in (one, step, command)] == ["CancelledError"] * 3
  mark = len(log)
  engine.send("out", command, "late\n", "stdout", by=WORLD)
  await settle()
  assert [a for a in log[mark:] if a[2] in (one, step, command)] == []


async def test_a_cancel_reaches_to_any_depth_and_on_whatever_chain() -> None:
  """A cancel reaches to any depth, and on whatever chain."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["two = chain('two')\nx = bash('slow', on=two)\nclose((await x).code)"]
  one = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  assert engine.get(command)[3] == said(log, "chain")[-1][1] != root
  assert engine.under(command, step) and engine.under(step, one)
  engine.cancel(one)
  await settle()
  assert isinstance(engine.peek(command), CancelledError)
