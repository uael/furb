"""skill, a skill read into a chain."""

import pytest

from conftest import STANDS, Sand, extension, life, of, texted
from furb import engine
from furb.engine import Refused

WORDS, _ = extension("skills")
"""The words of the files extension and of the skills extension."""
PATH = "/w/.furb/skills/brew/SKILL.md"
"""Where the SKILL.md file of the skill of the suite stands."""
BODY = "---\nname: brew\ndescription: Make tea.\n---\nBoil the water.\nPour it on the leaves.\n"
"""The SKILL.md file of the skill of the suite."""


def world(description: str = "Make tea.") -> Sand:
  """A World with one skill and its SKILL.md file."""
  return Sand(
    files={PATH: BODY},
    stands=STANDS,
    words=WORDS,
    answers={"skills": lambda _: [{"name": "brew", "description": description, "path": PATH}]},
  )


async def test_a_skill_read_into_a_chain() -> None:
  """A skill read into a chain: the text of its SKILL.md file, which the chain tells as a read of that file."""
  _, root = life(world())
  assert texted(await engine.rung('close(skill("brew"))', on=root)) == (PATH, BODY)
  assert of(engine.turns(on=root), "read") == [
    f"#read {PATH}\n# {PATH}, 0 known\n# 1 ---\n# 2 name: brew\n# 3 description: Make tea.\n# 4 ---\n# 5 Boil the water.\n# 6 Pour it on the leaves."
  ]


async def test_skill_finds_the_skill_of_that_name_among_what_skills_gives() -> None:
  """skill finds the skill of that name among what skills gives, which tells what changed of the skills."""
  sand = world()
  _, root = life(sand)
  await engine.rung("skills()", on=root)
  sand.answers["skills"] = lambda _: [{"name": "brew", "description": "Make it strong.", "path": PATH}]
  await engine.rung('skill("brew")', on=root)
  assert of(engine.turns(on=root), "skills") == ["#skills\n# brew: Make tea.", "#skills\n# brew: Make it strong."]


async def test_skill_reads_the_path_of_that_skill_with_its_show() -> None:
  """skill reads the path of that skill with its show, so its lines stand in the turns as the lines of any read."""
  _, root = life(world())
  await engine.rung('skill("brew", span(5, 5))\nskill("brew", HIDDEN)', on=root)
  assert of(engine.turns(on=root), "read") == [f"#read {PATH}\n# {PATH}, 0 known\n# 5 Boil the water."]


async def test_a_skill_of_a_name_that_no_skill_has_raises_refused_in_the_caller() -> None:
  """A skill of a name that no skill has raises Refused in the caller."""
  _, root = life(world())
  with pytest.raises(Refused, match="no skill steep"):
    await engine.rung('close(skill("steep"))', on=root)
