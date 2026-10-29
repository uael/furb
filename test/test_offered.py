"""offered, the window of the roster entry of an actor."""

from conftest import STANDS, born, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_the_window_an_actor_reads() -> None:
  """The window of the roster entry of an actor, and nothing at all when the roster holds no such actor, or when that one takes no such effort."""
  assert engine.offered(STANDS[0], "m/low") == 400000
  assert engine.offered(STANDS[0], "m/high") == 400000
  assert engine.offered(STANDS[0], "m") == 400000
  assert engine.offered(STANDS[0], OPERATOR) == 200000
  assert engine.offered(STANDS[0], "n/high") is None
  assert engine.offered(STANDS[0], "ghost") is None
  roster = [*STANDS[0], ["q", ["low"], 300000]]
  assert engine.offered(roster, "q/low") == 300000
  assert engine.offered(roster, "q") == 300000
  assert engine.offered(roster, "q/high") is None
  _, _, root = born()
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
