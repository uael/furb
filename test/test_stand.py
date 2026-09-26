"""stand, the question of what the chains stand on."""

from conftest import STANDS, Sand, life, relived, said, settle
from furb import engine
from furb.engine import OPERATOR

LATER = [[["operator", [], 200000], ["o", ["low"], 200000]], "/z", "o/low"]
"""What a later World offers: another roster, another directory and another default actor."""


async def test_stand_asks_the_world_what_the_chains_stand_on_and_gives_the_answer() -> None:
  """stand asks the World what the chains stand on and gives the answer, which boot does on the root at the tip of every life."""
  sand = Sand()
  _, root = life(sand)
  assert [(a[1], a[2], a[3]) for a in said(sand.calls, "stand")] == [("stand1", OPERATOR, root)]
  sand.stands = LATER
  assert engine.stand() == LATER and engine.standing() == LATER
  await settle()
  later = Sand()
  await relived(later, list(sand.record))
  assert [(a[1], a[2], a[3]) for a in said(later.calls, "stand")] == [("stand3", OPERATOR, root)]


async def test_only_a_stand_on_the_root_changes_the_standing() -> None:
  """Only a stand on the root changes the standing, since the standing has one home."""
  sand = Sand()
  _, root = life(sand)
  two = engine.chain("two")
  sand.stands = LATER
  assert engine.stand(on=two) == LATER
  assert engine.standing() == STANDS and engine.module(two)["actor"] == engine.module(root)["actor"] == "m/low"
  assert engine.stand() == LATER and engine.module(two)["actor"] == engine.module(root)["actor"] == "o/low"
