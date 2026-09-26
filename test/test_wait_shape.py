"""Wait, the seconds that must pass before the World is done with it."""

from conftest import Sand, life, said, settle
from furb import engine


async def test_a_wait_carries_the_seconds_that_must_pass_before_the_world_is_done_with_it() -> None:
  """A wait carries the seconds that must pass before the World is done with it."""
  sand = Sand()
  _, root = life(sand)
  slow = engine.wait(0.2, on=root)
  quick = engine.wait(on=root)
  held = engine.transcript(root)
  assert [(a[1], a[4]) for a in said(held, "wait")] == [(slow, 0.2), (quick, 0.0)]
  await settle()
  assert engine.peek(quick, ...) is not ... and engine.peek(slow, ...) is ...
  assert await slow is None
