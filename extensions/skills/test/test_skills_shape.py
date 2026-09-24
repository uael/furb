"""Skills, the question of the skills of a chain."""

from conftest import STANDS, Sand, extension, life
from furb import engine

WORDS, _ = extension("skills")
"""The word of the skills extension."""


async def test_a_skills_is_the_question_of_the_skills_of_a_chain() -> None:
  """A skills is the question of the skills of a chain, which carries no word, and which the World answers with a list of tables, each with a name, a description and a path."""
  table = {"name": "brew", "description": "Make tea.", "path": "/w/.furb/skills/brew/SKILL.md"}
  _, root = life(Sand(stands=STANDS, words=WORDS, answers={"skills": lambda _: [table]}))
  await engine.rung("skills()", on=root)
  [asked] = [a for a in engine.asked.values() if a[0] == "skills"]
  assert (asked[0], asked[3], len(asked)) == ("skills", root, 4)
  assert engine.outcomes[asked[1]] == [table]
