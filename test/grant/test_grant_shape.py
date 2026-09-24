"""Grant, the act that carries a ceiling."""

from conftest import GRANT, STANDS, Sand, life, paragraphs, said, settle, verb
from furb import engine
from furb.engine import OPERATOR


async def test_a_grant_carries_the_ceiling_in_dollars_and_the_ceiling_in_share_of_the_window() -> None:
  """A grant carries the ceiling in dollars and the ceiling in share of the window."""
  sand = Sand(stands=STANDS, words=GRANT)
  log, root = life(sand)
  act = verb("grant", root)(usd=2.0, share=0.25)
  await settle()
  assert said(log, "grant") == [("grant", act, OPERATOR, root, 2.0, 0.25)]
  assert paragraphs(engine.turns(on=root))[-1] == f"#{act} usd=2.0 share=0.25\n{act}: Act[None] = Act({act!r})"
