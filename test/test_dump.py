"""dump, the names of the module of a chain as python."""

import pytest

from conftest import born, said
from furb import engine
from furb.engine import Refused

VALUES = "k = [1, 'x', {2: (3,)}, {4.5}, (None,)]\nt = Text('a.txt', 'one')\ne = ValueError('no')\nx = Exit(0, Text('', 'out'), Text('', ''))"
"""A word that binds a value of each kind that a checkpoint carries."""
DEFINES = "def f(n):\n  return n + 1\n\nclass P:\n  size = 2"
"""A word that defines a function and a class."""


async def test_dump_gives_names_of_the_module_of_a_chain_as_python_which_a_word_of_another_life_runs_to_bind_them_again() -> (
  None
):
  """dump gives names of the module of a chain as python, which a word of another life runs to bind them again, so no verb loads them."""
  _, _, root = born()
  await engine.rung(VALUES, on=root)
  await engine.rung(DEFINES, on=root)
  text = engine.dump("k", "t", "e", "x", "f", "P", on=root)
  first = engine.module(root)
  held = {name: first[name] for name in ("k", "t", "x")}
  error = repr(first["e"])
  _, _, again = born()
  assert "k" not in engine.module(again)
  await engine.rung(text, on=again)
  await engine.rung("made = (f(1), P.size, P().size)", on=again)
  later = engine.module(again)
  assert {name: later[name] for name in held} == held
  assert repr(later["e"]) == error
  assert later["made"] == (2, 2, 2)


async def test_a_name_whose_value_a_checkpoint_carries_gives_its_name_and_the_literal_of_its_value() -> None:
  """A name whose value a checkpoint carries gives its name, =, and the literal of its value."""
  _, _, root = born()
  await engine.rung(VALUES, on=root)
  assert (
    engine.dump("k", "t", on=root) == "k = [1, 'x', {2: (3,)}, {4.5}, (None,)]\nt = Text(path='a.txt', content='one')"
  )


async def test_a_name_of_a_function_or_of_a_class_gives_the_last_word_that_defines_it() -> None:
  """A name of a function or of a class gives the last word of the program of the chain that defines it at the start of a line, whole, and a word that defines two of the names comes once."""
  _, _, root = born()
  await engine.rung("def f(n):\n  return n", on=root)
  await engine.rung(DEFINES, on=root)
  await engine.rung("def g():\n  def f():\n    return 0\n  return f", on=root)
  assert engine.dump("f", "P", on=root) == DEFINES
  assert engine.dump("g", "f", on=root) == "def g():\n  def f():\n    return 0\n  return f\n" + DEFINES


async def test_a_name_of_a_function_or_of_a_class_whose_own_name_is_another_raises_refused() -> None:
  """A name of a function or of a class whose own name is another, as after an import of another function as f, raises Refused, since no word of the program defines its value under that name."""
  _, _, root = born()
  await engine.rung("def f():\n  return 1\n\n\ndef g():\n  return 2", on=root)
  await engine.rung("from math import sqrt as f", on=root)
  with pytest.raises(Refused, match="no dump of f"):
    engine.dump("f", on=root)
  assert engine.dump("g", on=root).startswith("def f():")


async def test_a_name_of_an_act_or_of_a_value_that_holds_one_raises_refused_which_names_it() -> None:
  """A name of an Act, or of a value that holds one, raises Refused, which names it, since an act is of its life alone, so dump never writes a live Act."""
  _, _, root = born()
  await engine.rung("shell = bash('echo one')\nheld = {'one': [shell]}\nnamed = shell[:]", on=root)
  for name in ("shell", "held"):
    with pytest.raises(Refused, match=f"^no dump of {name}$"):
      engine.dump(name, on=root)
  assert engine.dump("named", on=root) == f"named = {str(engine.module(root)['shell'])!r}"


async def test_a_name_that_dump_cannot_write_raises_refused_which_names_it() -> None:
  """A name of a value that literal cannot write, a name of a function or a class that no word of the program defines, and a name that the module does not hold raise Refused, which names it, so no name is lost in silence."""
  _, _, root = born()
  await engine.rung("gone = iter([1])\nmany = [1, iter([2])]\nsquare = lambda n: n * n", on=root)
  for name in ("gone", "many", "square", "read", "nothing"):
    with pytest.raises(Refused, match=f"^no dump of {name}$"):
      engine.dump(name, on=root)


async def test_dump_reads_the_module_and_the_program_of_the_chain_and_asks_nothing() -> None:
  """dump reads the module and the program of the chain and asks nothing, so no act is made and the journal keeps nothing."""
  sand, log, root = born()
  await engine.rung(VALUES, on=root)
  facts, kept = len(log), len(sand.record)
  assert engine.dump("k", on=root)
  assert (len(log), len(sand.record)) == (facts, kept)
  assert said(log, "dump") == []
