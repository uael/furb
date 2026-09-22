"""The extension: one life in the sandbox, and the gate of the crate, reached from python.

Every value that crosses is what monty carries, made python: none, a truth, a number, a text, a list, a tuple, a
map, an instance of a class of the engine as the instance it is, an exception as the one object it is for the
life, a name of the engine as the name this interpreter holds, and a callable the engine made as one that calls
it back. A generator of this interpreter crosses as an ear the host hears, and a callable as one the host calls.
"""

from collections.abc import Callable, Sequence
from typing import final

@final
class Life:
  """One life: the engine in the sandbox, and the host it reaches."""

  def __init__(self, ears: object, names: Sequence[str], record: object) -> None:
    """A life, opened on the ears of the host, the names they hear by in the order the engine hears them, and the
    record a World kept. The ears are one object with `hears(name, fact)`, `answered(name, value)`,
    `called(name, args, kwargs)`, `ear(generator)` and `callable(function)`."""

  @staticmethod
  def restored(ears: object, names: Sequence[str], dump: bytes) -> Life:
    """A life restored from a dump of one that stood still, on the ears of the host under the names it was
    dumped with."""

  def dump(self) -> bytes:
    """The life as bytes, where it stands still, for a later life to go on from."""

  @property
  def root(self) -> str:
    """The root chain of the life, which is the first act of any record."""

  @property
  def raised(self) -> BaseException | None:
    """What boot raised, if it raised, as the exception it is, and nothing otherwise."""

  def verb(self, name: str, args: Sequence[object], kwargs: dict[str, object]) -> object:
    """One verb of the engine by its name, said by the operator with these words, and what it gave."""

  def word(self, word: str, inputs: dict[str, object]) -> object:
    """One word of the operator, run in the names of the engine with these values bound, and what it gave."""

  def made(self, n: int, args: Sequence[object], kwargs: dict[str, object]) -> object:
    """One callable the engine made, called back by its handle with these words, and what it gave."""

  def forget(self, n: int) -> None:
    """A callable the engine made, forgotten: this interpreter holds its handle no more, so the sandbox drops it."""

  def held(self, name: str, keys: Sequence[str], ask: str) -> object:
    """One reading of a map of the life where it stands, under these keys: `in`, `at`, `keys` or `len`."""

  def site(self, value: str | None) -> str:
    """Who speaks in the life, and who speaks from now on when a value is given."""

  def watch(self, act: str, then: Callable[[object], object]) -> None:
    """What to call when an act is done, with what it came to: at once for one done already, and once otherwise."""

def gate(sheet: str) -> list[tuple[int, str]]:
  """The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker found on the
  sheet, each error by its line, and no warning. It raises when the checker could not read the sheet."""
