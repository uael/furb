"""Grant, the act that carries a ceiling."""

from conftest import STANDS, Sand, attr, life, said, settle, tags
from furb import engine


async def test_a_grant_carries_the_ceiling_in_dollars_and_the_ceiling_in_share_of_the_window() -> None:
  """A grant carries the ceiling in dollars and the ceiling in share of the window."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.grant(usd=2.0, share=0.25, on=root)
  await settle()
  word = said(log, "grant")[0]
  assert (word[4], word[5], word[1], word[3]) == (2.0, 0.25, act, root) and len(word) == 6
  told = [tag for tag in tags(engine.turns(on=root), "opened") if ("id", act) in tag[1]]
  assert [(attr(tag, "usd"), attr(tag, "share")) for tag in told] == [(2.0, 0.25)]
