"""recall, the memory of a chain, read where its word runs."""

from pathlib import Path

from conftest import chained
from furb import engine
from skills.conftest import extended, memorized, noted, recalled, tilded


async def test_the_memory_of_a_chain_read_where_its_word_runs(tmp_path: Path) -> None:
  """The memory of a chain, read where its word runs: recall starts remember on its chain and gives it, so the turn after the word that runs it shows each memory file of the working directory of the chain as its read tells it, and the chain keeps its memory from there."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  (sub.parent / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path)
  watcher = await engine.rung("close(await recall())", on=root)
  assert isinstance(watcher, str)
  assert engine.get(watcher)[0] == "remember"
  assert recalled(root, tmp_path) == [memorized(top, "one\n")]
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path)[1:] == [memorized(sub, "two\n")]


async def test_the_life_word_of_the_extension_is_await_recall(tmp_path: Path) -> None:
  """The life word of the extension is await recall(), so the setup of the root reads its memory in the turn of the life words of the extensions, and each chain with no source holds those reads in its setup."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, lives=True, doctrine="Read first.\n")
  two = await chained("two")
  for on in (root, two):
    turns = engine.turns(on=on)
    (at,) = [n for n, one in enumerate(turns) if one[:2] == ("assistant", "await recall()")]
    assert f"_path = {tilded(top)!r}" in turns[at + 1][1]
