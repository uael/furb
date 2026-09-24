"""asked, every query the life has put under its name."""

from conftest import STANDS, Sand, life
from furb import engine
from furb.engine import OPERATOR


async def test_asked_holds_every_query_the_life_has_put_under_its_name() -> None:
  """asked holds every query the life has put under its name, and boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert [a[0] for a in engine.asked.values()] == ["stand"]
  assert engine.cwd(on=root) == "/w"
  assert engine.asked["cwd@operator.2"] == ("cwd", "cwd@operator.2", OPERATOR, "chain1")
  life(Sand(stands=STANDS))
  assert [a[0] for a in engine.asked.values()] == ["stand"]


async def test_it_must_be_answered_while_the_one_that_asked_waits() -> None:
  """It must be answered while the one that asked waits, so what it was answered a life holds under its name for as long as the life lives."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  got = engine.cwd(on=root)
  name = "cwd@operator.2"
  assert engine.outcomes[name] == got == "/w"
  assert engine.cd("/x", on=root) == "/x"
  assert engine.cwd(on=root) == "/x"
  assert engine.asked[name] == ("cwd", name, OPERATOR, root) and engine.outcomes[name] == "/w"
