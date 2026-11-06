from furb.engine import Act, Text

def memory(path: str = ".", on: str = "") -> list[Text]:
  """The memory of a path, told to its chain: each memory file that applies to the path and whose content the chain does not hold, told whole when the chain holds no text at its path, and else by the lines that differ from the text the chain holds there, as differs picks them, so the chain holds the whole file.
  A text whose lines a text that the chain holds at another path shares for the most part, as a checkout and its worktree hold one file, is told by the lines that the chain does not hold, so the chain holds the whole file.
  memory reads each memory file on its chain with a hidden show, and tells each in its order, in a paragraph headed with the name of its read, which binds the path of the text and the text, by the lines that the first sentence says.
  memory gives the texts that it told, and an empty list when the chain holds every memory file that applies.
  The memory files that apply to a path are those of the config directory of the user, then of each folder from the root of the file system down to the working directory of the chain, then of each folder under it down to the folder of the path, when the path stands under it.
  memory finds the folders with reads alone: config:// for the config directory, ./ for the working directory, the path of a folder and ../ for the folder above it, and the path of a folder and the name of an entry of it and / for a folder under it, each with a hidden show.
  The memory of a folder is its AGENTS.md file, or its CLAUDE.md file when the folder holds no AGENTS.md, then its AGENTS.local.md file, or its CLAUDE.local.md file when the folder holds no AGENTS.local.md, which holds what is true on one machine alone and which git ignores, each when the entries of the folder name it exactly, so a folder has the same memory on a disk that ignores the case of a name too.
  memory leaves out a memory file whose content the chain holds at any path: one that a read or a write from an act that tells gave the chain, its prefix among them, so one text in two folders, as in a checkout and its worktree, enters a chain once.
  memory leaves out each folder and each file whose read the World refuses, and each folder above one whose read the World refuses.
  A path of a scheme adds no folder of its own.
  """

async def recall(on: str = "") -> Act:
  """The memory of a chain, read where its word runs: recall starts remember on its chain and gives it, so the turn after the word that runs it shows each memory file of the working directory of the chain as its read tells it, and the chain keeps its memory from there.
  The life word of the extension is await recall(), so the setup of the root reads its memory in the turn of the life words of the extensions, and each chain with no source holds those reads in its setup.
  """

def remember(on: str = "") -> Act:
  """The memory of a chain, kept: an act on the chain that tells the memory of its working directory when it is made, then the memory of the path of each text that a read of a word on the chain after it gives, and the memory that changed at each stand, and that never completes.
  remember tells the memory of a folder after a cd of a word on the chain into it, as it does after a read of a text in it: the memory of the path that the cd gave.
  remember tells the memory of the folder of a text after a write of a word on the chain, as it does after a read of that text.
  A cancel over it ends it, as it ends every act, and a pause over it holds it until the wake.
  remember asks nothing of a read made before it, whose done a later life can say before or after the birth of remember, so a life and its replay make the same reads.
  remember asks nothing of a read of a memory file, or of a path that ends with /, as each read of memory, since it read the folder of each.
  A memory file enters a chain once, and again only when it changed, by the lines that changed, since memory leaves out what the chain holds.
  It tells the memory that changed at each stand from outside an act, as the stand that boot makes at the tip of a life, and at no stand that a word makes, as the one that each new chain plays on itself.
  A later life tells what the record holds, since the journal answers each read that the record holds, and a memory file that changed since enters the chain at the stand at the tip of that life.
  At a stand the catch-up that the extension catchup makes before remember tells what changed in each memory file that a read on the chain gave, the reads of memory among them, so remember tells at a stand only a memory file that is new to the chain, since memory leaves out the content that the catch-up read.
  """
