"""dump, the words that define functions and classes of a chain."""

from pathlib import Path

import pytest

from conftest import born
from furb import engine
from furb.engine import Refused
from skills.conftest import extended

DEFINES = "def f(n):\n  return n + 1\n\nclass P:\n  size = 2"
"""A word that defines a function and a class."""


async def test_dump_gives_the_words_that_define_the_functions_and_the_classes_that_it_names_which_a_word_of_another_life_runs(
  tmp_path: Path,
) -> None:
  """dump gives the words of the program of a chain that define the functions and the classes that it names, as python, which a word of another life runs to define them again."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  await engine.rung(DEFINES, on=root)
  text = await engine.rung("close(dump('f', 'P'))", on=root)
  assert text == DEFINES
  _, _, again = born()
  await engine.rung(str(text), on=again)
  await engine.rung("made = (f(1), P.size, P().size)", on=again)
  assert engine.module(again)["made"] == (2, 2, 2)


async def test_a_name_of_a_function_or_of_a_class_gives_the_last_word_that_defines_it(tmp_path: Path) -> None:
  """A name of a function or of a class gives the last word of the program of the chain that defines it at the start of a line, whole, and a word that defines two of the names comes once."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  await engine.rung("def f(n):\n  return n", on=root)
  await engine.rung(DEFINES, on=root)
  await engine.rung("def g():\n  def f():\n    return 0\n  return f", on=root)
  assert await engine.rung("close(dump('f', 'P'))", on=root) == DEFINES
  both = await engine.rung("close(dump('g', 'f'))", on=root)
  assert both == "def g():\n  def f():\n    return 0\n  return f\n" + DEFINES


async def test_a_name_of_a_function_or_of_a_class_whose_own_name_is_another_raises_refused(tmp_path: Path) -> None:
  """A name of a function or of a class whose own name is another, as after an import of another function as f, raises Refused, since no word of the program defines its value under that name."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  await engine.rung("def f():\n  return 1\n\n\ndef g():\n  return 2", on=root)
  await engine.rung("from math import sqrt as f", on=root)
  with pytest.raises(Refused, match=r"^no dump of f$"):
    await engine.rung("dump('f')", on=root)
  got = await engine.rung("close(dump('g'))", on=root)
  assert str(got).startswith("def f():")


async def test_a_name_that_dump_cannot_write_raises_refused_which_names_it(tmp_path: Path) -> None:
  """A name of a value, a name of a function or of a class that no word of the program defines, and a name that the module does not hold raise Refused, which names it, so no name is lost in silence, and a word writes the repr of a plain value itself."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  await engine.rung("k = [1, 'x']\ngone = iter([1])\nsquare = lambda n: n * n\nshell = bash('echo one')", on=root)
  for name in ("k", "gone", "square", "shell", "read", "nothing"):
    with pytest.raises(Refused, match=f"^no dump of {name}$"):
      await engine.rung(f"dump({name!r})", on=root)
  assert await engine.rung("close(repr(k))", on=root) == "[1, 'x']"


async def test_dump_reads_the_module_and_the_program_of_the_chain_and_asks_nothing(tmp_path: Path) -> None:
  """dump reads the module and the program of the chain and asks nothing, so no act is made and the journal keeps nothing."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  await engine.rung(DEFINES, on=root)
  before = len(engine.transcript(on=root))
  assert await engine.rung("close(dump('f'))", on=root)
  assert {a[0] for a in engine.transcript(on=root)[before:] if engine.question(a)} == {"rung", "gate", "run"}
