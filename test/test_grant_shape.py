"""Grant, the act that carries a ceiling."""

from conftest import Sand, life, paragraphs, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_a_grant_carries_the_ceiling_in_dollars_and_the_ceiling_in_share_of_the_window() -> None:
  """A grant carries the ceiling in dollars and the ceiling in share of the window."""
  sand = Sand()
  log, root = life(sand)
  act = engine.grant(usd=2.0, share=0.25, on=root)
  await settle()
  assert said(log, "grant") == [("grant", act, OPERATOR, root, 2.0, 0.25)]
  assert paragraphs(engine.turns(on=root))[-1] == f"#{act} usd=2.0 share=0.25\n{act}: Act[None] = Act({act!r})"
