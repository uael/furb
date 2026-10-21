from furb.engine import Act, Filter

compaction: str
"""compaction is the doctrine of the extension, a quote that says what each part does and how a project changes the defaults."""
SHARE: float
"""SHARE is the share of the window of its actor that an answer reads past, after which compacted compacts its chain, and it is 0.6 unless a word rebinds it."""
LABEL: str
"""LABEL is the label of the chain that compact makes when it is given none, and it is compacted unless a word rebinds it."""
CHOOSER: str
"""CHOOSER is the actor that chosen asks when it is given none, and an empty CHOOSER is the default actor of the chain of the model that chooses."""
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

async def chosen(to: str = "", on: str = "") -> Filter:
  """chosen() lists each act of the chain, by its id and its words, in a prompt to a model on a chain of its own, which gives the ids to keep as list[str], and it gives a take of those ids."""

def reground(source: str, on: str = "") -> None:
  """reground(source) reads again each file that the source read or wrote, once each, in the order of the first read or write, and skips a file that it cannot read, so the chain sees each file as it is now and no old text of it."""

def compact(filters: list[Filter] | None = None, label: str = "", on: str = "") -> str:
  """compact() makes a chain whose source is the chain of the caller, whose filter applies the filters in order, FILTERS when it is given none, and whose first word is reground of that chain, and gives that chain, which it does not prompt."""

def compacted(on: str = "") -> Act:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, at the next reply on the chain, it compacts the chain, moves each open prompt of a model to the new chain, closes the old prompt with what the new one comes to, and wraps the new chain in turn.
  It moves no prompt to the operator and no prompt that made it, and it never completes.
  """
