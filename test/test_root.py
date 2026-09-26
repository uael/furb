"""ROOT, the name of the root."""

from conftest import Sand, life, said
from furb import engine
from furb.engine import OPERATOR


async def test_root_is_the_name_of_the_root() -> None:
  """ROOT is the name of the root, which every life opens first under that one name, and on which boot stands the life."""
  sand = Sand()
  log, root = life(sand)
  assert root == engine.ROOT == "chain1" and said(log, "chain")[0][1] == engine.ROOT
  assert [(a[1], a[2], a[3]) for a in said(log, "stand")] == [("stand1", OPERATOR, engine.ROOT)]
