"""skills, the skills of a chain."""

from pathlib import Path

from conftest import of
from extensions.conftest import EXTENDING, extended, skilled
from furb import engine
from furb.engine import Text
from furb_monty import _monty


async def lines(tmp_path: Path) -> list[str]:
  """The lines of the list of the skills of the root of a life that stands in the folder work of tmp_path."""
  got = await engine.rung("close(skills())", on=extended("skills", tmp_path, _monty.skills)[1])
  assert isinstance(got, Text)
  return got.lines


async def test_the_skills_of_a_chain_as_a_text_that_the_chain_reads(tmp_path: Path) -> None:
  """The skills of a chain, as a text that the chain reads: one line for each skill that the World finds, with its name and what it is for."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung("close(skills())", on=root)
  assert got == Text("skills://", f"brew: Make tea.\n{EXTENDING}\nsteep: Wait for the leaves.")


async def test_skills_reads_the_path_skills_on_its_chain(tmp_path: Path) -> None:
  """skills reads the path skills:// on its chain, so it tells the list as every read does."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  await engine.rung("skills()", on=root)
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  await engine.rung("skills()", on=root)
  assert of(engine.turns(on=root), "read") == [
    f"#read1\nread1_path = 'skills://'\n<s:read1_text>\nbrew: Make tea.\n{EXTENDING}</s:read1_text>",
    f"#read2\nread2_path = 'skills://'\n<s:read2_text>\nbrew: Make tea.\n{EXTENDING}\nsteep: Wait for the leaves.</s:read2_text>",
  ]


async def test_the_world_answers_skills_with_the_skills_of_the_folders_of_the_chain(tmp_path: Path) -> None:
  """The World answers skills:// with the skills of the folders .furb/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, then of the folder skills of the config directory of the user, and then the skill extending, which the extension carries and which says how a model extends furb."""
  skilled(tmp_path / "work" / ".furb" / "skills", "a", "description: One.")
  skilled(tmp_path / "work" / ".claude" / "skills", "b", "description: Two.")
  skilled(tmp_path / ".claude" / "skills", "c", "description: Three.")
  skilled(tmp_path / "config" / "skills", "d", "description: Four.")
  skilled(tmp_path / "work" / "skills", "e", "description: Not a folder of skills.")
  assert await lines(tmp_path) == ["a: One.", "b: Two.", "c: Three.", "d: Four.", EXTENDING]


async def test_a_skill_is_a_folder_that_holds_a_skill_md_file(tmp_path: Path) -> None:
  """A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder."""
  skilled(tmp_path / "work" / ".furb" / "skills", "tea", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "description: Wait for the leaves.")
  (tmp_path / "work" / ".furb" / "skills" / "empty").mkdir()
  assert await lines(tmp_path) == ["brew: Make tea.", EXTENDING, "steep: Wait for the leaves."]


async def test_a_name_that_an_earlier_folder_holds_wins(tmp_path: Path) -> None:
  """A name that an earlier folder holds wins, and the lines stand in the order of the names."""
  skilled(tmp_path / "config" / "skills", "brew", "description: Make coffee.")
  skilled(tmp_path / "config" / "skills", "all", "description: Every drink.")
  skilled(tmp_path / ".furb" / "skills", "brew", "description: Make tea.")
  skilled(tmp_path / "config" / "skills", "extending", "description: Extend it my way.")
  assert await lines(tmp_path) == ["all: Every drink.", "brew: Make tea.", "extending: Extend it my way."]


async def test_a_chain_that_finds_no_skill_in_a_folder_reads_the_one_line_of_extending(tmp_path: Path) -> None:
  """A chain that finds no skill in a folder reads a list that holds the one line of extending."""
  assert await lines(tmp_path) == [EXTENDING]
