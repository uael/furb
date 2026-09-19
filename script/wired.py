"""The suite, with every World of the harness behind the boundary of a host that is not python.

A crate that runs the engine outside python gives it a World that is no generator. `src/preamble.py` stands in its
place: it makes every fact plain, hands it to the host, and says back what the host answers. This rig runs the
whole suite with each World double of the harness behind that boundary, so that what crosses is plain data alone
and nothing of python. A suite that is green here says the boundary carries every fact the engine says and every
answer a World gives.

    uv run python script/wired.py

It takes the arguments of pytest after its own, so `uv run python script/wired.py test/test_bash.py` runs one file.
"""

import builtins
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig, which holds the plugin it runs pytest with."""
SRC = HERE.parent / "src"
"""SRC holds the preamble, which is the boundary this rig proves."""

if str(SRC) not in sys.path:
  sys.path.insert(0, str(SRC))

from furb import engine  # noqa: E402
from preamble import outside, unwire, wire  # noqa: E402

NAMES = {**vars(builtins), **vars(engine)}
"""NAMES is what a plain value is made again by: the names of the interpreter and the names of the engine."""


def crossed(made):  # noqa: ANN001, ANN201
  """One World double behind the boundary: a host that drives it, and the generator that carries facts to it.

  The host says no fact itself. It hands back what the double said, and the boundary says each of them, which is
  the one way a host of the outside speaks.
  """

  def hears(self):  # noqa: ANN001, ANN202
    held = {"g": made(self)}
    held["g"].send(None)

    def host(_name: str, said: object) -> object:
      got = unwire(said, NAMES)
      out = []
      try:
        back = held["g"].send(tuple(got) if isinstance(got, list) else got)
        while back is not None:
          out.append(wire(back))
          back = held["g"].send(None)
      except StopIteration:
        held["g"] = made(self)
        held["g"].send(None)
      return ("say", out)

    return outside("world", host, NAMES)

  return hears


def pytest_sessionstart(session: object) -> None:  # noqa: ARG001
  """Every World double of the harness, as the suite loaded it, put behind the boundary where it stands."""
  done = set()
  for mod in list(sys.modules.values()):
    for name in ("Sand", "Dead", "Where"):
      cls = getattr(mod, name, None)
      if isinstance(cls, type) and "hears" in cls.__dict__ and id(cls) not in done:
        done.add(id(cls))
        cls.hears = crossed(cls.__dict__["hears"])


def main() -> None:
  """The suite through the boundary, run as pytest with this file as its plugin."""
  args = sys.argv[1:] or ["test", "--ignore=test/outside"]
  said = subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "--no-cov", "-p", "wired", *args],
    cwd=HERE.parent,
    env={**__import__("os").environ, "PYTHONPATH": f"{HERE}:{SRC}"},
    check=False,
  )
  raise SystemExit(said.returncode)


if __name__ == "__main__":
  main()
