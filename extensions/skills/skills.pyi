from dataclasses import dataclass
from typing import Literal

from furb.engine import HEAD, Show, Text

def skills(on: str = "") -> list[Skill]:
  """The skills that the World finds for a chain, each with its name, its description and the path of its SKILL.md file, which a model reads before it chooses one.
  skills asks the World a skills question on its chain, and gives a Skill for each table of the answer, in the order of the answer.
  A World that answers no skills question gives no skill.
  A skills question that the World refuses raises Refused in the caller.
  skills tells one line for each skill that is new since the skills question before it on the chain, or whose description changed since then, with its name and its description.
  skills tells one line for each skill that is gone since the skills question before it on the chain.
  skills tells nothing when nothing changed, so a chain hears of each skill once.
  The life word of the extension calls skills in every life, so a chain without a source tells its skills before its first ask, and a later life tells what changed.
  """

def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  """A skill read into a chain: the text of its SKILL.md file, which the chain tells as a read of that file.
  skill finds the skill of that name among what skills gives, which tells what changed of the skills.
  skill reads the path of that skill with its show, so its lines stand in the turns as the lines of any read.
  A skill of a name that no skill has raises Refused in the caller.
  """

@dataclass
class Skill:
  """A skill that the World found: its name, what it is for, and the path of its SKILL.md file.
  The World answers each skill as a table of plain data, and skills makes the Skill of it, so the record holds no Skill.
  """

  name: str
  description: str
  path: str

type Skills = tuple[Literal["skills"], str, str, str]
"""A skills is the question of the skills of a chain, which carries no word, and which the World answers with a list of tables, each with a name, a description and a path."""
