"""Module, the fact that carries the globals of a chain."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import OPERATOR, Text


async def test_a_module_carries_the_globals_of_a_chain() -> None:
  """A module carries the globals of a chain, which the chain says at its birth and at each replay, so its transcript says which module each of its rungs ran in."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "edit", to=OPERATOR, on=root)
  engine.write(Text(act, "k = 1"), on=root)
  await settle()
  engine.write(Text(act, "k = 2"), on=root)
  await settle()
  made = said(engine.transcript(root), "module")
  assert [(a[1], a[2]) for a in made] == [(root, root)] * 3 and "k" not in made[0][3]
  assert made[1][3]["k"] == 1 and made[2][3]["k"] == 2 and engine.module(root) == made[2][3]
