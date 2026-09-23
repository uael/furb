"""Chain, the act that carries the label and the source of a chain."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_chain_carries_the_label_and_the_source_of_a_chain() -> None:
  """A chain carries the label and the source of a chain, and every chain is chainN whether boot or chain opened it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two", source=root)
  await settle()
  assert (root, two) == ("chain1", "chain2")
  assert said(log, "chain") == [("chain", root, OPERATOR, "", "root", ""), ("chain", two, OPERATOR, "", "two", root)]
  sand.script[root] = ["close(chain('deep'))"]
  one = engine.prompt(str, "fork", on=root)
  deep = await one
  (step,) = [a[1] for a in said(log, "rung") if a[2] == one]
  assert deep == "chain3"
  assert said(log, "chain")[-1] == engine.get(deep) == ("chain", deep, step, root, "deep", "")
