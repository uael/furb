"""Skill, a skill that the World found."""

from conftest import STANDS, Sand, extension, life, plain, said
from furb import engine

WORDS, _ = extension("skills")
"""The words of the files extension and of the skills extension."""
BREW = {"name": "brew", "description": "Make tea.", "path": "/w/.furb/skills/brew/SKILL.md"}
"""A skill as the World answers it."""


async def test_a_skill_that_the_world_found() -> None:
  """A skill that the World found: its name, what it is for, and the path of its SKILL.md file."""
  _, root = life(Sand(stands=STANDS, words=WORDS, answers={"skills": lambda _: [BREW]}))
  got = await engine.rung("close([(x.name, x.description, x.path, type(x).__name__) for x in skills()])", on=root)
  assert got == [("brew", "Make tea.", BREW["path"], "Skill")]


async def test_the_world_answers_each_skill_as_a_table_of_plain_data() -> None:
  """The World answers each skill as a table of plain data, and skills makes the Skill of it, so the record holds no Skill."""
  sand = Sand(stands=STANDS, words=WORDS, answers={"skills": lambda _: [BREW]})
  log, root = life(sand)
  await engine.rung("skills()", on=root)
  asked = [a[1] for a in engine.asked.values() if a[0] == "skills"]
  assert [a[3] for a in said(log, "done") if a[1] in asked] == [[BREW]]
  assert "'is': 'Skill'" not in str(plain(sand.record))
  assert [entry[1] for entry in plain(sand.record) if entry[0][1] in asked] == [[BREW]]
