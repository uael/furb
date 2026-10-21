from typing import Final

from furb.engine import Act

SHARE: Final[float] = 0.6
"""SHARE is the share of the window of an actor that an answer of a model reads past, at which compaction tells the model to go on in a new chain."""

def compaction(on: str = "") -> Act:
  """The fill of a chain, watched: an act on the chain that, at the first answer of a model on the chain that reads more than SHARE of the window of its actor, tells the share that the answer filled and the word that goes on in a chain with a source and a take of the old work, once, and that never completes, so its end asks the model nothing.
  The share is what filled of the engine gives of the reply, so compaction tells the share that the advance binds, to two places.
  An answer that reads SHARE of the window or less tells nothing, and the act watches on.
  The word that it tells opens a chain whose source is the chain, and whose filter is a take that is not inside of each prompt on the chain that is done, so the new chain keeps the work that is still open.
  It does not act for the model: it tells the word and runs nothing, so the model decides whether and when to go on in the new chain.
  The life word of the extension is compaction(), so each chain watches its fill from its birth, and a host turns it off as it turns off any extension, with false in a config.
  """
