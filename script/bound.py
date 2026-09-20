"""The tests of the python binding, with the binding built for the interpreter that runs them.

`bind/python` is the crate's surface for a host of python: one life of the engine, in the sandbox, with a World
the host writes. The module it makes is compiled against one interpreter and imported by that one alone, so this
rig builds it for the interpreter it is run by and then puts pytest on it.

    uv run python script/bound.py

It takes the arguments of pytest after its own, so `uv run python script/bound.py -k voice` runs one of them.
"""

import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig."""
ROOT = HERE.parent
"""ROOT is the root of the repository, which cargo is run from."""
BUILT = ROOT / "target" / "debug" / "libfurb_sand.so"
"""BUILT is the library cargo makes, which is the module under another name."""


def main() -> int:
  """Build the binding for this interpreter, put the module where python finds it, and run its tests."""
  # The binding is compiled against one interpreter. `sys._base_executable` is the one that holds the headers,
  # which a virtual environment does not, so it is what pyo3 is pointed at.
  # `sys._base_executable` is not in the stubs, since it is not of the public surface, and it is the one name
  # that gives the interpreter behind a virtual environment.
  base: str = getattr(sys, "_base_executable", sys.executable)
  held = dict(os.environ, PYO3_PYTHON=base)
  built = subprocess.run(
    ["cargo", "build", "-p", "furb-sand", "--no-default-features"],  # noqa: S607
    cwd=ROOT,
    env=held,
    check=False,
  )
  if built.returncode != 0:
    return built.returncode
  if not BUILT.is_file():
    sys.stderr.write(f"cargo made no {BUILT}\n")
    return 1

  # The module is `furb_sand`, and cargo names the library `libfurb_sand.so`, so it is linked under the name
  # python imports it by, in a directory of the build and not of the source.
  where = ROOT / "target" / "python"
  where.mkdir(parents=True, exist_ok=True)
  module = where / "furb_sand.so"
  module.unlink(missing_ok=True)
  module.symlink_to(BUILT)

  # The suite of the engine is imported by name from its own directory, so the tests of the binding say where
  # theirs is too, and the module the build made comes before either.
  path = [str(where), str(ROOT / "bind" / "python" / "test"), held.get("PYTHONPATH", "")]
  ran = subprocess.run(  # noqa: S603
    [sys.executable, "-m", "pytest", "-q", "--no-cov", "-p", "no:cacheprovider", "bind/python/test", *sys.argv[1:]],
    cwd=ROOT,
    env=dict(held, PYTHONPATH=os.pathsep.join(one for one in path if one)),
    check=False,
  )
  return ran.returncode


if __name__ == "__main__":
  raise SystemExit(main())
