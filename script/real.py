"""What the scripts that run a real life share: the life they open on the engine of this interpreter, the claude they
ask, and what the answers of a record cost.

The scripts run the engine of CPython, as the DeepSWE rig needs, on the World of `furb.world`, the ears of the crate,
the provider of the crate among them, and the Kernel and the gate of the crate. The command line `furb` runs the
engine of the crate; the scripts do not.
"""

import os
import shutil
import sys
from collections.abc import Sequence
from pathlib import Path

import furb_monty
from furb import engine, python, sheet
from furb.kernel import kernel
from furb.world import Live, answered, entries, kept
from furb_monty import _monty

ACTOR = "claude-cli:opus/low"
"""ACTOR is the actor a script asks when it names none, which is opus of the claude command line at the least effort
it takes."""


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
  record: Path | None, cwd: Path, actor: str, *, keeps: bool, extensions: bool = True
) -> tuple[Live, str, list[tuple]]:
  """One life on the loop that runs: its World, the provider of the crate, then the ears of the crate as every host
  opens a life on them, the Kernel and the gate of the crate, and its root.

  The life is made again from what the record holds, and it keeps what it says to the record when it keeps, through
  the store of the crate, which holds the lease of the record until the World ends. A life that only reads a record
  keeps nothing, since every life stands as it opens, and a life that keeps keeps that stand. It enables at its tip
  the extensions that the configs turn on, unless `extensions` is false, and runs what its record enables either
  way. The journal says the whole record again before boot returns, so the life stands whole on its record when this
  gives the root.
  """
  world = Live(str(cwd.absolute()), actor)
  if keeps and record is not None:
    record.parent.mkdir(parents=True, exist_ok=True)
  where = None if record is None else str(record)
  stored, ears = _monty.opened(world.directory, where, keeps=keeps, extensions=extensions)
  world.provider()
  world.ears.update(ears)
  gate = sheet.gating(vars(python), furb_monty.gate)
  held = entries(stored)
  root = engine.boot(held, world=world.hears(), kernel=kernel(vars(python)), gate=gate, **world.ears)
  return world, root, held


def again(held: Sequence[tuple], root: str, shape: type | None, message: str, to: str) -> str:
  """The name of the prompt the record already holds for this message, and nothing when it holds none.

  The engine matches nothing the operator says again, so a life stood up on its own record would open a second
  prompt beside the one that record stands on, and ask a model for what it was answered once.
  """
  named = shape.__name__ if isinstance(shape, type) else repr(shape)
  for entry in held:
    match entry:
      case (("prompt", id, "operator", on, kind, said, who),) if (on, kind, said, who) == (root, named, message, to):
        return id
  return ""


async def turned(record: Path, cwd: Path, actor: str = ACTOR, *, extensions: bool = True) -> None:
  """The turns of the root of a life made again from its record, on the actor that life stood on, each as the
  python a model reads of it."""
  world, root, _ = lived(record, cwd, actor, keeps=False, extensions=extensions)
  try:
    for role, py, _, _ in engine.turns(on=root):
      say(f"[{role}] {py}")
  finally:
    world.end()


def bought(record: Path) -> list[tuple]:
  """Every answer of a model that the record holds, and none before the record is written."""
  return answered(kept(record)) if record.is_file() else []


def spent(record: Path) -> float:
  """The dollars that the answers of the record cost."""
  return sum(one[3][2][4] for one in bought(record) if one[3][2])
