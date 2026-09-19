"""actor, the default actor of the chain."""

from conftest import STANDS, Sand, life
from furb import engine


async def test_actor_is_the_default_actor_of_the_chain_bound_from_the_standing() -> None:
  """actor is the default actor of the chain, bound from the standing."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert STANDS[2] == "m/low"
  assert engine.modules[root]["actor"] == "m/low"
  assert await engine.rung("close(actor)", on=root) == "m/low"


async def test_the_program_rebinds_actor_like_any_name_and_the_last_binding_wins() -> None:
  """The program rebinds actor like any name, and the last binding wins."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("actor = 'n/low'", on=root)
  assert engine.modules[root]["actor"] == "n/low"
  await engine.rung("actor = 'm/high'", on=root)
  assert engine.modules[root]["actor"] == "m/high"
  assert await engine.rung("close(actor)", on=root) == "m/high"
