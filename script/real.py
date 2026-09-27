"""What the scripts that run a real life share: the claude they ask, and what the answers of a record cost."""

import os
import shutil
from pathlib import Path

from furb.cli import say
from furb.provider.claude import BIN
from furb.world import answered, kept


def ready() -> None:
  """Stop the script at once when no claude stands on PATH, since no model can be asked then."""
  if shutil.which(os.environ.get(BIN) or "claude") is None:
    say("no claude on PATH, so no model can be asked and the script stops here")
    raise SystemExit(1)


def bought(record: Path) -> list[tuple]:
  """Every answer of a model that the record holds, and none before the record is written."""
  return answered(kept(record)) if record.is_file() else []


def spent(record: Path) -> float:
  """The dollars that the answers of the record cost."""
  return sum(one[3][2][4] for one in bought(record) if one[3][2])
