"""lineage, the makers of an act one under the other."""

from conftest import STANDS, Sand, life, said
from furb import engine


async def test_the_lineage_of_an_act_the_makers_of_it_one_under_the_other() -> None:
  """The lineage of an act, the makers of it one under the other, which its name holds after its kind."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  act, step, command = said(log, "prompt")[0][1], said(log, "rung")[0][1], said(log, "bash")[0][1]
  assert engine.lineage(root) == "operator.1"
  assert (engine.lineage(act), engine.lineage(step)) == ("operator.2", "operator.2.1")
  assert engine.lineage(command) == "operator.2.1.1"
  assert command == f"bash://{engine.lineage(command)}"
