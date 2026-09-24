from typing import Final, Literal

from furb.engine import Act

WINDOW: Final[int] = 200000
"""WINDOW is the window, in tokens, of a model whose roster entry does not say one.
The window that a roster entry leaves unsaid is the window that the file names.
"""

def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells that ledger at each answer of a model, so no turn an ask has sent grows a line after it.
  grant on a chain puts a ceiling on it: dollars, a share of the window, or both.
  A grant enters a pause on the chain when a response carries the ledger to its ceiling.
  The word of the response that crossed the ceiling runs.
  No ask follows the response that carried the ledger to the ceiling, until a wake.
  The model continues after a later grant and a wake.
  An answer that carries the ledger past the ceiling pauses the chain, so the word that answer brought runs and what it gave waits, and no rung of the chain asks until the wake.
  Lifting a ceiling wakes nothing: the pause stands until a wake, ceiling or no ceiling.
  It stands until it is lifted, as the chain it is on does, so what it comes to is what lifted it: nothing for a later grant that closes it, and a CancelledError for a cancel.
  A grant of nothing, of a ceiling under zero, or of a share past one is no ceiling: it is done with the refusal, which whoever made it takes by awaiting it, and it tells nothing, since it never stood.
  A later grant that stands closes every grant of the chain before it that stands, and none that is over, so the ledger counts from the new one alone; one that is no ceiling closes nothing, and the ceiling that stands stands on.
  A cancel of it lifts the ceiling, since it is an act like any other.
  A grant is any caller's, on any chain.
  A grant finds the grants of its chain among the acts of the life, so a grant on a chain with a source closes no grant of its origin.
  A grant reads the window of a rung off the standing that the chain it is on stood on when it heard that rung.
  """

type Grant = tuple[Literal["grant"], str, str, str, float | None, float | None]
"""A grant carries the ceiling in dollars and the ceiling in share of the window."""
