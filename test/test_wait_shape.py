"""Wait, the seconds that must pass before the World is done with it."""

import pytest

from conftest import STANDS, Sand, life, said, settle
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_wait_carries_the_seconds_that_must_pass_before_the_world_is_done_with_it() -> None:
  """A wait carries the seconds that must pass before the World is done with it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  slow = engine.wait(0.2, on=root)
  quick = engine.wait(on=root)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [(a[1], a[4]) for a in said(held, "wait")] == [(slow, 0.2), (quick, 0.0)]
  await settle()
  assert quick in engine.outcomes and slow not in engine.outcomes
  assert await slow is None
