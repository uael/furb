"""The suite, run without pytest, so that it can run where the engine runs.

The suite drives the engine through its public API alone, it takes no fixture, and it reaches pytest for one thing
only: that a call raises. So it needs no test runner, and a runner of a few lines can carry it into a sandbox and
hold the engine to every sentence of the contract in there, which is the only place that proof means anything.

    uv run python script/inside.py

What runs the tests is [`suite`], which asks for nothing of a machine: no disk, no import, no traceback, no
context manager and no module of the interpreter. It is given the modules of the suite as they stand and it runs
what they hold. Everything that needs this machine is in [`main`], which reads the files and makes the modules
the way this interpreter makes them, so a sandbox makes them its own way and runs the same suite.

`test/test_hygiene.py` is left out. It holds the contract and the suite to the hygiene laws, by reading the files
with a tokeniser and a minifier, and it is no test of the engine.
"""

import asyncio
import re
import sys
import traceback
from collections.abc import Callable, Sequence
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parent.parent
"""ROOT is the root of the repository, which holds the suite and the package."""
SUITE = ROOT / "test"
"""SUITE is where the tests of the engine stand, one file per definition of the contract."""
APART = {"test_hygiene"}
"""APART are the files that are no test of the engine, which hold the contract and the suite to the hygiene laws."""


class RaisedNothingError(AssertionError):
  """What a call that had to raise, and did not, fails with."""


class raises:  # noqa: N801
  """The one thing the suite asks of a test runner: that a call raises, and what it says when it does.

  It is a class and no context manager of the library, since a sandbox holds no such library, and what a `with`
  asks of a value is these two names and nothing else.
  """

  def __init__(self, kind: type[BaseException], match: str = "") -> None:
    """The exception the call must raise, and what its text must hold."""
    self.kind = kind
    self.match = match

  def __enter__(self) -> "raises":  # noqa: UP037
    """The call stands inside this, and what it raises is read at the end of it."""
    return self

  def __exit__(self, kind: type[BaseException] | None, got: BaseException | None, held: object) -> bool:
    """What the call raised, read: the one that was wanted is taken, and anything else goes on."""
    if got is None:
      why = f"nothing raised {self.kind.__name__}"
      raise RaisedNothingError(why)
    if not isinstance(got, self.kind):
      return False
    if self.match and not re.search(self.match, str(got)):
      why = f"{got!r} does not match {self.match!r}"
      raise RaisedNothingError(why) from got
    return True


def fixture(one: Callable[..., object]) -> Callable[..., object]:
  """A fixture of the harness, which no test of the suite takes, so it stands as the function it is."""
  return one


def shim() -> ModuleType:
  """A pytest of two names, which is all the suite reads of it: that a call raises, and that a name is made.

  A module of the interpreter is what this machine puts in sys.modules. A sandbox puts what its own imports find,
  and holds the same two names.
  """
  made = ModuleType("pytest")
  made.raises = raises  # ty: ignore[unresolved-attribute]
  made.fixture = fixture  # ty: ignore[unresolved-attribute]
  return made


def at(one: Callable[[], object]) -> int:
  """The line a test stands on, which is the order the file holds them in."""
  code = getattr(one, "__code__", None)
  return getattr(code, "co_firstlineno", 0)


def tests(module: object) -> list[tuple[str, Callable[[], object]]]:
  """Every test of one module, in the order the file of it holds them."""
  held = module if isinstance(module, dict) else vars(module)
  found = [(name, got) for name, got in held.items() if name.startswith("test_") and callable(got)]
  return sorted(found, key=lambda pair: at(pair[1]))


def ran(one: Callable[[], object]) -> None:
  """One test, run: a test that is async is run on a loop of its own, as the suite runs each of them."""
  got = one()
  if hasattr(got, "__await__"):
    asyncio.run(got)  # ty: ignore[invalid-argument-type]


def suite(modules: Sequence[tuple[str, object]]) -> tuple[int, list[tuple[str, BaseException]]]:
  """The suite, run module by module: how many tests passed, and what each one that failed raised.

  What failed carries the exception itself and no text of it, since what makes the text of an exception is the
  machine the suite runs on, and this runs on any of them.
  """
  done, failed = 0, []
  for name, module in modules:
    for test, one in tests(module):
      try:
        ran(one)
        done += 1
      except BaseException as no:
        failed.append((f"{name}.{test}", no))
  return done, failed


def main() -> None:
  """The suite of the engine, without pytest, and what it came to."""
  sys.modules.setdefault("pytest", shim())
  sys.path[:0] = [str(SUITE), str(ROOT)]
  files = [p for p in sorted(SUITE.glob("test_*.py")) if p.stem not in APART]
  modules = [(path.stem, __import__(path.stem)) for path in files]
  done, failed = suite(modules)
  for name, no in failed:
    held = "".join(traceback.format_exception(type(no), no, no.__traceback__))
    sys.stdout.write(f"FAILED {name}\n{held}\n")
  sys.stdout.write(f"{done} passed, {len(failed)} failed, in {len(files)} files\n")
  raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
  main()
