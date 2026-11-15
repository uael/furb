"""What the scripts that run a real life share: the life they open on the engine of this interpreter, the claude they
ask, and what the answers of a record cost.

The scripts run the engine of CPython, as the benches of DeepSWE need, on the World of `furb.world`, which opens the life on
the ears of the crate, the provider of the crate among them, and on the Kernel of the crate. The command
line `furb` runs the engine of the crate; a script runs it as a program of its own.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import TypedDict

from furb import engine, python
from furb.checkpoint import journal
from furb.kernel import kernel
from furb.record import recorded
from furb.world import Live, answered, kept

# The catalog of the crate stands in the module of the door that furb.world reads too, and furb_monty gives no other.
from furb_monty import _monty


def say(text: str) -> None:
  """One line to the operator, said as it happens, since a line of a long run that waits for the end says nothing."""
  sys.stdout.write(f"{text}\n")
  sys.stdout.flush()


def ready(actor: str | None = None) -> None:
  """Stop the script at once when its actor cannot be asked: a model of the claude command line, by its name or by its
  id alone, when the catalog of the crate finds no claude, or no model at all when the script names no actor and the
  catalog offers none, since the default actor is the first model that it offers. A claude on PATH is the default
  actor, so the catalog, which may ask a local server, is read only when none stands there."""
  if shutil.which(os.environ.get("FURB_CLAUDE_BIN") or "claude") is not None:
    return
  offered = _monty.models()
  # The catalog finds claude where PATH does not, as in the folders of Claude Desktop, and offers it first then.
  if offered and str(offered[0]["name"]).startswith("claude-cli:"):
    return
  if actor is None and not offered:
    say("the catalog offers no model, so no model can be asked and the script stops here")
    raise SystemExit(1)
  actor = actor or str(offered[0]["name"])
  model = _monty.model(actor) or _monty.model(actor.rsplit("/", 1)[0])
  if str(model["name"] if model else actor).startswith("claude-cli:"):
    say("no claude stands, so no model can be asked and the script stops here")
    raise SystemExit(1)


def lived(
  record: Path | None, cwd: Path, actor: str | None, *, keeps: bool, skills: bool = True
) -> tuple[Live, str, list[tuple]]:
  """One life on the loop that runs: its World, the ears of the crate as every host opens a life on them, the Kernel
  of the crate, and its root. The actor unsaid is the default actor of the crate.

  The life is made again from what the record holds, and it keeps what it says to the record when it keeps, through
  the store of the crate, which holds the lease of the record until the World ends. It enables at its tip the
  skills that the configs turn on, unless `skills` is false, and runs what its record enables either way.
  The journal says the whole record again before boot returns, so the life stands whole on its record when this
  gives the root.
  """
  world = Live(str(cwd.absolute()), actor)
  held = world.opened(record, keeps=keeps, skills=skills)
  root = engine.boot(
    journal(vars(python), recorded(vars(python), held)), world=world.hears(), kernel=kernel(vars(python)), **world.ears
  )
  return world, root, held


def bought(record: Path) -> list[tuple]:
  """Every answer of a model that the record holds, and none before the record is written."""
  return answered(kept(record)) if record.is_file() else []


def spent(record: Path) -> float:
  """The dollars that the answers of the record cost."""
  return sum(one[3][2][4] for one in bought(record) if one[3][2])


def deepswe(*words: str) -> str:
  """One verb of `cargo xtask deepswe`, which seeds, freezes and grades a task, and what it printed on stdout, while
  its log goes to stderr. It stops the script when the verb fails."""
  root = Path(__file__).resolve().parent.parent
  ran = subprocess.run(["cargo", "xtask", "deepswe", *words], cwd=root, stdout=subprocess.PIPE, text=True, check=False)  # noqa: S603, S607
  if ran.returncode:
    say(f"cargo xtask deepswe {' '.join(words)} failed, so the script stops here")
    raise SystemExit(1)
  return ran.stdout


class Seed(TypedDict):
  """A seeded task, as `cargo xtask deepswe seed` gives it: its workroot, its checkout, the commit of its base, its
  instruction as the task wrote it, and the words of the rig, which a bench puts to its own life."""

  work: str
  app: str
  base: str
  instruction: str
  asked: str
  again: str
  confident: float


def seeded(task: str) -> Seed:
  """A fresh checkout of a task."""
  return json.loads(deepswe("seed", task))


def outside() -> None:
  """Take the venv that runs the script out of the environment of its children, so a command of a model finds the
  python of the checkout and not the one of furb."""
  own = Path(sys.prefix).resolve()
  if own == Path(sys.base_prefix).resolve():
    return
  path = os.environ.get("PATH", "").split(os.pathsep)
  os.environ["PATH"] = os.pathsep.join(one for one in path if Path(one).resolve() != own / "bin")
  if (named := os.environ.get("VIRTUAL_ENV")) and Path(named).resolve() == own:
    del os.environ["VIRTUAL_ENV"]
