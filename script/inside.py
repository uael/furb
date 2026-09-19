"""The suite, run without pytest, so that it can run where the engine runs.

The suite drives the engine through its public API alone, it takes no fixture, and it reaches pytest for one thing
only: that a call raises. So it needs no test runner, and a runner of a few lines can carry it into a sandbox and
hold the engine to every sentence of the contract in there, which is the only place that proof means anything.

    uv run python script/inside.py

`test/test_hygiene.py` is left out. It holds the contract and the suite to the hygiene laws, by reading the files
with a tokeniser and a minifier, and it is no test of the engine.
"""

import asyncio
import re
import sys
import traceback
from collections.abc import Callable, Generator, Sequence
from contextlib import contextmanager
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


@contextmanager
def raises(kind: type[BaseException], match: str = "") -> Generator[None]:
  """The one thing the suite asks of a test runner: that a call raises, and what it says when it does."""
  try:
    yield
  except kind as got:
    if match and not re.search(match, str(got)):
      why = f"{got!r} does not match {match!r}"
      raise RaisedNothingError(why) from got
  else:
    why = f"nothing raised {kind.__name__}"
    raise RaisedNothingError(why)


def fixture(one: Callable[..., object]) -> Callable[..., object]:
  """A fixture of the harness, which no test of the suite takes, so it stands as the function it is."""
  return one


def shim() -> ModuleType:
  """A pytest of two names, which is all the suite reads of it: that a call raises, and that a name is made."""
  made = ModuleType("pytest")
  made.raises = raises  # ty: ignore[unresolved-attribute]
  made.fixture = fixture  # ty: ignore[unresolved-attribute]
  return made


def at(one: Callable[[], object]) -> int:
  """The line a test stands on, which is the order the file holds them in."""
  code = getattr(one, "__code__", None)
  return getattr(code, "co_firstlineno", 0)


def tests(module: ModuleType) -> list[tuple[str, Callable[[], object]]]:
  """Every test of one file, in the order the file holds them."""
  found = [(name, got) for name, got in vars(module).items() if name.startswith("test_") and callable(got)]
  return sorted(found, key=lambda pair: at(pair[1]))


def ran(one: Callable[[], object]) -> None:
  """One test, run: a test that is async is run on a loop of its own, as the suite runs each of them."""
  got = one()
  if asyncio.iscoroutine(got):
    asyncio.run(got)


def suite(files: Sequence[Path]) -> tuple[int, list[tuple[str, str]]]:
  """The suite, run file by file: how many tests passed, and what each one that failed said."""
  done, failed = 0, []
  for path in files:
    module = __import__(path.stem)
    for name, one in tests(module):
      try:
        ran(one)
        done += 1
      except BaseException:
        failed.append((f"{path.stem}.{name}", traceback.format_exc()))
  return done, failed


def main() -> None:
  """The suite of the engine, without pytest, and what it came to."""
  sys.modules.setdefault("pytest", shim())
  sys.path[:0] = [str(SUITE), str(ROOT)]
  files = [p for p in sorted(SUITE.glob("test_*.py")) if p.stem not in APART]
  done, failed = suite(files)
  for name, why in failed:
    sys.stdout.write(f"FAILED {name}\n{why}\n")
  sys.stdout.write(f"{done} passed, {len(failed)} failed, in {len(files)} files\n")
  raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
  main()
