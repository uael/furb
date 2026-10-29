from typing import Final

from furb.engine import Act

CAP: Final[int] = 40
"""CAP is the number of lines that differ in a file past which a catch-up tells the path of the file with that number alone."""
BUDGET: Final[int] = 4096
"""BUDGET is the number of bytes of the paragraph of a catch-up past which the paragraph gives the paths alone."""

def catchup(on: str = "") -> Act:
  """The catch-up of a chain: an act on the chain that never completes, and that catches the chain up at each stand: it reads again, with a hidden show, the path of each text that the last read or write of that path on the chain gave, but a path of a scheme and a door, and it tells what changed in one paragraph headed caught up, about the chain.
  The life word of the extension is catchup(), and the extension plays on every chain, so every chain catches up from its birth.
  On the root it reads the clock as it starts and at each stand, once the chain told the standing that the stand gives it, so the record holds a reading of the clock at the tip of every life.
  The paragraph binds changed to the paths that changed, the most recent first, each to the lines that differ by their numbers, with None for a line past the new end, or to their count when they are more than CAP, or to gone when the read is refused.
  The paragraph binds hours to the hours between the last two readings of the clock of the root, to one decimal place, only when they are more than one, so it says the time since the life before.
  Past BUDGET bytes the paragraph gives the paths alone, the most recent first, as many as fit.
  A chain where nothing changed tells nothing at a stand, so its catch-up costs no token.
  The journal keeps each read and each clock of a catch-up, so a later life on the same record tells the same catch-up at the same place.
  It hears no control, as the chain that it serves hears none, so a pause of the chain holds no catch-up.
  """
