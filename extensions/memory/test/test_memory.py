"""memory, the memory of a path, told to its chain."""

from pathlib import Path

import pytest

from conftest import STANDS, Dead, life
from extensions.conftest import extended, memorized, noted, recalled
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty


async def test_the_memory_of_a_path_told_to_its_chain(tmp_path: Path) -> None:
  """The memory of a path, told to its chain: each memory file that applies to the path and that the chain does not hold as it stands, read with the show of every line, so the chain holds the whole file, and told whole."""
  text = "".join(f"{n}\n" for n in range(1, 2101))
  top = noted(tmp_path / "work", text)
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("memory()", on=root)
  assert recalled(root, tmp_path) == [memorized("memory1", top, text)]
  top.write_text(text.replace("\n5\n", "\nfive\n"))
  await engine.rung("memory()", on=root)
  assert recalled(root, tmp_path)[1:] == [memorized("memory2", top, text.replace("\n5\n", "\nfive\n"))]


async def test_memory_asks_the_world_a_memory_question_and_tells_each_text(tmp_path: Path) -> None:
  """memory asks the World a memory question on its chain with the path, and tells each text of the answer in its order, in a paragraph headed with the name of the question, which binds the path of the text and the text, with the show of every line."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sub = noted(tmp_path / "work" / "sub", "three\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung('memory("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == [memorized("memory1", top, "one\ntwo\n"), memorized("memory1", sub, "three\n")]


async def test_memory_gives_the_texts_of_the_answer(tmp_path: Path) -> None:
  """memory gives the texts of the answer, and an empty list when the chain holds every memory file that applies."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  got = await engine.rung("close(memory())", on=root)
  assert isinstance(got, list)
  assert [one for one in got if one.path.startswith(str(tmp_path))] == [Text(str(top), "one\n")]
  assert await engine.rung("close(memory())", on=root) == []


async def test_a_memory_question_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A memory question that the World refuses raises Refused in the caller."""
  one = next(one for one in _monty.official() if one["name"] == "memory")
  _, root = life(Dead(stands=STANDS), extensions=_monty.extensions([{**one, "life": ""}]))
  with pytest.raises(Refused, match="a dead World answers no memory"):
    await engine.rung("close(memory())", on=root)
