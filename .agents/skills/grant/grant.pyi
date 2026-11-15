from typing import Literal

from furb.engine import Act

def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells the dollars of that ledger at each answer of a model, since the advance binds the share before the next reply, so no turn a reply has sent grows a line after it.
  grant on a chain puts a ceiling on it: dollars, a share of the window, or both.
  The engine enters a pause on the chain when a response carries the ledger to its ceiling, whose cause is ceiling.
  The word of the response that crossed the ceiling runs.
  No reply follows the response that carried the ledger to the ceiling, until a wake.
  The model continues after a later grant and a wake.
  An answer that carries the ledger past the ceiling pauses the chain, so the word that answer brought runs and what it gave waits, and no rung of the chain asks until the wake.
  Lifting a ceiling wakes nothing: the pause stands until a wake, ceiling or no ceiling.
  It stands until it is lifted, as the chain it is on does, so what it comes to is what lifted it: nothing for a later grant that closes it, and a CancelledError for a cancel.
  A grant of nothing, of a ceiling under zero, or of a share past one is no ceiling: it is done with the refusal, which whoever made it takes by awaiting it, and it tells nothing, since it never stood; a prompt to no actor of the roster is closed the same way after it has told its open.
  A later grant that stands closes every grant of the chain before it that stands, and none that is over, so the ledger counts from the new one alone; one that is no ceiling closes nothing, and the ceiling that stands stands on.
  A cancel of it lifts the ceiling, since it is an act like any other.
  A reply that a cancel ended fills no share of the window, so an answer that its model gives after the cancel counts its dollars alone.
  A grant is any caller's, on any chain.
  A grant finds the grants of its chain among the acts of the life, so a grant on a chain with a source closes no grant of its origin.
  """

type Grant = tuple[Literal["grant"], str, str, str, float | None, float | None]
"""A grant carries the ceiling in dollars and the ceiling in share of the window."""
