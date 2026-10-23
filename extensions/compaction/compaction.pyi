from furb.engine import Act, Filter

compaction: str
"""compaction is the doctrine of the extension, a quote that says what each part does and how a project changes the defaults."""
SHARE: float
"""SHARE is the share of the window of its actor that an answer reads past, after which compacted asks choice what to keep of its chain, and it is 0.6 unless a word rebinds it."""
CHOOSER: str
"""CHOOSER is the actor that choice asks when it is given none, and an empty CHOOSER is the default actor of the chain of the model that chooses."""
LISTED: int
"""LISTED is the most acts of one step that choice lists, and it is 40 unless a word rebinds it."""
BRIEF: str
"""BRIEF is what the prompt of choice says first: which acts to keep, and how to close."""
FILTERS: list[Filter]
"""FILTERS are the filters that compact applies in order when it is given none: latest(), then recent(8)."""

def together(*filters: Filter) -> Filter:
  """together(*filters) gives each filter what the one before it kept, in order, and gives what the last one kept."""

def latest() -> Filter:
  """latest() keeps of each path only the last read or write, so a text that a later read or write of that path replaced is gone, and it keeps every other act."""

def quiet() -> Filter:
  """quiet() drops each command with its output, and keeps the rung whose word ran it, so the word stays in the turns."""

def recent(n: int) -> Filter:
  """recent(n) keeps the rungs of the last n replies, and all that they made."""

def choice(to: str = "", on: str = "") -> Act:
  """choice() asks a model, on a chain of its own, which acts of the chain the work still needs, in a prompt of the shape tuple[list[str], str], the ids to keep and a summary of what the rest holds, and gives that prompt.
  The prompt says what to keep, then lists each open prompt with its message, then each act of the chain by its id and its words, with the thousands of characters of its text or its output when they pass a thousand, each act that a step made under that step, and no reply, run, wants, gate or stand.
  It lists of a step only the last LISTED acts that it made, and says how many it left out.
  """

async def chosen(to: str = "", on: str = "") -> Filter:
  """chosen() asks choice() and gives a take of the ids that it comes to."""

def reground(on: str = "") -> None:
  """reground() reads again each file of each read or write whose tell the chain holds, once each, in the order of the first read or write, shows only the lines that changed since the last text that the chain holds of it, and skips a file that it cannot read, so the chain sees each file that it kept as it is now."""

def supersede(keep: Filter | None = None, on: str = "") -> str:
  """supersede(keep) says on the chain of the caller a prefix of what the filter keep kept of its acts, everything that made what it kept, and every act of the chain, and gives the chain.
  So the chain goes on under its name with its module, its ladders and its acts, and only its turns are cut.
  """

def compact(filters: list[Filter] | None = None, summary: str = "", on: str = "") -> str:
  """compact() supersedes the chain of the caller with a filter that applies the filters in order, FILTERS when it is given none, so the chain goes on under its name with less, and gives that chain.
  The next word of that chain binds the summary that compact is given as the quote summary when there is one, and regrounds the chain.
  """

def compacted(on: str = "") -> Act:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, it asks choice() of the chain, and at the next reply on the chain once that came, it compacts the chain, which it wraps still.
  The chain keeps the acts that the model chose, each act said on the chain after it asked, and of each path only the last read or write, and the summary of the model.
  When the model gives no tuple of a list and a str, it compacts the chain with FILTERS and no summary of the model.
  It moves, stops and pauses nothing, so each open prompt, each rung that waits and each word that runs goes on in the chain.
  It never completes.
  """
