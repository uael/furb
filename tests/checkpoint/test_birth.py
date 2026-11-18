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


async def test_birth_makes_each_chain_that_jumped_go_on_as_the_chain_that_it_went_on_as_when_it_hears_the_fact_jumped_that_seed_gives_of_it() -> (
  None
):
  """birth makes each chain that jumped go on as the chain that it went on as when it hears the fact jumped that seed gives of it."""
  held = {"turns": [], "definitions": [], "values": {}, "lost": [], "cut": [], "paused": False}
  went = {"id": "chain2", "on": "", "label": "root", "source": "chain1", **held}
  root = {"id": "chain1", "by": "operator", "on": "", "label": "root", "source": "", "to": "chain2"}
  stand = {"kind": "stand", "id": "stand1", "on": "chain2", "value": STANDS}
  data = {"taken": {"chain": 2, "stand": 1}, "chains": [went], "acts": [stand], "jumped": [root]}
  life(sown())
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  assert (engine.scope("chain1"), engine.get("chain1")[:4]) == ("chain2", ("chain", "chain1", "operator", ""))
