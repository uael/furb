from typing import Literal

from furb.engine import Act, Text

def memory(path: str = ".", on: str = "") -> list[Text]:
  """The memory of a path, told to its chain: each memory file that applies to the path and that the chain does not hold as it stands, read with the show of every line, so the chain holds the whole file, and told by every line that the chain has not seen.
  memory asks the World a memory question on its chain with the path, and tells each text of the answer in its order, under the header memory and the path of the text, with the show of every line.
  memory gives the texts of the answer, and an empty list when the chain holds every memory file that applies.
  A memory question that the World refuses raises Refused in the caller.
  """

def remember(on: str = "") -> Act:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read on the chain gives, and the memory that changed at each stand, and that never completes.
  A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake.
  The life word of the extension is remember(), so a chain tells its memory when the life enables the extension, and a chain born later tells it at its birth, before its first prompt asks a model.
  A memory file enters a chain once, and again only when it changed, and then only its lines that changed, since the World leaves out what the chain holds and a text tells only the lines that the chain has not seen.
  A later life tells what the record holds, since the journal answers each memory question that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life.
  """

type Memory = tuple[Literal["memory"], str, str, str, str]
"""A memory is the question of the memory files that apply to a path, which carries the path, and which the World answers with a list of texts.
The World answers with the memory of the user first, then of each folder from the root of the file system down to the working directory of the chain, then of each folder under it down to the folder of the path, when the path stands under it.
The memory of a folder is its CLAUDE.md file, or its AGENTS.md file when the folder holds no CLAUDE.md, and the memory of the user is that of the config directory of the user.
The World leaves out a memory file whose content the chain holds: one that a memory question told it, or that it read or wrote, with that content, its prefix among them.
A path of a scheme adds no folder of its own.
"""
