"""The extension: one engine in the sandbox, the ears of the crate, and the gate of the crate, reached from python.

Every value that crosses is what monty carries, made python: none, a truth, a number, a text, a list, a tuple, a
map, an instance of a class of the engine as the instance it is, an exception as the one object it is for the
life, a name of the engine as the name this interpreter holds, and a callable the engine made as one that calls
it back. A generator of this interpreter crosses as an ear, an ear of the crate as itself, and a function as one
the sandbox calls back.
"""

from collections.abc import Callable, Iterable
from typing import final

@final
class NativeEar:
  """An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear."""

  def dispose(self) -> None:
    """The ear is let go before any engine hears it, so what it holds goes: a store lets its record go."""

@final
class Engine:
  """One engine, held on the thread of python, which says each name of the contract by its name. A verb that makes an
  act gives its name."""

  @staticmethod
  def boot(record: Iterable[object], ears: Iterable[tuple[str, object]]) -> Engine:
    """An engine, opened from the record, on these ears, each a generator of python or an ear of the crate under the
    name the engine hears it by, in the order the engine offers them a question. It is booted in the running loop,
    which drives it when an ear of the crate speaks from a thread of its own."""

  @property
  def root(self) -> str:
    """The root chain of the life, which is the first act of any record."""

  @property
  def raised(self) -> BaseException | None:
    """What boot raised, as the exception it is, and nothing when it raised nothing."""

  def site(self, value: str | None = None) -> str:
    """Who speaks in the life, and who speaks from now on when a name is given."""

  def verb(self, name: str, args: Iterable[object], kwargs: dict[str, object]) -> object:
    """One name of the engine, said by its name with these words, and what it gave."""

  def made(self, n: int, args: Iterable[object], kwargs: dict[str, object]) -> object:
    """One callable the engine made, called back by the handle it crossed under, with these words."""

  def forget(self, n: int) -> None:
    """A callable the engine made, forgotten: python holds its handle no more."""

  def watch(self, act: str, then: Callable[[object], object]) -> None:
    """What to call when an act is done, with what it came to: at once for one done already, and once otherwise."""

  def pump(self) -> None:
    """The engine driven as far as it goes: what the voices of its ears said is said into it."""

  def dispose(self) -> None:
    """The engine is gone, and its ears with it."""

def files() -> NativeEar:
  """The ear of the files, which reads and writes a path."""

def bash() -> NativeEar:
  """The ear of commands, which runs each in a shell of this machine."""

def time() -> NativeEar:
  """The ear of time, which reads the clock, draws a chance, and ends a wait."""

def store(path: str) -> tuple[list[object], NativeEar]:
  """The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says
  to keep."""

def kept(path: str) -> list[object]:
  """What the store kept at a path, read with no lease and changed in nothing."""

def gate(sheet: str) -> list[tuple[int, str]]:
  """The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker found on the
  sheet, each error by its line, and no warning. It raises when the checker could not read the sheet."""
