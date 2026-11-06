"""defined, the Show that selects the definitions of a text by name."""

from pathlib import Path

from conftest import of
from extensions.conftest import extended, skilled
from furb import engine
from furb.engine import Text
from furb_monty import _monty

SOURCE = "LIMIT = 3\n\n@cached\ndef one() -> int:\n  return 1\n\nclass Two:\n  pass\nasync def three(\n  x,\n):\n  return x\ntype Alias = int\n"
"""SOURCE is a text of five definitions: an assignment, a decorated def, a class, an async def whose head takes three lines, and a type."""

WORD = "LIMIT = 3\n\n\ndef one() -> int:\n  return 1\n\n\ndef two() -> int:\n  return 2\n"
"""WORD is the file of the word of the skill note."""


async def test_defined_selects_the_lines_of_each_definition_by_name(tmp_path: Path) -> None:
  """defined(*names) is a Show: it selects the lines of each definition at the top of a text whose name is one of names, a definition being a def, an async def, a class, a type or an assignment, with its decorators and each line under it that is indented, blank or closes a bracket."""
  _, root = extended("skills", tmp_path, _monty.skills)
  for names, lines in (('"LIMIT", "one", "three"', [1, 3, 4, 5, 9, 10, 11, 12]), ('"Alias"', [13]), ('"none"', [])):
    got = await engine.rung(f"close(shown(Text('p', {SOURCE!r}), defined({names})))", on=root)
    assert got == "\n".join(SOURCE.splitlines()[n - 1] for n in lines)


async def test_a_read_with_defined_tells_those_definitions_and_gives_the_whole_word(tmp_path: Path) -> None:
  """A read of skills:// and the name of a skill with defined(*names) tells only those definitions and gives the whole word, so a word that reads a skill with it edits the whole word."""
  head = "name: note\ndescription: A note.\nmetadata:\n  furb-word: note.py"
  folder = skilled(tmp_path / "work" / ".furb" / "skills", "note", head).parent
  (folder / "note.py").write_text(WORD, encoding="utf-8", newline="")
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung('close(read("skills://note", defined("one")))', on=root)
  assert got == Text("skills://note", WORD.rstrip("\n"))
  told = of(engine.turns(on=root), "read")[-1]
  assert "def one" in told and "def two" not in told and "LIMIT" not in told
