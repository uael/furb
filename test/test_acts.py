"""acts, every act of the life under its name."""

import pytest

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR, TIMEOUT, Exit


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_acts_holds_every_act_of_the_life_under_its_name() -> None:
  """acts holds every act of the life under its name, the one fact the life holds under that name, and boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert engine.acts[root] == ("chain", root, OPERATOR, "", "root", "")
  assert engine.acts[one] == ("bash", one, OPERATOR, root, "echo hi", False, TIMEOUT)
  assert set(engine.acts) == {root, one}
  _, other = life(Sand(stands=STANDS))
  assert set(engine.acts) == {other}


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_what_a_caller_holds_of_an_act_is_its_name() -> None:
  """What a caller holds of an act is its name, and the act itself the life holds under that name, so what the life fills in the caller reads through the name."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  one = engine.bash("echo hi", on=root)
  assert isinstance(one, str) and one == engine.acts[one][1]
  assert one not in engine.outcomes
  await settle()
  got = engine.outcomes[one]
  assert isinstance(got, Exit) and got.code == 0
  assert engine.peek(one) is engine.outcomes[one] and (await one) is engine.outcomes[one]
