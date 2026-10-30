"""remember, the memory of a chain, kept."""

from asyncio import CancelledError
from pathlib import Path

from conftest import paragraphs, plain, settle
from extensions.conftest import extended, memorized, noted, recalled, tilded
from furb import engine
from furb_monty import _monty


async def test_the_memory_of_a_chain_kept(tmp_path: Path) -> None:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read made on the chain after it gives, and the memory that changed at each stand, and that never completes."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  (tmp_path / "work" / "sub" / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  sand, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  assert recalled(root, tmp_path) == [memorized("memory1", top, "one\n")]
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path)[1:] == [memorized("memory2", sub, "two\n")]
  sand.files["note://a"] = "a\n"
  asked = [a for a in engine.transcript(on=root) if a[0] == "memory"]
  await engine.rung('read("note://a")', on=root)
  assert [a for a in engine.transcript(on=root) if a[0] == "memory"] == asked, "a path of a scheme is no folder"
  top.write_text("three\n", encoding="utf-8", newline="")
  engine.stand()
  assert recalled(root, tmp_path)[2:] == [memorized("memory3", top, "three\n")]
  assert engine.peek(watcher, "living") == "living"


async def test_a_cancel_over_it_ends_it_and_a_pause_over_it_holds_it_until_the_wake(tmp_path: Path) -> None:
  """A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake."""
  sub = noted(tmp_path / "work" / "sub", "two\n")
  deep = noted(tmp_path / "work" / "deep", "three\n")
  for folder in (sub.parent, deep.parent):
    (folder / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  engine.pause(watcher)
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == []
  engine.wake(watcher)
  assert recalled(root, tmp_path) == [memorized("memory2", sub, "two\n")]
  engine.cancel(watcher)
  await engine.rung('read("deep/a.txt")', on=root)
  assert isinstance(engine.peek(watcher), CancelledError)
  assert len(recalled(root, tmp_path)) == 1


async def test_remember_asks_nothing_of_a_read_made_before_it(tmp_path: Path) -> None:
  """remember asks nothing of a read made before it, whose done a later life can say before or after the birth of remember, so a life and its replay ask the same memory questions."""
  noted(tmp_path / "work" / "sub", "two\n")
  (tmp_path / "work" / "sub" / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung('read("sub/a.txt")\nclose(remember())', on=root)
  assert isinstance(watcher, str)
  await settle()
  assert [a[4] for a in engine.transcript(on=root) if a[0] == "memory"] == ["."]
  await engine.rung('read("sub/a.txt")', on=root)
  assert [a[4] for a in engine.transcript(on=root) if a[0] == "memory"] == [
    ".",
    tilded(tmp_path / "work" / "sub" / "a.txt"),
  ]


async def test_a_memory_file_enters_a_chain_once_and_again_only_when_it_changed(tmp_path: Path) -> None:
  """A memory file enters a chain once, and again only when it changed, by the lines that changed, since the World leaves out what the chain holds."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("remember()", on=root)
  engine.stand()
  await engine.rung("memory()", on=root)
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  engine.stand()
  assert recalled(root, tmp_path) == [
    memorized("memory1", top, "one\ntwo\n"),
    f"#memory4\nmemory4_path = {tilded(top)!r}\nmemory4_text = 'three'",
  ]


async def test_a_later_life_tells_what_the_record_holds(tmp_path: Path) -> None:
  """A later life tells what the record holds, since the journal answers each memory question that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sand, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("remember()", on=root)
  first = recalled(root, tmp_path)
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, _monty.memory, record=plain(sand.record))
  await settle()
  assert recalled(root, tmp_path) == [*first, f"#memory2\nmemory2_path = {tilded(top)!r}\nmemory2_text = 'three'"]
  assert first == [memorized("memory1", top, "one\ntwo\n")]


async def test_at_a_stand_the_catch_up_of_the_chain_tells_each_memory_file_that_a_read_or_a_write_gave(
  tmp_path: Path,
) -> None:
  """At a stand the catch-up that the extension catchup makes before remember tells each memory file that a read or a write on the chain gave, so remember tells again only a memory file that a memory question alone gave the chain, since the World leaves out the content that the catch-up read."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sub = noted(tmp_path / "work" / "sub", "sub\n")
  (sub.parent / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True, also=("catchup",))
  await engine.rung(f"read({str(top)!r})", on=root)
  await engine.rung('read("sub/a.txt")', on=root)
  was = recalled(root, tmp_path)
  assert was == [memorized("memory4", sub, "sub\n")]
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  sub.write_text("sub!\n", encoding="utf-8", newline="")
  engine.stand()
  caught = [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#chain1 caught up")]
  assert caught == [f"#chain1 caught up\nchain1_changed = {{{tilded(top)!r}: {{2: 'three'}}}}"]
  assert recalled(root, tmp_path)[1:] == [memorized("memory6", sub, "sub!\n")]


async def test_it_tells_the_memory_that_changed_at_each_stand_from_outside_an_act(tmp_path: Path) -> None:
  """It tells the memory that changed at each stand from outside an act, as the stand that boot makes at the tip of a life, and at no stand that a word makes, as the one that the World plays on each new chain."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung("remember()", on=root)
  asked = [a for a in engine.transcript(on=root) if a[0] == "memory"]
  top.write_text("three\n", encoding="utf-8", newline="")
  await engine.rung("stand()", on=root)
  assert [a for a in engine.transcript(on=root) if a[0] == "memory"] == asked
  assert recalled(root, tmp_path) == [memorized("memory1", top, "one\n")]
  engine.stand()
  assert recalled(root, tmp_path)[1:] == [memorized("memory2", top, "three\n")]
