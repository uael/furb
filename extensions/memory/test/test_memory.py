"""memory, the memory of a path, told to its chain."""

import sys
from pathlib import Path

import pytest

from conftest import STANDS, Dead, life, settle
from extensions.conftest import extended, memorized, noted, recalled, tilded
from furb import engine
from furb.engine import Text
from furb_monty import _monty


async def paths(root: str, at: Path, path: str) -> list[str]:
  """The paths of the memory of a path on a chain, under a directory or in the config directory, since a folder above
  the directory belongs to the machine."""
  got = await engine.rung(f"close([one.path for one in memory({path!r})])", on=root)
  assert isinstance(got, list)
  return [one for one in got if one.startswith((tilded(at), "config://"))]


async def test_the_memory_of_a_path_told_to_its_chain(tmp_path: Path) -> None:
  """The memory of a path, told to its chain: each memory file that applies to the path and whose content the chain does not hold, told whole when the chain holds no text at its path, and else by the lines that differ from the text the chain holds there, as differs picks them, so the chain holds the whole file."""
  text = "".join(f"{n}\n" for n in range(1, 2101))
  top = noted(tmp_path / "work", text)
  _, root = extended("memory", tmp_path)
  await engine.rung("memory()", on=root)
  assert recalled(root, tmp_path) == [memorized(top, text)]
  top.write_text(text.replace("\n5\n", "\nfive\n"), encoding="utf-8", newline="")
  await engine.rung("memory()", on=root)
  assert recalled(root, tmp_path)[1:] == [f"#read\nread_path = {tilded(top)!r}\nread_text = 'five'"]


async def test_a_text_that_shares_most_lines_with_a_text_at_another_path_is_told_by_the_lines_the_chain_lacks(
  tmp_path: Path,
) -> None:
  """A text whose lines a text that the chain holds at another path shares for the most part, as a checkout and its worktree hold one file, is told by the lines that the chain does not hold, so the chain holds the whole file."""
  text = "".join(f"{n}\n" for n in range(1, 2101))
  other = noted(tmp_path / "other", text)
  top = noted(tmp_path / "work", text.replace("\n5\n", "\nfive\n"))
  _, root = extended("memory", tmp_path)
  await engine.rung(f"read({str(other)!r}, HIDDEN)", on=root)
  await engine.rung("memory()", on=root)
  assert recalled(root, tmp_path) == [f"#read\nread_path = {tilded(top)!r}\nread_text = 'five'"]


async def test_memory_reads_each_memory_file_with_a_hidden_show_and_tells_each_in_its_order(tmp_path: Path) -> None:
  """memory reads each memory file on its chain with a hidden show, and tells each in its order, in a paragraph headed with the name of its read, which binds the path of the text and the text, by the lines that the first sentence says."""
  top = noted(tmp_path / "work", "one\ntwo\n")
  sub = noted(tmp_path / "work" / "sub", "three\n")
  _, root = extended("memory", tmp_path)
  await engine.rung('memory("sub/a.txt")', on=root)
  assert recalled(root, tmp_path) == [memorized(top, "one\ntwo\n"), memorized(sub, "three\n")]
  heads = [one.split("\n")[0][1:] for one in engine.turns(on=root)[-1][1].split("\n\n") if one.startswith("#read")]
  told = [engine.get(one) for one in heads]
  assert [(one[0], one[3], one[4]) for one in told][-2:] == [
    ("read", root, f"{tilded(tmp_path / 'work')}/AGENTS.md"),
    ("read", root, f"{tilded(tmp_path / 'work' / 'sub')}/AGENTS.md"),
  ]
  assert all(engine.peek(one) == Text(tilded(top), "one\ntwo\n") for one in heads[-2:-1])


async def test_memory_gives_the_texts_that_it_told(tmp_path: Path) -> None:
  """memory gives the texts that it told, and an empty list when the chain holds every memory file that applies."""
  top = noted(tmp_path / "work", "one\n")
  _, root = extended("memory", tmp_path)
  got = await engine.rung("close(memory())", on=root)
  assert isinstance(got, list)
  assert [one for one in got if one.path.startswith(tilded(tmp_path))] == [Text(tilded(top), "one\n")]
  assert await engine.rung("close(memory())", on=root) == []


async def test_the_memory_files_that_apply_to_a_path(tmp_path: Path) -> None:
  """The memory files that apply to a path are those of the config directory of the user, then of each folder from the root of the file system down to the working directory of the chain, then of each folder under it down to the folder of the path, when the path stands under it."""
  noted(tmp_path / "config", "u\n")
  up = noted(tmp_path, "p\n")
  top = noted(tmp_path / "work", "t\n")
  deep = noted(tmp_path / "work" / "a" / "b", "d\n")
  noted(tmp_path / "other", "o\n")
  _, root = extended("memory", tmp_path)
  assert await paths(root, tmp_path, "a/b/c.txt") == ["config://AGENTS.md", tilded(up), tilded(top), tilded(deep)]
  assert await paths(root, tmp_path, "../other/c.txt") == []


async def test_memory_finds_the_folders_with_reads_alone(tmp_path: Path) -> None:
  """memory finds the folders with reads alone: config:// for the config directory, ./ for the working directory, and the path of a folder and the name of an entry of it and / for a folder under it, each with a hidden show, and for a folder above the working directory it reads the memory files by name, never a listing."""
  noted(tmp_path / "work" / "a", "a\n")
  _, root = extended("memory", tmp_path)
  ran = engine.rung('memory("a/b.txt")', on=root)
  await ran
  reads = [a[4] for a in engine.transcript(root) if a[0] == "read" and a[2] == ran]
  work = tilded(tmp_path / "work")
  assert [one for one in reads if str(one).endswith("/")] == ["./", f"{work}/a/", "config://"]
  assert reads[-1] == f"{tilded(tmp_path / 'work' / 'a')}/AGENTS.md"
  told = {a[1] for a in engine.transcript(root) if a[0] == "tell"}
  assert [a[4] for a in engine.transcript(root) if a[0] == "read" and a[2] == ran and a[1] in told] == [
    f"{tilded(tmp_path / 'work' / 'a')}/AGENTS.md"
  ]


async def test_the_memory_of_a_folder_is_its_agents_md_file_or_its_claude_md_file_then_its_local_file(
  tmp_path: Path,
) -> None:
  """The memory of a folder is its AGENTS.md file, or its CLAUDE.md file when the folder holds no AGENTS.md, then its AGENTS.local.md file, or its CLAUDE.local.md file when the folder holds no AGENTS.local.md, which holds what is true on one machine alone and which git ignores, each when the entries of the folder name it exactly, so a folder has the same memory on a disk that ignores the case of a name too."""
  noted(tmp_path / "config", "u\n", "CLAUDE.md")
  top = noted(tmp_path / "work", "t\n")
  noted(tmp_path / "work", "c\n", "CLAUDE.md")
  mine = noted(tmp_path / "work", "tl\n", "AGENTS.local.md")
  noted(tmp_path / "work", "cl\n", "CLAUDE.local.md")
  sub = noted(tmp_path / "work" / "sub", "s\n", "CLAUDE.md")
  near = noted(tmp_path / "work" / "sub", "sl\n", "CLAUDE.local.md")
  noted(tmp_path / "work" / "low", "l\n", "agents.md")
  noted(tmp_path / "work" / "low", "ll\n", "agents.local.md")
  _, root = extended("memory", tmp_path)
  assert await paths(root, tmp_path, "low/c.txt") == ["config://CLAUDE.md", tilded(top), tilded(mine)]
  assert await paths(root, tmp_path, "sub/c.txt") == [tilded(sub), tilded(near)]


async def test_memory_leaves_out_a_memory_file_whose_content_the_chain_holds_at_any_path(tmp_path: Path) -> None:
  """memory leaves out a memory file whose content the chain holds at any path: one that a read or a write from an act that tells gave the chain, its prefix among them, so one text in two folders, as in a checkout and its worktree, enters a chain once."""
  top = noted(tmp_path / "work", "t\n")
  noted(tmp_path / "work" / "sub", "s\n")
  noted(tmp_path / "work" / "other", "o\n")
  _, root = extended("memory", tmp_path)
  await engine.rung('memory()\nread("sub/AGENTS.md")\nwrite(Text("other/AGENTS.md", "o\\n"))', on=root)
  assert (await paths(root, tmp_path, "sub/c.txt"), await paths(root, tmp_path, "other/c.txt")) == ([], [])
  two = engine.chain("two", root)
  await settle()
  assert await paths(two, tmp_path, "other/c.txt") == []
  top.write_text("t2\n", encoding="utf-8", newline="")
  assert await paths(root, tmp_path, ".") == [tilded(top)]
  # A read from outside an act tells nothing, so the chain holds nothing of it.
  three = engine.chain("three")
  await settle()
  engine.read("sub/AGENTS.md", on=three)
  assert await paths(three, tmp_path, "sub/c.txt") == [tilded(top), tilded(tmp_path / "work" / "sub" / "AGENTS.md")]
  twin = noted(tmp_path / "work" / "twin", "t2\n")
  assert await paths(root, tmp_path, "twin/c.txt") == []
  assert twin.read_text(encoding="utf-8", newline="") == "t2\n"


async def test_memory_leaves_out_each_folder_and_each_file_whose_read_the_world_refuses() -> None:
  """memory leaves out each folder and each file whose read the World refuses, and each folder above one whose read the World refuses."""
  one = next(one for one in _monty.official() if one["name"] == "memory")
  _, root = life(Dead(stands=STANDS), extensions=_monty.extensions([{**one, "life": ""}]))
  ran = engine.rung("close(memory())", on=root)
  assert await ran == []
  assert [a[4] for a in engine.transcript(root) if a[0] == "read" and a[2] == ran] == ["./", "config://"]


async def test_a_path_of_a_scheme_adds_no_folder_of_its_own(tmp_path: Path) -> None:
  """A path of a scheme adds no folder of its own."""
  top = noted(tmp_path / "work", "t\n")
  if sys.platform != "win32":
    noted(tmp_path / "work" / "skills:", "s\n")
  _, root = extended("memory", tmp_path)
  assert await paths(root, tmp_path, "skills://brew") == [tilded(top)]


async def test_the_memory_files_above_the_home_of_the_user_load_from_the_root_down(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  """When the working directory stands under the home of the user, memory reads the memory files above the home, from the root down to the home, by name and never by a listing: it takes the folders from the absolute home that the standing ends with, as many as the path of the home has, since the World writes the home as ~."""
  home = tmp_path / "a" / "b" / "c" / "d" / "u"
  home.mkdir(parents=True)
  home = home.resolve()
  monkeypatch.setenv("HOME", str(home))
  monkeypatch.setenv("USERPROFILE", str(home))
  fifth, fourth, above = noted(home.parents[4], "e\n"), noted(home.parents[3], "f\n"), noted(home.parent, "a\n")
  top = noted(home, "h\n")
  work = noted(home / "work", "t\n")
  _, root = extended("memory", home, doctrine="Read first.\n")
  got = await engine.rung("close([one.path for one in memory()])", on=root)
  assert isinstance(got, list)
  kept = [one for one in got if one.startswith((str(home.parents[4]), "~"))]
  assert kept == [str(fifth), str(fourth), str(above), tilded(top), tilded(work)]
  listed = [str(a[4]) for a in engine.transcript(root) if a[0] == "read" and str(a[4]).endswith("/")]
  assert listed == ["./", "config://"]
