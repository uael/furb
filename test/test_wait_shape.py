"""Wait, the seconds that must pass before the World is done with it."""

from conftest import born, said, settle
from furb import engine


async def test_a_wait_carries_the_seconds_that_must_pass_before_the_world_is_done_with_it() -> None:
  """A wait carries the seconds that must pass before the World is done with it."""
  _, _, root = born()
  # The slow wait is long enough that no load of the machine lets it pass while the loop settles.
  slow = engine.wait(30.0, on=root)
  quick = engine.wait(on=root)
  held = engine.transcript(root)
  assert [(a[1], a[4]) for a in said(held, "wait")] == [(slow, 30.0), (quick, 0.0)]
  await settle()
  assert engine.peek(quick, ...) is not ... and engine.peek(slow, ...) is ...
  engine.cancel(slow)
