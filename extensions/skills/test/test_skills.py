"""skills, the skills of a chain."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import extended, index, skilled
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty


async def lines(tmp_path: Path) -> list[str]:
  """The lines of the list of the skills of the root of a life that stands in the folder work of tmp_path."""
  got = await engine.rung("close(skills())", on=extended("skills", tmp_path, _monty.skills)[1])
  assert isinstance(got, Text)
  return got.lines


async def test_the_skills_of_a_chain_as_a_text_that_the_chain_reads(tmp_path: Path) -> None:
  """The skills of a chain, as a text that the chain reads: a first line that says how to read a skill, then one line for each skill that the World finds, with its name, its marks and what it is for."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung("close(skills())", on=root)
  assert got == Text("skills://", "\n".join(index("brew: Make tea.", "steep: Wait for the leaves.")))


async def test_skills_reads_the_path_skills_on_its_chain(tmp_path: Path) -> None:
  """skills reads the path skills:// on its chain, so it tells the list as every read does."""
  skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  _, root = extended("skills", tmp_path, _monty.skills)
  await engine.rung("skills()", on=root)
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "name: steep\ndescription: Wait for the leaves.")
  await engine.rung("skills()", on=root)
  one, two = "\n".join(index("brew: Make tea.")), "\n".join(index("brew: Make tea.", "steep: Wait for the leaves."))
  assert of(engine.turns(on=root), "read") == [
    f"#read1\nread1_path = 'skills://'\n<s:read1_text>\n{one}</s:read1_text>",
    f"#read2\nread2_path = 'skills://'\n<s:read2_text>\n{two}</s:read2_text>",
  ]


async def test_the_world_answers_skills_with_the_skills_of_the_folders_of_the_chain(tmp_path: Path) -> None:
  """The World answers skills:// with the skills of the folders .furb/skills, .agents/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, then of the folder skills of the config directory of the user, then of the folders of the extensions of the project in .furb/extensions and of each folder that a config turns on, then of the official extensions, and then the skill extending, which the extension carries and which says how a model extends furb."""
  skilled(tmp_path / "work" / ".furb" / "skills", "a", "description: One.")
  skilled(tmp_path / "work" / ".agents" / "skills", "b", "description: Two.")
  skilled(tmp_path / "work" / ".claude" / "skills", "c", "description: Three.")
  skilled(tmp_path / ".claude" / "skills", "d", "description: Four.")
  skilled(tmp_path / "config" / "skills", "f", "description: Five.")
  skilled(tmp_path / "work" / ".furb" / "extensions", "g", "description: Six.\nmetadata:\n  furb-word: g.py")
  skilled(tmp_path / "there", "h", "description: Seven.\nmetadata:\n  furb-word: h.py")
  (tmp_path / "work" / ".furb" / "config.json").write_text('{"extensions": {"h": "../../there/h"}}', encoding="utf-8")
  skilled(tmp_path / "work" / ".furb" / "extensions", "memory", "description: Mine.\nmetadata:\n  furb-word: m.py")
  skilled(tmp_path / "work" / "skills", "e", "description: Not a folder of skills.")
  found = ["a: One.", "b: Two.", "c: Three.", "d: Four.", "f: Five.", "g (code): Six.", "h (code): Seven."]
  assert await lines(tmp_path) == index(*found, "memory (code): Mine.")


async def test_a_skill_is_a_folder_that_holds_a_skill_md_file(tmp_path: Path) -> None:
  """A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder."""
  skilled(tmp_path / "work" / ".furb" / "skills", "tea", "name: brew\ndescription: Make tea.")
  skilled(tmp_path / "work" / ".furb" / "skills", "steep", "description: Wait for the leaves.")
  (tmp_path / "work" / ".furb" / "skills" / "empty").mkdir()
  assert await lines(tmp_path) == index("brew: Make tea.", "steep: Wait for the leaves.")


async def test_a_skill_whose_metadata_names_a_word_has_code(tmp_path: Path) -> None:
  """A skill whose metadata names a word under furb-word has code, and its line marks code, then on when the life runs it on the chain, then default when its metadata says furb-default true."""
  skilled(
    tmp_path / "work" / ".furb" / "skills",
    "note",
    'description: A note.\nmetadata:\n  furb-word: note.py\n  furb-default: "true"',
  )
  got = await lines(tmp_path)
  assert [x for x in got if x.startswith(("note ", "skills ", "memory "))] == [
    "memory (code, default): Tells a chain the AGENTS.md or CLAUDE.md files of the project that apply to the files that it reads.",
    "note (code, default): A note.",
    "skills (code, on): Lists the skills of the project and of the user, and reads one.",
  ]


async def test_a_name_that_an_earlier_folder_holds_wins(tmp_path: Path) -> None:
  """A name that an earlier folder holds wins, and the lines stand in the order of the names."""
  skilled(tmp_path / "config" / "skills", "brew", "description: Make coffee.")
  skilled(tmp_path / "config" / "skills", "all", "description: Every drink.")
  skilled(tmp_path / ".furb" / "skills", "brew", "description: Make tea.")
  skilled(tmp_path / "config" / "skills", "extending", "description: Extend it my way.")
  assert await lines(tmp_path) == index("all: Every drink.", "brew: Make tea.", "extending: Extend it my way.")


async def test_a_chain_that_finds_no_skill_in_a_folder_reads_a_list_of_the_official_extensions(tmp_path: Path) -> None:
  """A chain that finds no skill in a folder reads a list of the official extensions and of extending."""
  assert await lines(tmp_path) == index()


async def test_the_world_answers_a_file_of_the_folder_of_a_skill(tmp_path: Path) -> None:
  """The World answers skills:// with a name, a slash and a file with that file of the folder of the skill, and refuses a file that leaves the folder."""
  path = skilled(tmp_path / "work" / ".furb" / "skills", "brew", "name: brew\ndescription: Make tea.")
  (path.parent / "notes.md").write_text("Use hot water.\n", encoding="utf-8")
  _, root = extended("skills", tmp_path, _monty.skills)
  got = await engine.rung('close(read("skills://brew/notes.md", HIDDEN))', on=root)
  assert got == Text("skills://brew/notes.md", "Use hot water.\n")
  carried = Path(__file__).parent.parent.parent / "catchup" / "catchup.py"
  got = await engine.rung('close(read("skills://catchup/catchup.py", HIDDEN))', on=root)
  assert got == Text("skills://catchup/catchup.py", carried.read_text(encoding="utf-8"))
  with pytest.raises(Refused, match=r"The skill brew holds no file \.\./x\."):
    await engine.rung('close(read("skills://brew/../x", HIDDEN))', on=root)
  with pytest.raises(Refused, match=r"The skill catchup holds no file tui\.ts\."):
    await engine.rung('close(read("skills://catchup/tui.ts", HIDDEN))', on=root)
