from furb.engine import HEAD, Show, Text

def skills(on: str = "") -> Text:
  """The skills of a chain, as a text that the chain reads: one line for each skill that the World finds, with its name and what it is for.
  skills reads the path skills:// on its chain, so it tells the lines of the list that the chain has not seen, as every read does.
  The World answers skills:// with the skills of the folders .furb/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, and then of the folder skills of the config directory of the user.
  A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder.
  A name that an earlier folder holds wins, and the lines stand in the order of the names.
  A chain that finds no skill reads a list that holds no line.
  The life word of the extension is skills(), so a chain tells its skills when the life enables the extension, and a chain born later tells them at its birth.
  """

def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  """A skill read into a chain: the SKILL.md file of the skill of that name, which the chain tells as a read.
  skill reads the path skills:// and the name, with its show, so its lines stand in the turns as the lines of any read.
  The World answers it with the text of the SKILL.md file at the path of that file, so a later read of that path tells no line that the chain knows.
  A skill of a name that no skill has raises Refused in the caller.
  """
