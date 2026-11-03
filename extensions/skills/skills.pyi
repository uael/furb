from furb.engine import HEAD, Show, Text

def skills(on: str = "") -> Text:
  """The skills of a chain, as a text that the chain reads: a first line that says how to read a skill, then one line for each skill that the World finds, with its name, its marks and what it is for.
  skills reads the path skills:// on its chain, so it tells the list as every read does.
  The World answers skills:// with the skills of the folders .furb/skills, .agents/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, then of the folder skills of the config directory of the user, then of the folders of the extensions of the project in .furb/extensions and of each folder that a config turns on, then of the official extensions, and then the skill extending, which the extension carries and which says how a model extends furb.
  A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder.
  A skill whose metadata names a word under furb-word has code, and its line marks code, then on when the life runs it on the chain, then default when its metadata says furb-default true.
  A name that an earlier folder holds wins, and the lines stand in the order of the names.
  A chain that finds no skill in a folder reads a list of the official extensions and of extending.
  The World answers skills:// with a name, a slash and a file with that file of the folder of the skill, and refuses a file that leaves the folder.
  """

def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  """A skill read into a chain: the SKILL.md file of the skill of that name, or the word of a skill with code, which the chain tells as a read.
  skill reads the path skills:// and the name, with its show, so its lines stand in the turns as the lines of any read.
  The World answers it with the text of the SKILL.md file at the path of that file, so the text of a skill is the text of a read of that path, and it answers the skill extending that the extension carries with its text at the path skills://extending.
  The World answers a skill with code with the word that the life last enabled for it, else with the word of its file, at the path skills:// and the name.
  A skill of a name that no skill has raises Refused in the caller.
  """

async def listed(on: str = "") -> None:
  """The skills of a chain, read where its word runs: listed runs skills() on its chain, so the turn after the word that runs it shows the list of the skills of the chain as every read tells it.
  The life word of the extension is await listed(), so the setup of the root reads the skills in the turn of the life words of the extensions, and each chain with no source holds that read in its setup.
  """
