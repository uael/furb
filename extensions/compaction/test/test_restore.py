"""restore, the module of a chain bound again."""

from pathlib import Path

from conftest import chained, settle
from extensions.conftest import compacting
from furb import engine


async def test_restore_binds_in_the_chain_a_copy_of_each_name_that_the_module_of_the_source_holds(
  tmp_path: Path,
) -> None:
  """restore(source) binds in the chain a copy of each name that the module of the source holds, but a callable, a name that starts with two underscores and a name that holds the object that the chain holds already, so the chain holds what a word of the source bound or changed, though that word is not over."""
  _, root, _ = compacting(tmp_path)
  await engine.rung("items = [1]\ndef f():\n  return 1", on=root)
  engine.rung("items.append(2)\nk = 3\nn = await prompt(int, 'n', to=OPERATOR)", on=root)
  await settle()
  two = await chained("two", root)
  assert (engine.module(two)["items"], "k" in engine.module(two)) == ([1], False)
  await engine.rung("restore('chain1')\nmine = f()", on=two)
  held = engine.module(two)
  assert (held["items"], held["k"], held["__name__"], held["mine"]) == ([1, 2], 3, two, 1)
  assert "n" not in held


async def test_the_chain_owns_what_restore_binds(tmp_path: Path) -> None:
  """The chain owns what restore binds: it copies each list, dict and set, and what each holds, and shares every other object."""
  _, root, _ = compacting(tmp_path)
  engine.rung("held = {'a': [1, {2}], 't': Text('p', 'x')}\nn = await prompt(int, 'n', to=OPERATOR)", on=root)
  await settle()
  two = await chained("two", root)
  await engine.rung("restore('chain1')", on=two)
  await settle()
  word = "held['a'][1].add(3)\nfresh = held['a'] is not module('chain1')['held']['a']\nsame = held['t'] is module('chain1')['held']['t']"
  await engine.rung(word, on=two)
  assert engine.module(root)["held"]["a"][1] == {2} and engine.module(two)["held"]["a"][1] == {2, 3}
  assert (engine.module(two)["fresh"], engine.module(two)["same"]) == (True, True)


async def test_a_rung_of_restore_binds_each_name_that_the_chain_did_not_hold(tmp_path: Path) -> None:
  """A rung of restore binds each name that it binds and that the chain did not hold to its value in the globals of the chain, so the gate reads a word of the chain that names it."""
  _, root, _ = compacting(tmp_path)
  await engine.rung("j = 4", on=root)
  engine.rung("k = 3\nn = await prompt(int, 'n', to=OPERATOR)", on=root)
  await settle()
  two = await chained("two", root)
  await engine.rung("restore('chain1')", on=two)
  await settle()
  declared = [a[4] for a in engine.transcript(two) if a[0] == "rung" and a[2] == two and str(a[4]).startswith("k = ")]
  assert declared == ["k = globals()['k']"]
  await engine.rung("total = k + j", on=two)
  assert engine.module(two)["total"] == 7
