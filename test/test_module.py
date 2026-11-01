"""module, the globals of a chain."""

from conftest import born
from furb import engine


async def test_module_gives_the_globals_of_a_chain() -> None:
  """module gives the globals of a chain: the dict that the last module of its transcript carries, in which every rung of the chain runs."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  last = [a for a in engine.transcript(root) if a[0] == "module"][-1]
  held = engine.module(root)
  assert held == last[3] and held["k"] == 1 and held["__name__"] == root


async def test_a_chain_that_holds_no_module_gives_an_empty_dict() -> None:
  """A chain that holds no module gives an empty dict."""
  born()
  assert engine.module("chain9") == {}
