"""The tests of the bindings, with each one built for the interpreter that runs it.

`bind/python` and `bind/js` are the crate's surface for a host that is not rust: one life of the engine, in the
sandbox, with a World the host writes. The module each one makes is compiled against one interpreter and read by
that one alone, so this rig builds each for the interpreter it is run by and then puts its tests on it.

    uv run python script/bound.py

It takes the arguments of pytest after its own, so `uv run python script/bound.py -k voice` narrows the tests of
the python binding; the tests of the javascript one run whole either way.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig."""
ROOT = HERE.parent
"""ROOT is the root of the repository, which cargo is run from."""


def main() -> int:
  """Build each binding for the interpreter that reads it, and run the tests of each."""
  # The python binding is compiled against one interpreter. `sys._base_executable` is the one that holds the
  # headers, which a virtual environment does not, so it is what pyo3 is pointed at. The name is not in the
  # stubs, since it is not of the public surface, and it is the one that gives the interpreter behind a virtual
  # environment.
  base: str = getattr(sys, "_base_executable", sys.executable)
  held = dict(os.environ, PYO3_PYTHON=base)

  got = bound(held)
  return got if got != 0 else noded(held)


def bound(held: dict[str, str]) -> int:
  """The python binding, built and held to its tests."""
  made = built(held, "furb-sand")
  if made is None:
    return 1

  # The module is `furb_sand`, and cargo names the library `libfurb_sand.so`, so it is linked under the name
  # python imports it by, in a directory of the build and not of the source.
  where = ROOT / "target" / "python"
  where.mkdir(parents=True, exist_ok=True)
  module = where / "furb_sand.so"
  module.unlink(missing_ok=True)
  module.symlink_to(made)

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


def noded(held: dict[str, str]) -> int:
  """The javascript binding, built and held to its tests.

  A machine with no node runs nothing here and says so, since the binding is no less built for it.
  """
  made = built(held, "furb-js")
  if made is None:
    return 1

  # Node reads a native module by the name it is given, and cargo names a library for the platform, so the built
  # one is put under the name node reads it as. A hard link costs nothing and keeps the name.
  where = ROOT / "target" / "node"
  where.mkdir(parents=True, exist_ok=True)
  module = where / "furb.node"
  module.unlink(missing_ok=True)
  os.link(made, module)

  if shutil.which("node") is None:
    sys.stderr.write("no node on this machine, so the tests of the javascript binding are not run\n")
    return 0
  ran = subprocess.run(
    ["node", "--test", "bind/js/test/*.test.mjs"],  # noqa: S607
    cwd=ROOT,
    env=held,
    check=False,
  )
  return ran.returncode


def built(held: dict[str, str], crate: str) -> Path | None:
  """One binding, built, and the library cargo made of it."""
  ran = subprocess.run(  # noqa: S603
    ["cargo", "build", "-p", crate, "--no-default-features"],  # noqa: S607
    cwd=ROOT,
    env=held,
    check=False,
  )
  if ran.returncode != 0:
    return None
  made = ROOT / "target" / "debug" / f"lib{crate.replace('-', '_')}.so"
  if not made.is_file():
    sys.stderr.write(f"cargo made no {made}\n")
    return None
  return made


if __name__ == "__main__":
  raise SystemExit(main())
