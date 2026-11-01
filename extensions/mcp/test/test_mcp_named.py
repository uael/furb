"""mcp_named, the name of the class that a word defines."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine

WORDS = [
  "class a:\n  pass\n",
  "x = 1\nclass b_2(Base):\n  pass\nclass c:\n  pass\n",
  "def f():\n  class d:\n    pass\n",
  "",
]
"""WORDS is a word that defines a class at the start of a line, the first of two, a class in a function, and a word with
none."""


async def test_mcp_named_gives_the_name_of_the_first_class_that_a_word_defines(tmp_path: Path) -> None:
  """mcp_named(word) gives the name of the first class that a word defines at the start of a line, and an empty text when the word defines none."""
  root = mcped(tmp_path, {})[1]
  got = await engine.rung(f"close([mcp_named(x) for x in {WORDS!r}])", on=root)
  assert got == ["a", "b_2", "", ""]
