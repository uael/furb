"""acting, the run a word speaks from."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import OPERATOR, WORLD, site


async def test_the_run_a_word_speaks_from() -> None:
  """The run a fact speaks from, which is the name the site holds when that name is an act's, and nothing at all for the operator and for the World, since a fact of theirs is said from no run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(acting())"]
  assert await engine.prompt(str, "who speaks", on=root) == said(log, "rung")[0][1]
  assert site.get() == OPERATOR and engine.acting() == ""
  token = site.set(WORLD)
  assert engine.acting() == ""
  site.reset(token)
  token = site.set(root)
  assert engine.acting() == root
  site.reset(token)
