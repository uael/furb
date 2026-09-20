"""The extension: one life in the sandbox, and the gate of the crate, reached from python.

Every value that crosses is plain data: none, a truth, a number, a text, a list, and a map from a text to plain data.
"""

from collections.abc import Callable, Sequence
from typing import final

type Plain = None | bool | int | float | str | list[Plain] | dict[str, Plain]
"""Plain is what crosses the door: what a tuple, a shape, an exception or a callable is in that form is
`furb_monty.engine`'s to say, on both sides."""

@final
class Life:
  """One life: the engine in the sandbox, and the host it reaches."""

  def __init__(self, host: Callable[[str, Plain], Plain], ears: Sequence[str], record: Plain) -> None: ...
  @property
  def root(self) -> str:
    """The root chain of the life, which is the first act of any record."""

  @property
  def raised(self) -> Plain:
    """What boot raised, if it raised, plain, and nothing otherwise."""

  def word(self, word: str) -> Plain:
    """One word of the operator, run in the names of the engine, and what it gave, plain."""

@final
class Gate:
  """The gate of the crate, for the Kernel of this interpreter to read a sheet with."""

  def __init__(self) -> None: ...
  def checked(self, sheet: str) -> list[tuple[int, str]]:
    """What ty found on a sheet, each finding by its line, and none of the warnings."""

class Raised(Exception):
  """What the engine raised, as its name and what it was made with, which `furb_monty.engine` raises as the
  exception it is."""
