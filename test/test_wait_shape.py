"""Wait, the seconds that must pass before the World is done with it."""

from conftest import STANDS, Sand, born, plain, relived, said, settle
from furb import engine


async def test_a_wait_carries_the_seconds_that_must_pass_before_the_world_is_done_with_it() -> None:
  """A wait carries the seconds that must pass before the World is done with it, and the names of its acts, which are plain, so the journal makes it again."""
  sand, _, root = born()
  # The slow wait is long enough that no load of the machine lets it pass while the loop settles.
  slow = engine.wait(30.0, on=root)
  quick = engine.wait(on=root)
  both = engine.wait(30.0, slow, quick, on=root)
  held = engine.transcript(root)
  assert [a[1:2] + a[4:] for a in said(held, "wait")] == [(slow, 30.0), (quick, 0.0), (both, 30.0, slow, quick)]
  await settle()
  assert engine.peek(quick, ...) is not ... and engine.peek(slow, ...) is ...
  engine.cancel(slow)
  again, _ = await relived(Sand(stands=STANDS), plain(sand.record))
  assert sorted(a[1:2] + a[4:] for a in said(again, "wait")) == [(slow, 30.0), (quick, 0.0), (both, 30.0, slow, quick)]
