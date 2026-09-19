"""modules, the module of every chain of the life."""

from conftest import STANDS, Sand, life, settle
from furb import engine


async def test_modules_holds_the_module_of_every_chain_of_the_life_by_the_name_of_the_chain() -> None:
  """modules holds the module of every chain of the life, by the name of the chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  await settle()
  assert sorted(engine.modules) == sorted([root, two])
  assert engine.modules[root]["__name__"] == root and engine.modules[two]["__name__"] == two
  await engine.rung("k = 1", on=two)
  assert engine.modules[two]["k"] == 1 and "k" not in engine.modules[root]


async def test_modules_holds_the_module_of_every_chain_of_the_life_and_boot_empties_it() -> None:
  """modules holds the module of every chain of the life, and boot empties it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  engine.chain("two")
  await settle()
  assert len(engine.modules) == 2
  _, over = life(Sand(stands=STANDS))
  await settle()
  assert list(engine.modules) == [over] and over == root
