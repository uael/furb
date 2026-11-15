"""birth, the ear that binds the module of each chain that seed makes again."""

from conftest import STANDS, life, settle, sown
from furb import checkpoint, engine


async def test_birth_gives_the_ear_of_the_outside_that_binds_the_module_of_each_chain_that_seed_makes_again() -> None:
  """birth gives the ear of the outside that binds the module of each chain that seed makes again, which a host gives boot under the name birth, so the engine binds no module from data of its own."""
  held = {"turns": [], "definitions": [], "values": {"SIZE": "6"}, "lost": [], "cut": [], "paused": False}
  root = {"id": "chain1", "on": "", "label": "root", "source": "", **held}
  stand = {"kind": "stand", "id": "stand1", "on": "chain1", "value": STANDS}
  life(sown())
  life(sown(), checkpoint.seed(vars(engine), {"taken": {"chain": 1, "stand": 1}, "chains": [root], "acts": [stand]}))
  await settle()
  assert engine.module("chain1")["SIZE"] == 6 and "birth" in engine.listening()
