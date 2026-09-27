"""skills, the skills of a chain."""

from pathlib import Path

from conftest import extended, of, settle, skilled
from furb import engine
from furb.engine import Text
from furb_monty import _monty


async def lines(tmp_path: Path) -> list[str]:
  """The lines of the list of the skills of the root of a life that stands in the folder work of tmp_path."""
  got = await engine.rung("close(skills())", on=extended("skills", tmp_path, _monty.skills))
  assert isinstance(got, Text)
  return got.lines


async def test_the_skills_of_a_chain_as_a_text_that_the_chain_reads(tmp_path: Path) -> None:
  """The skills of a chain, as a text that the chain reads: one line for each skill that the World finds, with its name and what it is for."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung("close(skills())", on=root)
  assert got == Text("skills://", "brew: Make tea.\nsteep: Wait for the leaves.")


async def test_skills_reads_the_path_skills_on_its_chain(tmp_path: Path) -> None:
  """skills reads the path skills:// on its chain, so it tells the lines of the list that the chain has not seen, as every read does."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  root = extended("skills", tmp_path, _monty.skills)
  await engine.rung("skills()", on=root)
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  await engine.rung("skills()", on=root)
  assert of(engine.turns(on=root), "read") == [
    "#read skills://\n# skills://, 0 known\n# 1 brew: Make tea.",
    "#read skills://\n# skills://, 1 known\n# 2 steep: Wait for the leaves.",
  ]


async def test_the_world_answers_skills_with_the_skills_of_the_folders_of_the_chain(tmp_path: Path) -> None:
  """The World answers skills:// with the skills of the folders .furb/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, and then of the folder skills of the config directory of the user."""
  skilled(tmp_path / "work" / ".furb" / "skills", "a", "description: One.")
  skilled(tmp_path / "work" / ".claude" / "skills", "b", "description: Two.")
  skilled(tmp_path / ".claude" / "skills", "c", "description: Three.")
  skilled(tmp_path / "config" / "skills", "d", "description: Four.")
  skilled(tmp_path / "work" / "skills", "e", "description: Not a folder of skills.")
  assert await lines(tmp_path) == ["a: One.", "b: Two.", "c: Three.", "d: Four."]


async def test_a_skill_is_a_folder_that_holds_a_skill_md_file(tmp_path: Path) -> None:
  """A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder."""
  skilled(tmp_path / "work" / ".furb" / "skills", "tea", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "description: Wait for the leaves.")
  (tmp_path / "work" / ".furb" / "skills" / "empty").mkdir()
  assert await lines(tmp_path) == ["brew: Make tea.", "steep: Wait for the leaves."]


async def test_a_name_that_an_earlier_folder_holds_wins(tmp_path: Path) -> None:
  """A name that an earlier folder holds wins, and the lines stand in the order of the names."""
  skilled(tmp_path / "config" / "skills", "brew", "description: Make coffee.")
  skilled(tmp_path / "config" / "skills", "all", "description: Every drink.")
  skilled(tmp_path / ".furb" / "skills", "brew", "description: Make tea.")
  assert await lines(tmp_path) == ["all: Every drink.", "brew: Make tea."]


async def test_a_chain_that_finds_no_skill_reads_a_list_that_holds_no_line(tmp_path: Path) -> None:
  """A chain that finds no skill reads a list that holds no line."""
  assert await lines(tmp_path) == []


async def test_the_life_word_of_the_extension_is_skills(tmp_path: Path) -> None:
  """The life word of the extension is skills(), so a chain tells its skills when the life enables the extension, and a chain born later tells them at its birth."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  root = extended("skills", tmp_path, _monty.skills, lives=True)
  two = engine.chain("two")
  await settle()
  told = ["#read skills://\n# skills://, 0 known\n# 1 brew: Make tea."]
  assert (of(engine.turns(on=root), "read"), of(engine.turns(on=two), "read")) == (told, told)
