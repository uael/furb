"""memory, the memory of a path, told to its chain."""

from pathlib import Path

import pytest

from conftest import STANDS, Dead, extended, life, noted, recalled
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty


async def test_the_memory_of_a_path_told_to_its_chain(tmp_path: Path) -> None:
  """The memory of a path, told to its chain: each memory file that applies to the path and that the chain does not hold as it stands, told whole."""
  top = noted(tmp_path / "work", "".join(f"{n}\n" for n in range(1, 2101)))
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("memory()", on=root)
  (told,) = recalled(root, tmp_path)
  assert told.splitlines()[:3] == [f"#memory {top}", f"# {top}, 0 known", "# 1 1"]
  assert told.splitlines()[-1] == "# 2100 2100"


async def test_memory_asks_the_world_a_memory_question_and_tells_each_text(tmp_path: Path) -> None:
  """memory asks the World a memory question on its chain with the path, and tells each text of the answer in its order, under the header memory and the path of the text, with every line of it."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sub = noted(tmp_path / "work" / "sub", "three\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung('memory("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == [
    f"#memory {top}\n# {top}, 0 known\n# 1 one\n# 2 two",
    f"#memory {sub}\n# {sub}, 0 known\n# 1 three",
  ]


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
  name, word, _ = next(one for one in _monty.official() if one[0] == "memory")
  _, root = life(Dead(stands=STANDS), extensions=_monty.extensions([(name, word, "")]))
  with pytest.raises(Refused, match="a dead World answers no memory"):
    await engine.rung("close(memory())", on=root)
