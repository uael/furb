"""Memory, the question of the memory files that apply to a path."""

from pathlib import Path

from conftest import settle
from extensions.conftest import extended, noted, recalled
from furb import engine
from furb.engine import Text
from furb_monty import _monty


async def paths(root: str, at: Path, path: str) -> list[str]:
  """The paths of the memory of a path on a chain, under a directory, since a folder above it belongs to the machine."""
  got = await engine.rung(f"close([one.path for one in memory({path!r})])", on=root)
  assert isinstance(got, list)
  return [one for one in got if one.startswith(str(at))]


async def test_a_memory_is_the_question_of_the_memory_files_that_apply_to_a_path(tmp_path: Path) -> None:
  """A memory is the question of the memory files that apply to a path, which carries the path, and which the World answers with a list of texts."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung('memory("sub")', on=root)
  (asked,) = [a for a in engine.transcript(on=root) if a[0] == "memory"]
  assert asked == ("memory", "memory1", "rung2", root, "sub")
  got = engine.peek("memory1")
  assert isinstance(got, list)
  assert [one for one in got if one.path.startswith(str(tmp_path))] == [Text(str(top), "one\n")]


async def test_the_world_answers_with_the_memory_of_the_user_first(tmp_path: Path) -> None:
  """The World answers with the memory of the user first, then of each folder from the root of the file system down to the working directory of the chain, then of each folder under it down to the folder of the path, when the path stands under it."""
  user = noted(tmp_path / "config", "u\n")
  up = noted(tmp_path, "p\n")
  top = noted(tmp_path / "work", "t\n")
  deep = noted(tmp_path / "work" / "a" / "b", "d\n")
  noted(tmp_path / "other", "o\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  assert await paths(root, tmp_path, "a/b/c.txt") == [str(user), str(up), str(top), str(deep)]
  assert await paths(root, tmp_path, "../other/c.txt") == []


async def test_the_memory_of_a_folder_is_its_claude_md_file_or_its_agents_md_file(tmp_path: Path) -> None:
  """The memory of a folder is its CLAUDE.md file, or its AGENTS.md file when the folder holds no CLAUDE.md, and the memory of the user is that of the config directory of the user."""
  user = noted(tmp_path / "config", "u\n", "AGENTS.md")
  top = noted(tmp_path / "work", "t\n")
  noted(tmp_path / "work", "a\n", "AGENTS.md")
  sub = noted(tmp_path / "work" / "sub", "s\n", "AGENTS.md")
  noted(tmp_path / "work" / "low", "l\n", "claude.md")
  _, root = extended("memory", tmp_path, _monty.memory)
  assert await paths(root, tmp_path, "low/c.txt") == [str(user), str(top)]
  assert await paths(root, tmp_path, "sub/c.txt") == [str(sub)]


async def test_the_world_leaves_out_a_memory_file_whose_content_the_chain_holds(tmp_path: Path) -> None:
  """The World leaves out a memory file whose content the chain holds: one that a memory question told it, or that it read or wrote, with that content, its prefix among them."""
  top = noted(tmp_path / "work", "t\n")
  sub = noted(tmp_path / "work" / "sub", "s\n")
  noted(tmp_path / "work" / "other", "o\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  await engine.rung('memory()\nread("sub/CLAUDE.md")\nwrite(Text("other/CLAUDE.md", "o\\n"))', on=root)
  assert (await paths(root, tmp_path, "sub/c.txt"), await paths(root, tmp_path, "other/c.txt")) == ([], [])
  two = engine.chain("two", root)
  await settle()
  assert await paths(two, tmp_path, "other/c.txt") == []
  top.write_text("t2\n", encoding="utf-8")
  assert await paths(root, tmp_path, ".") == [str(top)]
  # A question from outside an act tells nothing, so the chain holds nothing of it.
  three = engine.chain("three")
  await settle()
  engine.ask("memory", three, "sub/c.txt")
  engine.read("sub/CLAUDE.md", on=three)
  assert await paths(three, tmp_path, "sub/c.txt") == [str(top), str(sub)]
  # A read in the step whose done the watcher hears first is held all the same.
  (sub.parent / "c.txt").write_text("c\n", encoding="utf-8")
  four = engine.chain("four")
  await settle()
  await engine.rung("remember()", on=four)
  await engine.rung('read("sub/c.txt")\nread("sub/CLAUDE.md")', on=four)
  assert [one.split("\n")[1] for one in recalled(four, tmp_path)] == [f"memory8_path = {str(top)!r}"]


async def test_a_path_of_a_scheme_adds_no_folder_of_its_own(tmp_path: Path) -> None:
  """A path of a scheme adds no folder of its own."""
  top = noted(tmp_path / "work", "t\n")
  noted(tmp_path / "work" / "skills:", "s\n")
  _, root = extended("memory", tmp_path, _monty.memory)
  assert await paths(root, tmp_path, "skills://brew") == [str(top)]
