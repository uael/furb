from furb.engine import HEAD, Act, Show, Text

def skills(on: str = "") -> Text:
  """The skills of a chain, as a text that the chain reads: a first line that says how to load and read a skill, then one line for each skill that the World finds, with its name, its marks and what it is for.
  skills reads the path skills:// on its chain, so it tells the list as every read does.
  The World answers skills:// with the skills of the folders .furb/skills, .agents/skills and .claude/skills of the working directory of the chain and of each folder above it, nearest first, then of the folder skills of the config directory of the user, then of the folders of the extensions of the project in .furb/extensions and of each folder that a config turns on, then of the official extensions, and then the skill extending, which the extension carries and which says how a model extends furb.
  A skill is a folder that holds a SKILL.md file, whose frontmatter gives its name and its description, and a skill with no name there takes the name of its folder.
  A skill whose metadata names a word under furb-word has code, and its line marks code, then on when the life runs it on the chain, then default when its metadata says furb-default true.
  A name that an earlier folder holds wins, and the lines stand in the order of the names.
  A chain that finds no skill in a folder reads a list of the official extensions and of extending.
  The World answers skills:// and the name of a skill with code with the word that the life last enabled for it, else with the word of its file, and with the SKILL.md file of a skill with no code.
  The World answers skills:// with a name, a slash and a file with that file of the folder of the skill, and refuses a file that leaves the folder.
  A write of skills:// and the name of a skill with code writes the word in its folder in .furb/extensions of the project, after a copy of its folder there when it stands elsewhere, and keeps the imports of furb at the top of its file.
  When the life runs the skill that a write of skills:// writes, the World enables it again with the new word, and each chain plays the definitions of that word.
  """

def skill(name: str = "", show: Show = HEAD, on: str = "") -> Act[Text]:
  """A skill loaded on a chain: an act of Text to await, which tells its text with its show when it is done.
  The World answers a skill with no code with the text of its SKILL.md file.
  The World answers a skill with code with the prose of its SKILL.md and its interface, or its word when its metadata names no interface under furb-interface, at the path skills:// and the name.
  The first time that the life plays a word of a skill with code, the World says the fact enable with its name, its word, its life word and the digest of its word, so the skill plays as an extension that a config turns on.
  skill with no name plays each skill with code whose metadata says furb-default true and that the life does not run, and the World answers it with the line of each in the list of the skills.
  A skill of a name that no skill has raises Refused where it is awaited.
  """

async def listed(on: str = "") -> None:
  """The skills of a chain, read where its word runs: listed runs skills() on its chain, so the turn after the word that runs it shows the list of the skills of the chain as every read tells it.
  The life word of the extension is await listed(), so the setup of the root reads the skills in the turn of the life words of the extensions, and each chain with no source holds that read in its setup.
  """
