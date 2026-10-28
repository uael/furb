"""recall, the memory of a chain as turns of its model."""

from pathlib import Path

from conftest import settle
from extensions.conftest import extended, memorized, noted, recalled
from furb import engine
from furb_monty import _monty


def crafted(on: str) -> list[list]:
  return [list(one[:2]) for one in engine.turns(on=on)]


async def test_the_memory_of_a_chain_as_turns_of_its_model(tmp_path: Path) -> None:
  """The memory of a chain as turns of its model: recall asks the World a memory question for the working directory of its chain, and runs a read of the path of each text of the answer, in its order, as one rung of the default actor of the chain, so the turns show the model that reads each memory file, then each read as every read tells it."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("await recall()", on=root)
  turns = crafted(root)
  at = turns.index(["assistant", f"read({str(top)!r})"])
  assert turns[at + 1][0] == "user"
  assert f"_path = {str(top)!r}" in turns[at + 1][1]
  assert recalled(root, tmp_path) == []


async def test_recall_awaits_that_rung(tmp_path: Path) -> None:
  """recall awaits that rung, and runs no rung when the chain holds every memory file that applies."""
  noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  outer = engine.rung("await recall()", on=root)
  await outer
  (inner,) = [a[1] for a in engine.transcript(root) if a[0] == "rung" and a[2] == outer]
  done = [a[1] for a in engine.transcript(root) if a[0] == "done"]
  assert done.index(inner) < done.index(outer)
  again = engine.rung("await recall()", on=root)
  await again
  assert [a for a in engine.transcript(root) if a[0] == "rung" and a[2] == again] == []


async def test_recall_then_starts_remember_on_its_chain_and_gives_it(tmp_path: Path) -> None:
  """recall then starts remember on its chain and gives it, so the chain keeps its memory from there, and remember tells nothing of a file that the reads hold, since the World leaves out what the chain holds."""
  noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  (sub.parent / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung("close(await recall())", on=root)
  assert isinstance(watcher, str)
  assert (engine.get(watcher)[0], recalled(root, tmp_path)) == ("remember", [])
  await engine.rung('read("sub/a.txt")', on=root)
  assert [one.split("\n")[1:] for one in recalled(root, tmp_path)] == [
    memorized("memory3", sub, "two\n").split("\n")[1:]
  ]


async def test_the_life_word_of_the_extension_is_await_recall(tmp_path: Path) -> None:
  """The life word of the extension is await recall(), so a chain reads its memory when the life enables the extension, and a chain born later reads it at its birth, before its first prompt asks a model."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True)
  two = engine.chain("two")
  engine.prompt(None, "go", on=two)
  await settle()
  turn = ["assistant", f"read({str(top)!r})"]
  turns = crafted(two)
  asked = next(at for at, one in enumerate(turns) if "#prompt1" in one[1])
  assert (turn in crafted(root), turns.index(turn) < asked) == (True, True)
