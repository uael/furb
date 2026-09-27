"""What the scripts that run a real life share: the life they open on the engine of this interpreter, the claude they
ask, and what the answers of a record cost.

The scripts run the engine of CPython, as the DeepSWE rig needs, on the World of `furb.world`, which opens the life on
the ears of the crate, the provider of the crate among them, and on the Kernel and the gate of the crate. The command
line `furb` runs the engine of the crate; a script runs it as a program of its own.
"""

import os
import shutil
import sys
from pathlib import Path

import furb_monty
from furb import engine, python, sheet
from furb.kernel import kernel
from furb.world import Live, answered, kept


def say(text: str) -> None:
  """One line to the operator, said as it happens, since a line of a long run that waits for the end says nothing."""
  sys.stdout.write(f"{text}\n")
  sys.stdout.flush()


def ready() -> None:
  """Stop the script at once when no claude stands on PATH, since no model can be asked then."""
  if shutil.which(os.environ.get("FURB_CLAUDE_BIN") or "claude") is None:
    say("no claude on PATH, so no model can be asked and the script stops here")
    raise SystemExit(1)


def lived(
  record: Path | None, cwd: Path, actor: str | None, *, keeps: bool, extensions: bool = True
) -> tuple[Live, str, list[tuple]]:
  """One life on the loop that runs: its World, the ears of the crate as every host opens a life on them, the Kernel
  and the gate of the crate, and its root. The actor unsaid is the default actor of the crate.

  The life is made again from what the record holds, and it keeps what it says to the record when it keeps, through
  the store of the crate, which holds the lease of the record until the World ends. It enables at its tip the
  extensions that the configs turn on, unless `extensions` is false, and runs what its record enables either way.
  The journal says the whole record again before boot returns, so the life stands whole on its record when this
  gives the root.
  """
  world = Live(str(cwd.absolute()), actor)
  held = world.opened(record, keeps=keeps, extensions=extensions)
  gate = sheet.gating(vars(python), furb_monty.gate)
  root = engine.boot(held, world=world.hears(), kernel=kernel(vars(python)), gate=gate, **world.ears)
  return world, root, held


def bought(record: Path) -> list[tuple]:
  """Every answer of a model that the record holds, and none before the record is written."""
  return answered(kept(record)) if record.is_file() else []


def spent(record: Path) -> float:
  """The dollars that the answers of the record cost."""
  return sum(one[3][2][4] for one in bought(record) if one[3][2])
