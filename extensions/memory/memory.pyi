from typing import Literal

from furb.engine import Act, Text

def memory(path: str = ".", on: str = "") -> list[Text]:
  """The memory of a path, told to its chain: each memory file that applies to the path and whose content the chain does not hold, told whole when the chain holds no text at its path, and else by the lines that differ from the text the chain holds there, as differs picks them, so the chain holds the whole file.
  A text whose lines a text that the chain holds at another path shares for the most part, as a checkout and its worktree hold one file, is told by the lines that the chain does not hold, so the chain holds the whole file.
  memory asks the World a memory question on its chain with the path, and tells each text of the answer in its order, in a paragraph headed with the name of the question, which binds the path of the text and the text, by the lines that the first sentence says.
  memory gives the texts of the answer, and an empty list when the chain holds every memory file that applies.
  A memory question that the World refuses raises Refused in the caller.
  """

async def recall(on: str = "") -> Act:
  """The memory of a chain, read where its word runs: recall asks the World a memory question for the working directory of its chain, then reads the path of each text of the answer, in its order, so the turn after the word that runs it shows each memory file as every read tells it.
  recall then starts remember on its chain and gives it, so the chain keeps its memory from there, and remember tells nothing of a file that the reads hold, since the World leaves out what the chain holds.
  The life word of the extension is await recall(), so the setup of the root reads its memory in the turn of the life words of the extensions, and each chain with no source holds those reads in its setup.
  """

def remember(on: str = "") -> Act:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read made on the chain after it gives, and the memory that changed at each stand, and that never completes.
  A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake.
  remember asks nothing of a read made before it, whose done a later life can say before or after the birth of remember, so a life and its replay ask the same memory questions.
  A memory file enters a chain once, and again only when it changed, by the lines that changed, since the World leaves out what the chain holds.
  A later life tells what the record holds, since the journal answers each memory question that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life.
  At a stand the catch-up that the extension catchup makes before remember tells each memory file that a read or a write on the chain gave, so remember tells again only a memory file that a memory question alone gave the chain, since the World leaves out the content that the catch-up read.
  """

type Memory = tuple[Literal["memory"], str, str, str, str]
"""A memory is the question of the memory files that apply to a path, which carries the path, and which the World answers with a list of texts.
The World answers with the memory of the user first, then of each folder from the root of the file system down to the working directory of the chain, then of each folder under it down to the folder of the path, when the path stands under it.
The memory of a folder is its AGENTS.md file, or its CLAUDE.md file when the folder holds no AGENTS.md, and the memory of the user is that of the config directory of the user.
The World leaves out a memory file whose content the chain holds at any path: one that a memory question told it, or that it read or wrote, with that content, its prefix among them, so one text in two folders, as in a checkout and its worktree, enters a chain once.
A path of a scheme adds no folder of its own.
"""
