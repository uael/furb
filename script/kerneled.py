"""The suite, with the Kernel the crate ships in place of the one the harness makes.

The crate runs the engine in a sandbox, and the Kernel it gives that life is `kernel` in `src/preamble.py`: it
gates the word of a rung with the host and runs that word in the module of its chain. That Kernel is python, so it
can be held to the contract here, in this interpreter, before the sandbox can run it. This rig hands it to every
life of the suite and asks the host for the gate the harness itself would give, so that what is proved is the
Kernel and not the gate.

    uv run python script/kerneled.py

It takes the arguments of pytest after its own, so `uv run python script/kerneled.py test/test_rung.py` runs one
file.
"""

import builtins
import os
import subprocess
import sys
from collections import ChainMap
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig, which holds the plugin it runs pytest with."""
SRC = HERE.parent / "src"
"""SRC holds the preamble, which is the Kernel this rig proves."""

if str(SRC) not in sys.path:
  sys.path.insert(0, str(SRC))

from furb import engine  # noqa: E402
from preamble import kernel, unwire  # noqa: E402

NAMES = ChainMap(vars(engine), vars(builtins))
"""NAMES is what the Kernel reaches the engine through, live: boot binds the bus into the engine, and a name read
here is the one that stands now and not the one that stood when this rig was read."""


def kerneled(self):  # noqa: ANN001, ANN201
  """The Kernel of the crate for one life, with the gate of the harness behind the host it asks.

  The gate is the double's own, read off the double each time it is asked, so a harness that makes a stricter one
  is the one that answers.
  """

  def made():  # noqa: ANN202
    def host(_name: str, said: object) -> object:
      """The host of this rig: it answers the one question the Kernel asks, which is the gate."""
      asked = unwire(said, NAMES)
      assert isinstance(asked, tuple)
      _, word, ladder, returns = asked
      return self.gate(word, list(ladder), returns)

    held = kernel("kernel", host, NAMES)
    ran = self.ran

    def watched():
      """The Kernel, with every word it is told to run kept, which is what the harness reads of its own."""
      next(held)
      a = yield None
      while True:
        said = held.send(a)
        if a and a[0] == "run":
          ran.append(a[4])
        a = yield said

    return watched()

  return made()


def pytest_sessionstart(session: object) -> None:  # noqa: ARG001
  """Every Kernel the harness makes, replaced by the one the crate ships, wherever the suite loaded it."""
  done = set()
  for mod in list(sys.modules.values()):
    cls = getattr(mod, "Py", None)
    if isinstance(cls, type) and "kernel" in cls.__dict__ and id(cls) not in done:
      done.add(id(cls))
      cls.kernel = kerneled


def main() -> None:
  """The suite with the Kernel of the crate under it, run as pytest with this file as its plugin."""
  args = sys.argv[1:] or ["test", "--ignore=test/outside"]
  said = subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "--no-cov", "-p", "kerneled", *args],
    cwd=HERE.parent,
    env={**os.environ, "PYTHONPATH": f"{HERE}:{SRC}"},
    check=False,
  )
  raise SystemExit(said.returncode)


if __name__ == "__main__":
  main()
