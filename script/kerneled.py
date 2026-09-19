"""The suite, with the Kernel of the crate running every word in the sandbox.

The engine of python stays where it is. What changes is where the word of a rung runs: the crate takes it into a
sandbox of monty, where it reaches the engine through the verbs it calls and reaches nothing else. A model writes
that word, so this is the one part of a life that nobody trusts.

    cargo build --features python
    uv run python script/kerneled.py

The crate is loaded as `furbc`, which the build leaves in `target/debug`. It takes the arguments of pytest after
its own, so `uv run python script/kerneled.py test/test_bash.py` runs one file.
"""

import importlib
import os
import subprocess
import sys
from pathlib import Path

from furb import engine

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig, which holds the plugin it runs pytest with."""
ROOT = HERE.parent
"""ROOT is the root of the repository, where the crate is built."""
BUILT = ROOT / "target" / "debug" / "libfurb.so"
"""BUILT is the crate as cargo leaves it, which this rig names furbc for python."""
EXT = ROOT / "target" / "ext"
"""EXT is where the rig puts the crate under the name python imports it by."""


def crate() -> Path:
  """The crate, under the name python imports it by, and a word of what to do when it is not built."""
  if not BUILT.is_file():
    why = f"{BUILT} is not there: build the crate with `cargo build --features python` first"
    raise SystemExit(why)
  EXT.mkdir(parents=True, exist_ok=True)
  at = EXT / "furbc.so"
  if at.is_symlink() or at.exists():
    at.unlink()
  at.symlink_to(BUILT)
  return EXT


def pytest_sessionstart(session: object) -> None:  # noqa: ARG001
  """The Kernel of the suite, replaced by the Kernel of the crate wherever the suite made one."""
  # The crate is built by cargo and put on the path by main, so it is named here and not imported at the top.
  furbc = importlib.import_module("furbc")

  def kernel(self: object) -> object:
    """The Kernel of the crate for this life, which runs every word of it in the sandbox."""
    _ = self
    return furbc.Kernel(engine)

  for mod in list(sys.modules.values()):
    cls = getattr(mod, "Py", None)
    if isinstance(cls, type) and "kernel" in cls.__dict__:
      cls.kernel = kernel


def main() -> None:
  """The suite with the crate under it, run as pytest with this file as its plugin."""
  args = sys.argv[1:] or ["test", "--ignore=test/outside"]
  said = subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "--no-cov", "-p", "kerneled", *args],
    cwd=ROOT,
    env={**os.environ, "PYTHONPATH": f"{HERE}:{crate()}"},
    check=False,
  )
  raise SystemExit(said.returncode)


if __name__ == "__main__":
  main()
