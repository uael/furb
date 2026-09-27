"""remember, the memory of a chain, kept."""

from asyncio import CancelledError
from pathlib import Path

from conftest import extended, heads, noted, plain, recalled, settle
from furb import engine
from furb_monty import _monty


async def test_the_memory_of_a_chain_kept(tmp_path: Path) -> None:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read on the chain gives, and the memory that changed at each stand, and that never completes."""
  top = noted(tmp_path / "work", "one\n")
  sub = noted(tmp_path / "work" / "sub", "two\n")
  (tmp_path / "work" / "sub" / "a.txt").write_text("a\n", encoding="utf-8")
  _, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  assert recalled(root, tmp_path) == [f"#memory {top}\n# {top}, 0 known\n# 1 one"]
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path)[1:] == [f"#memory {sub}\n# {sub}, 0 known\n# 1 two"]
  top.write_text("three\n", encoding="utf-8")
  engine.stand()
  assert recalled(root, tmp_path)[2:] == [f"#memory {top}\n# {top}, 0 known\n# 1 three"]
  assert engine.peek(watcher, "living") == "living"


async def test_a_cancel_over_it_ends_it_and_a_pause_over_it_holds_it_until_the_wake(tmp_path: Path) -> None:
  """A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake."""
  sub = noted(tmp_path / "work" / "sub", "two\n")
  deep = noted(tmp_path / "work" / "deep", "three\n")
  for folder in (sub.parent, deep.parent):
    (folder / "a.txt").write_text("a\n", encoding="utf-8")
  _, root = extended("memory", tmp_path, _monty.memory)
  watcher = await engine.rung("close(remember())", on=root)
  assert isinstance(watcher, str)
  engine.pause(watcher)
  await engine.rung('read("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == []
  engine.wake(watcher)
  assert recalled(root, tmp_path) == [f"#memory {sub}\n# {sub}, 0 known\n# 1 two"]
  engine.cancel(watcher)
  await engine.rung('read("deep/a.txt")', on=root)
  assert isinstance(engine.peek(watcher), CancelledError)
  assert len(recalled(root, tmp_path)) == 1


async def test_the_life_word_of_the_extension_is_remember(tmp_path: Path) -> None:
  """The life word of the extension is remember(), so a chain tells its memory when the life enables the extension, and a chain born later tells it at its birth, before its first prompt asks a model."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True)
  two = engine.chain("two")
  engine.prompt(None, "go", on=two)
  await settle()
  told = f"#memory {top}\n# {top}, 0 known\n# 1 one"
  assert (recalled(root, tmp_path), recalled(two, tmp_path)) == ([told], [told])
  said = heads(engine.turns(on=two))
  assert said.index(f"#memory {top}") < said.index("#prompt1 go")


async def test_a_memory_file_enters_a_chain_once_and_again_only_when_it_changed(tmp_path: Path) -> None:
  """A memory file enters a chain once, and again only when it changed, and then only its lines that changed, since the World leaves out what the chain holds and a text tells only the lines that the chain has not seen."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True)
  engine.stand()
  await engine.rung("memory()", on=root)
  top.write_text("one\nthree\n", encoding="utf-8")
  engine.stand()
  assert recalled(root, tmp_path) == [
    f"#memory {top}\n# {top}, 0 known\n# 1 one\n# 2 two",
    f"#memory {top}\n# {top}, 1 known\n# 2 three",
  ]


async def test_a_later_life_tells_what_the_record_holds(tmp_path: Path) -> None:
  """A later life tells what the record holds, since the journal answers each memory question that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sand, root = extended("memory", tmp_path, _monty.memory, lives=True)
  first = recalled(root, tmp_path)
  top.write_text("one\nthree\n", encoding="utf-8")
  _, root = extended("memory", tmp_path, _monty.memory, lives=True, record=plain(sand.record))
  await settle()
  assert recalled(root, tmp_path) == [*first, f"#memory {top}\n# {top}, 1 known\n# 2 three"]
  assert first == [f"#memory {top}\n# {top}, 0 known\n# 1 one\n# 2 two"]
