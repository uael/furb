"""literal, the python that makes a value that a checkpoint carries again."""

import ast

from conftest import born
from furb import engine
from furb.engine import Exit, Text


async def test_literal_gives_the_python_that_makes_a_value_again() -> None:
  """literal gives the python that makes a value again when the value is one that a checkpoint carries: None, a bool, an int, a float, a str, a Text, an Exit or an exception as python shows it, and a list, a tuple, a set or a dict of those as python writes it."""
  born()
  value = {"one": [None, True, 1, 2.5, "x", Text("a", "b"), Exit(0, Text(""), Text("")), (1,), set(), {3}], 2: ()}
  assert engine.literal(value) == repr(value)
  assert engine.literal(KeyError("k")) == "KeyError('k')"


async def test_literal_gives_a_value_that_is_false_for_any_other_value() -> None:
  """literal gives a value that is false for any other value, and for a list, a tuple, a set or a dict that holds one, so a caller learns that the value is lost."""
  _, _, root = born()
  word = (
    "gone = iter([1])\nlost = [literal(x, set()) for x in (gone, [1, gone], (gone,), {'k': gone}, {1: [gone]}, len)]"
  )
  await engine.rung(word, on=root)
  assert not any(engine.module(root)["lost"])


async def test_literal_adds_to_into_each_str_of_the_value_that_names_an_act_of_the_life() -> None:
  """literal adds to into each str of the value that names an act of the life, an Act among them, and writes an Act as Act('bash1')."""
  _, _, root = born()
  word = "shell = bash('echo one')\ninto = set()\ngot = literal([shell, 'nothing', 'chain1'], into)"
  await engine.rung(word, on=root)
  held = engine.module(root)
  shell = str(held["shell"])
  assert held["got"] == f"[Act({shell!r}), 'nothing', 'chain1']"
  assert held["into"] == {shell, "chain1"}


async def test_literal_with_no_into_writes_no_act() -> None:
  """literal with no into writes no Act, so it gives a value that is false for an Act and for a value that holds one."""
  _, _, root = born()
  word = "shell = bash('echo one')\nnamed = literal(shell[:])\nlost = (literal(shell), literal({'k': [shell]}))"
  await engine.rung(word, on=root)
  held = engine.module(root)
  assert held["named"] == repr(str(held["shell"]))
  assert not any(held["lost"])


async def test_literal_gives_a_value_that_is_false_for_a_float_that_is_not_finite_and_for_an_exception_whose_arguments_it_cannot_write_so_a_caller_learns_that_the_value_is_lost() -> (
  None
):
  """literal gives a value that is false for a float that is not finite and for an exception whose arguments it cannot write, so a caller learns that the value is lost."""
  _, _, root = born()
  await engine.rung("lost = [literal(x) for x in (float('nan'), float('inf'), -float('inf'))]", on=root)
  assert not any(engine.module(root)["lost"])
  got = engine.literal(ValueError(len))
  assert not got or ast.parse(got)
  assert engine.literal(1.5) == "1.5"
