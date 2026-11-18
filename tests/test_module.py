"""module, the globals of a chain."""

from conftest import born
from furb import engine
from furb.engine import Text


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


async def test_the_module_of_a_rung_of_a_text_is_the_module_of_the_skills_of_its_chain() -> None:
  """The module of a rung of a text, and of a rung that retells one, is the module of the extensions of its chain, a dict that the module of the chain holds under __extensions__ and makes at its first use as a copy of the names of the engine, so what a word of the chain binds never reaches it."""
  _, _, root = born()
  await engine.rung('read = "x"\nk = 1', on=root)
  kit = await engine.rung(Text("kit.py", "j = 2"), on=root)
  assert kit is None
  (made,) = [a[1] for a in engine.transcript(root, "rung") if a[4] == Text("kit.py", "j = 2")]
  held = engine.module(made)
  assert (held["j"], callable(held["read"]), "k" in held) == (2, True, False)
  assert engine.module(root)["read"] == "x" and engine.module(root)["__extensions__"]["j"] == 2
