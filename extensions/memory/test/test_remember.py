"""remember, the memory of a chain, kept."""

from asyncio import CancelledError
from pathlib import Path

from conftest import paragraphs, plain, settle
from extensions.conftest import extended, memorized, noted, recalled, tilded
from furb import engine


def made(by: str, on: str) -> list[str]:
  """The path of each read that an act made on a chain, in order."""
  return [str(a[4]) for a in engine.transcript(on) if a[0] == "read" and a[2] == by]


async def test_the_memory_of_a_chain_kept(tmp_path: Path) -> None:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read of a word on the chain after it gives, and the memory that changed at each stand, and that never completes."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  (tmp_path / "work" / "sub" / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  sand, root = extended("memory", tmp_path)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  assert recalled(root, tmp_path) == [memorized(top, "one\n")]
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path)[1:] == [memorized(sub, "two\n")]
  sand.files["note://a"] = "a\n"
  asked = made(watcher, root)
  await engine.rung('read("note://a")', on=root)
  assert made(watcher, root) == asked, "a path of a scheme is no folder"
  top.write_text("three\n", encoding="utf-8", newline="")
  engine.stand(engine.ROOT)
  assert recalled(root, tmp_path)[2:] == [memorized(top, "three\n")]
  assert engine.peek(watcher, "living") == "living"


async def test_a_cancel_over_it_ends_it_and_a_pause_over_it_holds_it_until_the_wake(tmp_path: Path) -> None:
  """A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake."""
  sub = noted(tmp_path / "work" / "sub", "two\n")
  deep = noted(tmp_path / "work" / "deep", "three\n")
  for folder in (sub.parent, deep.parent):
    (folder / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  engine.pause(watcher)
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == []
  engine.wake(watcher)
  assert recalled(root, tmp_path) == [memorized(sub, "two\n")]
  engine.cancel(watcher)
  await engine.rung('read("deep/a.txt")', on=root)
  assert isinstance(engine.peek(watcher), CancelledError)
  assert len(recalled(root, tmp_path)) == 1


async def test_remember_asks_nothing_of_a_read_made_before_it(tmp_path: Path) -> None:
  """remember asks nothing of a read made before it, whose done a later life can say before or after the birth of remember, so a life and its replay make the same reads."""
  noted(tmp_path / "work" / "sub", "two\n")
  (tmp_path / "work" / "sub" / "a.txt").write_text("a\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path)
  watcher = await engine.rung('read("sub/a.txt")\nclose(remember())', on=root)
  assert isinstance(watcher, str)
  await settle()
  folder = f"{tilded(tmp_path / 'work')}/sub/"
  assert folder not in made(watcher, root)
  await engine.rung('read("sub/a.txt")', on=root)
  assert folder in made(watcher, root)


async def test_remember_asks_nothing_of_a_read_of_a_memory_file_or_of_a_path_that_ends_with_a_slash(
  tmp_path: Path,
) -> None:
  """remember asks nothing of a read of a memory file, or of a path that ends with /, as each read of memory, since it read the folder of each."""
  noted(tmp_path / "work" / "sub", "two\n")
  _, root = extended("memory", tmp_path)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  asked = made(watcher, root)
  await engine.rung('read("sub/AGENTS.md")\nread("sub/")\nmemory("sub/a.txt")', on=root)
  await settle()
  assert made(watcher, root) == asked


async def test_a_memory_file_enters_a_chain_once_and_again_only_when_it_changed(tmp_path: Path) -> None:
  """A memory file enters a chain once, and again only when it changed, by the lines that changed, since memory leaves out what the chain holds."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  _, root = extended("memory", tmp_path)
  await engine.rung("remember()", on=root)
  engine.stand(engine.ROOT)
  await engine.rung("memory()", on=root)
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  engine.stand(engine.ROOT)
  assert recalled(root, tmp_path) == [
    memorized(top, "one\ntwo\n"),
    f"#read\nread_path = {tilded(top)!r}\nread_text = 'three'",
  ]


async def test_it_tells_the_memory_that_changed_at_each_stand_from_outside_an_act(tmp_path: Path) -> None:
  """It tells the memory that changed at each stand from outside an act, as the stand that boot makes at the tip of a life, and at no stand that a word makes, as the one that each new chain plays on itself."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  asked = made(watcher, root)
  top.write_text("three\n", encoding="utf-8", newline="")
  await engine.rung("stand()", on=root)
  assert made(watcher, root) == asked
  assert recalled(root, tmp_path) == [memorized(top, "one\n")]
  engine.stand(engine.ROOT)
  assert recalled(root, tmp_path)[1:] == [memorized(top, "three\n")]


async def test_a_later_life_tells_what_the_record_holds(tmp_path: Path) -> None:
  """A later life tells what the record holds, since the journal answers each read that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sand, root = extended("memory", tmp_path)
  await engine.rung("remember()", on=root)
  first = recalled(root, tmp_path)
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  _, root = extended("memory", tmp_path, record=plain(sand.record))
  await settle()
  assert recalled(root, tmp_path) == [*first, f"#read\nread_path = {tilded(top)!r}\nread_text = 'three'"]
  assert first == [memorized(top, "one\ntwo\n")]


async def test_at_a_stand_the_catch_up_of_the_chain_tells_what_changed_in_each_memory_file_of_the_chain(
  tmp_path: Path,
) -> None:
  """At a stand the catch-up that the extension catchup makes before remember tells what changed in each memory file that a read on the chain gave, the reads of memory among them, so remember tells at a stand only a memory file that is new to the chain, since memory leaves out the content that the catch-up read."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  _, root = extended("memory", tmp_path, lives=True, also=("catchup",))
  assert recalled(root, tmp_path) == [memorized(top, "one\ntwo\n")]
  top.write_text("one\nthree\n", encoding="utf-8", newline="")
  new = noted(tmp_path / "work", "local\n", "AGENTS.local.md")
  engine.stand(engine.ROOT)
  caught = [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#chain1 caught up")]
  assert caught == [f"#chain1 caught up\nchain1_changed = {{{tilded(top)!r}: {{2: 'three'}}}}"]
  assert recalled(root, tmp_path)[1:] == [memorized(new, "local\n")]


async def test_remember_tells_the_memory_of_a_folder_after_a_cd_of_a_word_on_the_chain_into_it(tmp_path: Path) -> None:
  """remember tells the memory of a folder after a cd of a word on the chain into it, as it does after a read of a text in it: the memory of the path that the cd gave."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  _, root = extended("memory", tmp_path)
  await engine.rung("close(remember())", on=root)
  assert recalled(root, tmp_path) == [memorized(top, "one\n")]
  await engine.rung('cd("sub")', on=root)
  assert recalled(root, tmp_path)[1:] == [memorized(sub, "two\n")]


async def test_remember_tells_the_memory_of_the_folder_of_a_text_after_a_write_of_a_word_on_the_chain(
  tmp_path: Path,
) -> None:
  """remember tells the memory of the folder of a text after a write of a word on the chain, as it does after a read of that text."""
  top = noted(tmp_path / "work", "one\n")
  deep = noted(tmp_path / "work" / "deep", "three\n")
  _, root = extended("memory", tmp_path)
  await engine.rung("close(remember())", on=root)
  assert recalled(root, tmp_path) == [memorized(top, "one\n")]
  await engine.rung('write(Text("deep/b.txt", "b\\n"))', on=root)
  assert recalled(root, tmp_path)[1:] == [memorized(deep, "three\n")]
