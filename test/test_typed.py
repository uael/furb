"""typed, a standing as its declared type."""

from conftest import STANDS, Sand, attr, life, plain, relived, settle, tags
from furb import engine


async def test_a_standing_as_its_declared_type() -> None:
  """A standing as its declared type, whether the World or the record gave it, which is how a chain takes one and how the journal weighs one against what a chain stands on."""
  kept = plain([("", ("stand", "stand://x.1", "x", "x"), STANDS)])[0][2]
  assert kept != STANDS and engine.typed(kept) == STANDS == engine.typed(STANDS)
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  later = Sand(stands=STANDS)
  _, over = await relived(later, plain(sand.record))
  assert attr(tags(engine.turns(on=over), "opened")[1], "roster") == STANDS[0] and later.record == []
