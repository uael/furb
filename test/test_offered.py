"""offered, a slot of the roster entry of an actor."""

from conftest import STANDS, born, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_the_window_an_actor_reads() -> None:
  """The slot that at names of the roster entry of an actor, its window when at is unsaid and its doctrine at 3, the empty string for a slot that the entry leaves unsaid, and nothing at all when the roster holds no such actor, or when that one takes no such effort."""
  assert engine.offered(STANDS[0], "m/low") == 400000
  assert engine.offered(STANDS[0], "m/high") == 400000
  assert engine.offered(STANDS[0], "m") == 400000
  assert engine.offered(STANDS[0], OPERATOR) == 200000
  assert engine.offered(STANDS[0], "n/high") is None
  assert engine.offered(STANDS[0], "ghost") is None
  roster = [*STANDS[0], ["q", ["low"], 300000, "Look first."]]
  assert engine.offered(roster, "q/low", 3) == "Look first."
  assert engine.offered(roster, "q") == 300000
  assert engine.offered(roster, "m/low", 3) == ""
  assert engine.offered(roster, "q/high", 3) is None
  _, _, root = born()
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
