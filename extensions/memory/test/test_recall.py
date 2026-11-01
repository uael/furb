"""recall, the memory of a chain, read where its word runs."""

from pathlib import Path

from conftest import chained, of
from extensions.conftest import extended, memorized, noted, recalled, tilded
from furb import engine
from furb_monty import _monty


async def test_the_memory_of_a_chain_read_where_its_word_runs(tmp_path: Path) -> None:
  """The memory of a chain, read where its word runs: recall asks the World a memory question for the working directory of its chain, then reads the path of each text of the answer, in its order, so the turn after the word that runs it shows each memory file as every read tells it."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  ran = engine.rung("await recall()", on=root)
  await ran
  reads = [a for a in engine.transcript(root) if a[0] == "read" and a[2] == ran]
  assert [a[4] for a in reads if str(a[4]).startswith(tilded(tmp_path))] == [tilded(top)]
  assert f"_path = {tilded(top)!r}" in "\n".join(of(engine.turns(on=root), "read"))
  assert tilded(sub) not in "\n".join(of(engine.turns(on=root), "read"))
  assert recalled(root, tmp_path) == []


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
  """The life word of the extension is await recall(), so the setup of the root reads its memory in the turn of the life words of the extensions, and each chain with no source holds those reads in its setup."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True, doctrine="Read first.\n")
  two = await chained("two")
  for on in (root, two):
    turns = engine.turns(on=on)
    (at,) = [n for n, one in enumerate(turns) if one[:2] == ("assistant", "await recall()")]
    assert f"_path = {tilded(top)!r}" in turns[at + 1][1]
