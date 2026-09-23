"""offered, the window an actor reads."""

from conftest import STANDS, Sand, life, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_the_window_an_actor_reads() -> None:
  """The window an actor reads, and nothing at all when the roster holds no such actor, or when that one takes no such effort."""
  assert engine.offered(STANDS, "m/low") == 400000
  assert engine.offered(STANDS, "m/high") == 400000
  assert engine.offered(STANDS, "m") == 400000
  assert engine.offered(STANDS, OPERATOR) == 200000
  assert engine.offered(STANDS, "n/high") is None
  assert engine.offered(STANDS, "ghost") is None
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.outcomes[ghost], Refused)
