"""skill, a skill loaded on a chain."""

import hashlib
from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import extended, official_lines, skilled, tilded
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty

NOTE = "from furb.engine import Text, read\n\n\ndef note() -> Text:\n  return read('n')\n"
"""NOTE is the file of the word of the skill note, a python module that imports from furb.engine."""

WORD = "def note() -> Text:\n  return read('n')"
"""WORD is the word of the skill note: its file less its imports of furb."""


def brewed(tmp_path: Path) -> tuple[str, Path]:
  """A life that stands in the folder work of tmp_path, whose one skill is brew: the root, and the SKILL.md file."""
  path = skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  return extended("skills", tmp_path, _monty.skills)[1], path


def noted(tmp_path: Path, *, interface: bool = False, default: bool = False) -> Path:
  """The folder of the skill note with code in .furb/skills of the folder work of tmp_path, with its interface
  note.pyi when it ships one, and which plays by default when it says so."""
  fields = (
    "\n  furb-word: note.py"
    + ("\n  furb-interface: note.pyi" if interface else "")
    + ('\n  furb-default: "true"' if default else "")
  )
  folder = skilled(
    tmp_path / "work" / ".furb" / "skills", "note", f"name: note\ndescription: A note.\nmetadata:{fields}"
  ).parent
  (folder / "note.py").write_text(NOTE, encoding="utf-8", newline="")
  (folder / "note.pyi").write_text("def note() -> Text: ...\n", encoding="utf-8", newline="")
  return folder


async def test_a_skill_loaded_on_a_chain(tmp_path: Path) -> None:
  """A skill loaded on a chain: an act of Text to await, which tells its text with its show when it is done."""
  root, path = brewed(tmp_path)
  got = await engine.rung('close(await skill("brew", span(1, 1)))', on=root)
  assert got == Text(tilded(path), path.read_text(encoding="utf-8"))
  assert of(engine.turns(on=root), "skill")[-1] == "#skill1 done\nskill1_text = '---'"


async def test_the_world_answers_a_skill_with_no_code_with_its_skill_md(tmp_path: Path) -> None:
  """The World answers a skill with no code with the text of its SKILL.md file."""
  root, path = brewed(tmp_path)
  assert await engine.rung('close(await skill("brew"))', on=root) == Text(
    tilded(path), path.read_text(encoding="utf-8")
  )


async def test_the_world_answers_a_skill_with_code_with_its_prose_and_its_interface(tmp_path: Path) -> None:
  """The World answers a skill with code with the prose of its SKILL.md and its interface, or its word when its metadata names no interface under furb-interface, at the path skills:// and the name."""
  noted(tmp_path, interface=True)
  _, root = extended("skills", tmp_path, _monty.skills)
  assert await engine.rung('close(await skill("note"))', on=root) == Text(
    "skills://note", "Steps.\n\ndef note() -> Text: ...\n"
  )
  noted(tmp_path / "other")
  _, other = extended("skills", tmp_path / "other", _monty.skills)
  assert await engine.rung('close(await skill("note"))', on=other) == Text("skills://note", f"Steps.\n\n{WORD}")


async def test_the_first_play_of_a_word_says_the_fact_enable(tmp_path: Path) -> None:
  """The first time that the life plays a word of a skill with code, the World says the fact enable with its name, its word, its life word and the digest of its word, so the skill plays as an extension that a config turns on."""
  noted(tmp_path)
  _, root = extended("skills", tmp_path, _monty.skills)
  await engine.rung('await skill("note")', on=root)
  await engine.rung('await skill("note")', on=root)
  said = [a for a in engine.transcript(on=root) if a[0] == "enable" and a[3] == "note"]
  assert [(a[4], a[5], a[8]) for a in said] == [(WORD, "", hashlib.sha256(WORD.encode()).hexdigest())]
  assert callable(engine.module(root).get("note"))


async def test_skill_with_no_name_plays_each_default_skill(tmp_path: Path) -> None:
  """skill with no name plays each skill with code whose metadata says furb-default true and that the life does not run, and the World answers it with the line of each in the list of the skills."""
  noted(tmp_path, default=True)
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung("close(await skill())", on=root)
  lines = official_lines(on=["catchup", "memory", "compaction", "naming"])
  defaults = [x for x in lines if "default" in x.split(":")[0]]
  assert got == Text("skills://", "\n".join(sorted([*defaults, "note (code, on, default): A note."])))
  assert sorted(str(a[3]) for a in engine.transcript(on=root) if a[0] == "enable" and a[2] == "skills") == [
    "catchup",
    "compaction",
    "memory",
    "naming",
    "note",
  ]

  assert [a[5] for a in engine.transcript(on=root) if a[0] == "enable" and a[3] == "naming"] == ["name_session()"]


async def test_a_skill_of_a_name_that_no_skill_has_raises_refused(tmp_path: Path) -> None:
  """A skill of a name that no skill has raises Refused where it is awaited."""
  root, _ = brewed(tmp_path)
  with pytest.raises(Refused, match=r"There is no skill tea\."):
    await engine.rung('close(await skill("tea"))', on=root)
