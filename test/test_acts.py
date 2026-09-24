"""acts, every act of the life under its name."""

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR


async def test_acts_holds_every_act_of_the_life_under_its_name() -> None:
  """acts holds every act of the life under its name, the one fact the life holds under that name, and boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.wait(5, on=root)
  assert engine.acts[root] == ("chain", root, OPERATOR, "", "root", "")
  assert engine.acts[one] == ("wait", one, OPERATOR, root, 5)
  assert set(engine.acts) == {root, one}
  _, other = life(Sand(stands=STANDS))
  assert set(engine.acts) == {other}


async def test_what_a_caller_holds_of_an_act_is_its_name() -> None:
  """What a caller holds of an act is its name, and the act itself the life holds under that name, so what the life fills in the caller reads through the name."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  assert isinstance(one, str) and one == engine.acts[one][1]
  await settle()
  assert one not in engine.outcomes
  engine.close(3, one)
  await settle()
  assert engine.outcomes[one] == 3
  assert engine.peek(one) == engine.outcomes[one] and (await one) == engine.outcomes[one]
