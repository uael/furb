"""site, who is speaking."""

from conftest import keeping, life, said, sown
from furb import engine
from furb.engine import OPERATOR, WORLD


async def test_who_is_speaking_is_the_site_which_every_fact_is_said_from() -> None:
  """Who is speaking is the site, which every fact is said from: the generator while it speaks, the run while it is stepped, the operator otherwise."""
  sand = sown()
  log, root = life(sand)
  assert engine.site.get() == OPERATOR
  sand.script[root] = ["here = site.get()\nclock()\nclose(here)"]
  step = await engine.prompt(str, "who speaks", on=root)
  assert step == said(log, "rung")[0][1]
  assert said(log, "clock")[0][2] == step
  assert said(log, "answer")[0][2] == WORLD
  engine.clock(on=root)
  assert said(log, "clock")[-1][2] == OPERATOR
  engine.drive(keeping([], ("done", "none://one", None)), "keeper")
  assert said(log, "done")[-1][2] == "keeper"
  assert engine.site.get() == OPERATOR
