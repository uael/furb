"""extend, a word kept as an extension of the project."""

import json
from collections.abc import Generator
from dataclasses import dataclass
from pathlib import Path

from conftest import Sand, plain, settle
from furb import engine
from skills.conftest import extended, tilded


@dataclass
class Disk(Sand):
  """A World of the suite that leaves each write to the files of the machine, so the ear of the extensions reads
  what a write put on the disk."""

  def hear(self, a: tuple) -> Generator[tuple | None, tuple]:
    """What the World does with one fact, less a write."""
    if a[0] != "write":
      yield from super().hear(a)


F = "def f() -> int:\n  return 1"
"""A word that only defines."""

G = "def g() -> int:\n  return 2"
"""A second word that only defines."""


async def test_extend_writes_the_words_of_the_rungs_it_names(tmp_path: Path) -> None:
  """extend writes the words of the rungs it names, in order, as the program of the chain holds them, with a blank line between two, after the word that the file holds, as the word of the extension of that name in .furb/skills/<name>, with its manifest SKILL.md, whose metadata names that word under furb-word and which keeps each other line that it holds, and gives the word as it landed."""
  _, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f, g = engine.rung(F, on=root), engine.rung(G, on=root)
  await g
  got = await engine.rung(f"close(extend('kit', {g!r}))", on=root)
  folder = tmp_path / "work" / ".furb" / "skills" / "kit"
  assert isinstance(got, engine.Text)
  assert (got.path, got.content) == (tilded(folder / "kit.py"), G + "\n")
  await engine.rung(f"extend('kit', {f!r})", on=root)
  assert (folder / "kit.py").read_text(encoding="utf-8") == f"{G}\n\n{F}\n"
  said = "The extension kit of this project."
  made = f"---\nname: kit\ndescription: {said}\nmetadata:\n  furb-word: kit.py\n---\n{said}\n"
  assert (folder / "SKILL.md").read_text(encoding="utf-8") == made
  mine = "---\nname: kit\ndescription: Mine.\nlicense: MIT\nmetadata:\n  furb-life: f()\n  furb-word: old.py\n---\nMy body.\n"
  (folder / "SKILL.md").write_text(mine, encoding="utf-8")
  await engine.rung(f"extend('kit', {g!r})", on=root)
  kept = "---\nname: kit\ndescription: Mine.\nlicense: MIT\nmetadata:\n  furb-word: kit.py\n  furb-life: f()\n---\nMy body.\n"
  assert (folder / "SKILL.md").read_text(encoding="utf-8") == kept


async def test_extend_writes_tui_as_the_part_that_the_tui_loads(tmp_path: Path) -> None:
  """extend writes tui, when it is given, as tui.ts in the folder of the extension, the part that the TUI loads, and the manifest names tui.ts under furb-view of its metadata whenever the folder holds that file."""
  _, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f, g = engine.rung(F, on=root), engine.rung(G, on=root)
  await g
  folder = tmp_path / "work" / ".furb" / "skills" / "kit"
  part = 'export default (furb) => furb.panel("Kit", () => ["one"])'
  await engine.rung(f"extend('kit', {f!r}, tui={part!r})", on=root)
  assert (folder / "tui.ts").read_text(encoding="utf-8") == part + "\n"
  said = "The extension kit of this project."
  manifest = f"---\nname: kit\ndescription: {said}\nmetadata:\n  furb-word: kit.py\n  furb-view: tui.ts\n---\n{said}\n"
  assert (folder / "SKILL.md").read_text(encoding="utf-8") == manifest
  await engine.rung(f"extend('kit', {g!r})", on=root)
  assert (folder / "tui.ts").read_text(encoding="utf-8") == part + "\n"
  assert (folder / "SKILL.md").read_text(encoding="utf-8") == manifest


async def test_extend_turns_the_extension_on_in_the_config_of_the_project(tmp_path: Path) -> None:
  """extend turns the extension on in .furb/config.json with the path of its folder from the folder of that config, and keeps every other entry of the config."""
  (tmp_path / "work" / ".furb").mkdir(parents=True)
  (config := tmp_path / "work" / ".furb" / "config.json").write_text(
    '{"skills": {"memory": false}, "x": 1}', encoding="utf-8", newline=""
  )
  _, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f = engine.rung(F, on=root)
  await engine.rung(f"extend('kit', {f!r})", on=root)
  assert json.loads(config.read_text(encoding="utf-8")) == {"skills": {"memory": False, "kit": "skills/kit"}, "x": 1}


async def test_a_write_of_the_config_of_the_project_enables_what_it_turns_on(tmp_path: Path) -> None:
  """When a write of the config of the project lands, the World enables each extension that it turns on by its path and that the life does not run, so it plays as a rung on every chain there is and on each chain at its birth, and the record keeps the fact for every later life."""
  sand, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  there = engine.chain("there")
  await settle()
  f = engine.rung(F, on=root)
  await engine.rung(f"extend('kit', {f!r})", on=root)
  await settle()
  born = engine.chain("born")
  await settle()
  assert [a[3] for a in engine.transcript(on=root) if a[0] == "enable"] == ["ouroboros", "kit"]
  assert [callable(engine.module(one).get("f")) for one in (there, born)] == [True, True]
  _, again = extended("ouroboros", tmp_path, lives=True, sand=Disk(), record=plain(sand.record))
  await settle()
  assert await engine.rung("close(enabled())", on=again) == ["ouroboros", "kit"]
  assert callable(engine.module(again).get("f"))


async def test_a_write_again_of_an_extension_that_the_life_runs_reloads_its_definitions(tmp_path: Path) -> None:
  """When extend writes again the word of an extension that the life runs, the World enables it again with the new word, and each chain plays the definitions of that word alone, so a function and a name in capitals take their new value and every other name keeps its value."""
  sand, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  there = engine.chain("there")
  await settle()
  f = engine.rung(F + "\n\nkept = ['first']\nLIMIT = 1", on=root)
  await engine.rung(f"extend('kit', {f!r})", on=root)
  await settle()
  await engine.rung("kept.append('mine')", on=there)
  g = engine.rung(G + "\n\nkept = []\nLIMIT = 2", on=root)
  await engine.rung(f"extend('kit', {g!r})", on=root)
  await settle()
  assert [a[3] for a in engine.transcript(on=root) if a[0] == "enable"] == ["ouroboros", "kit", "kit"]
  held = engine.module(there)
  assert (held.get("kept"), held.get("LIMIT"), callable(held.get("g"))) == (["first", "mine"], 2, True)
  _, again = extended("ouroboros", tmp_path, lives=True, sand=Disk(), record=plain(sand.record))
  await settle()
  assert await engine.rung("close(enabled())", on=again) == ["ouroboros", "kit"]
  assert [engine.module(again).get(name) for name in ("kept", "LIMIT")] == [[], 2]


async def test_extend_with_session_keeps_the_kit_in_the_record_alone(tmp_path: Path) -> None:
  """extend with session keeps the kit in the record alone: it writes no file, says the fact enable of the root with the word, so each chain plays it as an extension, and gives the word at session://<name>/<name>.py."""
  _, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f = engine.rung(F, on=root)
  await f
  got = await engine.rung(f"close(extend('todo', {f!r}, session=True))", on=root)
  assert isinstance(got, engine.Text) and (got.path, got.content) == ("session://todo/todo.py", F + "\n")
  assert not (tmp_path / "work" / ".furb").exists()
  await engine.rung("two = chain('two')", on=root)
  await settle()
  assert engine.module(engine.module(root)["two"])["f"]() == 1


async def test_extend_with_session_and_view_names_the_value_of_the_view_in_the_fact(tmp_path: Path) -> None:
  """extend with session and view names in that fact the value of the root that holds the tree of the view of the kit, a node of the protocol as a dict, as {'id': 'todo', 'node': 'text', 'runs': [{'text': 'nothing to do'}]}: each window that mounts the view of the kit gets that tree from the daemon, and again each time that the value changes, at each later open too."""
  _, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f = engine.rung(F, on=root)
  await f
  await engine.rung(f"extend('todo', {f!r}, session=True, view='view')", on=root)
  enabled = [x for x in engine.transcript(root) if x[0] == "enable" and x[3] == "todo"]
  assert [x[10] for x in enabled] == ["view"]


async def test_a_later_life_plays_the_session_kit_from_the_record(tmp_path: Path) -> None:
  """A later life on the record plays the kit on each chain from that fact, with no file in the project or the config."""
  sand, root = extended("ouroboros", tmp_path, lives=True, sand=Disk())
  f = engine.rung(F, on=root)
  await f
  await engine.rung(f"extend('todo', {f!r}, session=True)", on=root)
  await settle()
  _, again = extended("ouroboros", tmp_path, lives=True, sand=Disk(), record=plain(sand.record))
  await settle()
  await engine.rung("three = chain('three')", on=again)
  await settle()
  assert engine.module(engine.module(again)["three"])["f"]() == 1
  assert not (tmp_path / "work" / ".furb").exists()
