"""asked, every query the life has put under its name."""

from conftest import STANDS, Sand, life
from furb import engine
from furb.engine import OPERATOR


async def test_asked_holds_every_query_the_life_has_put_under_its_name() -> None:
  """asked holds every query the life has put under its name, and boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert [a[0] for a in engine.asked.values()] == ["stand"]
  assert engine.clock(on=root) == 1001.0
  assert engine.asked["clock@operator.2"] == ("clock", "clock@operator.2", OPERATOR, "chain1")
  life(Sand(stands=STANDS))
  assert [a[0] for a in engine.asked.values()] == ["stand"]


async def test_it_must_be_answered_while_the_one_that_asked_waits() -> None:
  """It must be answered while the one that asked waits, so what it was answered a life holds under its name for as long as the life lives."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  got = engine.clock(on=root)
  name = "clock@operator.2"
  assert engine.outcomes[name] == got == 1001.0
  assert engine.clock(on=root) == 1002.0
  assert engine.asked[name] == ("clock", name, OPERATOR, root) and engine.outcomes[name] == 1001.0
