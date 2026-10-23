from furb.engine import Act, Filter

compaction: str
"""compaction is the doctrine of the extension, a quote that says what each part does and how a project changes the defaults."""
SHARE: float
"""SHARE is the share of the window of its actor that an answer reads past, after which compacted asks choice what to keep of its chain, and it is 0.6 unless a word rebinds it."""
LABEL: str
"""LABEL is the label of the chain that compact makes when it is given none, and it is compacted unless a word rebinds it."""
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

def restore(source: str, on: str = "") -> None:
  """restore(source) binds in the chain a copy of each name that the module of the source holds, but a callable, a name that starts with two underscores and a name that holds the object that the chain holds already, so the chain holds what a word of the source bound or changed, though that word is not over.
  The chain owns what restore binds: it copies each list, dict and set, and what each holds, and shares every other object.
  A rung of restore binds each name that it binds and that the chain did not hold to its value in the globals of the chain, so the gate reads a word of the chain that names it.
  """

def reground(on: str = "") -> None:
  """reground() reads again each file that the chain holds a read or write of from another chain, once each, in the order of the first read or write, shows only the lines that changed since the last text that the chain holds of it, and skips a file that it cannot read, so the chain sees each file that it kept as it is now."""

def compact(filters: list[Filter] | None = None, label: str = "", summary: str = "", on: str = "") -> str:
  """compact() makes a chain whose source is the chain of the caller, whose filter applies the filters in order, FILTERS when it is given none, and gives that chain, which it does not prompt.
  The first word of that chain restores the chain of the caller, binds the summary that compact is given as the quote summary when there is one, and regrounds the new chain.
  """

def compacted(on: str = "") -> Act:
  """compacted() wraps a chain: after an answer that reads more than SHARE of the window of its actor, it asks choice() of the chain, and at the next reply on the chain once that came, it compacts the chain and wraps the new chain in turn.
  The new chain keeps the acts that the model chose but each rung that is not over, each act said on the chain after it asked, and of each path only the last read or write, and the summary of the model.
  When the model gives no tuple of a list and a str, it compacts the chain with FILTERS and no summary of the model.
  It moves each open prompt of a model to the new chain, and closes the old prompt with what the new one comes to.
  It stops each open rung of a prompt that it moves with a close, which reaches that rung alone, and it pauses nothing, so what such a rung started goes on, as the workers that a conductor hired do.
  A rung of its own on each prompt that it moves keeps that prompt from asking its model again.
  The summary names each rung that it stopped, the act that it awaited and its word, and says to run no line before that await again, so a word that loops runs again in the new chain only when a word runs it again, on what restore bound.
  A prompt of a model that asks its model on a chain that it compacted moves to the newest chain at that reply.
  It moves no prompt to the operator and no prompt that made it, and it never completes.
  """
