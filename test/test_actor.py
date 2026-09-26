"""actor, the default actor of the chain."""

from conftest import STANDS, born
from furb import engine


async def test_actor_is_the_default_actor_of_the_chain_bound_from_the_standing() -> None:
  """actor is the default actor of the chain, bound from the standing."""
  _, _, root = born()
  assert STANDS[2] == "m/low"
  assert engine.module(root)["actor"] == "m/low"
  assert await engine.rung("close(actor)", on=root) == "m/low"


async def test_the_program_rebinds_actor_like_any_name_and_the_last_binding_wins() -> None:
  """The program rebinds actor like any name, and the last binding wins."""
  _, _, root = born()
  await engine.rung("actor = 'n/low'", on=root)
  assert engine.module(root)["actor"] == "n/low"
  await engine.rung("actor = 'm/high'", on=root)
  assert engine.module(root)["actor"] == "m/high"
  assert await engine.rung("close(actor)", on=root) == "m/high"
