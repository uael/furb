"""skills, the skills that the World finds for a chain."""

import pytest

from conftest import STANDS, Sand, extension, life, of, settle
from furb import engine
from furb.engine import Refused

WORDS, LIVES = extension("skills")
"""The word and the life word of the skills extension."""
BREW = {"name": "brew", "description": "Make tea.", "path": "/w/.furb/skills/brew/SKILL.md"}
"""A skill as the World answers it."""
STEEP = {"name": "steep", "description": "Wait for the leaves.", "path": "/w/.claude/skills/steep/SKILL.md"}
"""A second skill as the World answers it."""


def world(*found: dict[str, str]) -> tuple[Sand, list[list[dict[str, str]]]]:
  """A World that answers each skills question with the last answer of a list, which the test adds to."""
  answers: list[list[dict[str, str]]] = [list(found)]
  return Sand(stands=STANDS, words=WORDS, answers={"skills": lambda _: answers[-1]}), answers


def skilled(got: object) -> list[tuple[object, object, object]]:
  """The name, the description and the path of each skill a verb gave, on each engine."""
  assert isinstance(got, list)
  return [(one.name, one.description, one.path) for one in got]


async def test_the_skills_that_the_world_finds_for_a_chain() -> None:
  """The skills that the World finds for a chain, each with its name, its description and the path of its SKILL.md file, which a model reads before it chooses one."""
  sand, _ = world(BREW)
  _, root = life(sand)
  assert skilled(await engine.rung("close(skills())", on=root)) == [("brew", "Make tea.", BREW["path"])]


async def test_skills_asks_the_world_a_skills_question_on_its_chain() -> None:
  """skills asks the World a skills question on its chain, and gives a Skill for each table of the answer, in the order of the answer."""
  sand, _ = world(STEEP, BREW)
  _, root = life(sand)
  two = engine.chain("two")
  await settle()
  assert [one for one, *_ in skilled(await engine.rung("close(skills())", on=two))] == ["steep", "brew"]
  assert [(a[0], a[3]) for a in engine.asked.values() if a[0] == "skills"] == [("skills", two)]
  assert not [a for a in engine.asked.values() if a[0] == "skills" and a[3] == root]


async def test_a_world_that_answers_no_skills_question_gives_no_skill() -> None:
  """A World that answers no skills question gives no skill."""
  _, root = life(Sand(stands=STANDS, words=WORDS))
  assert await engine.rung("close(skills())", on=root) == []
  assert of(engine.turns(on=root), "skills") == []


async def test_a_skills_question_that_the_world_refuses_raises_refused_in_the_caller() -> None:
  """A skills question that the World refuses raises Refused in the caller."""
  _, root = life(Sand(stands=STANDS, words=WORDS, answers={"skills": lambda _: Refused("no skills here")}))
  with pytest.raises(Refused, match="no skills here"):
    await engine.rung("close(skills())", on=root)


async def test_skills_tells_one_line_for_each_skill_that_is_new_or_changed() -> None:
  """skills tells one line for each skill that is new since the skills question before it on the chain, or whose description changed since then, with its name and its description."""
  sand, answers = world(BREW)
  _, root = life(sand)
  await engine.rung("skills()", on=root)
  answers.append([{**BREW, "description": "Make it strong."}, STEEP])
  await engine.rung("skills()", on=root)
  assert of(engine.turns(on=root), "skills") == [
    "#skills\n# brew: Make tea.",
    "#skills\n# brew: Make it strong.\n# steep: Wait for the leaves.",
  ]


async def test_skills_tells_one_line_for_each_skill_that_is_gone() -> None:
  """skills tells one line for each skill that is gone since the skills question before it on the chain."""
  sand, answers = world(BREW, STEEP)
  _, root = life(sand)
  await engine.rung("skills()", on=root)
  answers.append([STEEP])
  assert skilled(await engine.rung("close(skills())", on=root)) == [("steep", STEEP["description"], STEEP["path"])]
  assert of(engine.turns(on=root), "skills")[-1] == "#skills\n# brew is gone"


async def test_skills_tells_nothing_when_nothing_changed() -> None:
  """skills tells nothing when nothing changed, so a chain hears of each skill once."""
  sand, _ = world(BREW)
  _, root = life(sand)
  for _ in range(3):
    await engine.rung("skills()", on=root)
  assert of(engine.turns(on=root), "skills") == ["#skills\n# brew: Make tea."]


async def test_the_life_word_of_the_extension_calls_skills_in_every_life() -> None:
  """The life word of the extension calls skills in every life, so a chain without a source tells its skills before its first ask, and a later life tells what changed."""
  sand, _ = world(BREW)
  sand.lives = LIVES
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "one", on=root) == 1
  await settle()
  first = engine.turns(on=root)[0][1]
  assert first.index("#skills\n# brew: Make tea.") < first.index("#prompt1 one")
  later, _ = world(BREW, STEEP)
  later.lives = LIVES
  _, again = life(later, sand.record)
  await settle()
  assert again == root
  assert of(engine.turns(on=root), "skills") == ["#skills\n# brew: Make tea.", "#skills\n# steep: Wait for the leaves."]
