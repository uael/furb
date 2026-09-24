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

  def __init__(
    self,
    ears: object,
    names: Sequence[str],
    record: object,
    words: Sequence[str] = (),
    lives: Sequence[str] = (),
    *,
    taken: Sequence[str] | None = None,
    engine: str | None = None,
  ) -> None:
    """A life, opened on the ears of the host, the names they hear by in the order the engine hears them, and the
    record a World kept. The ears are one object with `hears(name, fact)`, `answered(name, value)`,
    `called(name, args, kwargs)`, `ear(generator)` and `callable(function)`. The life takes the builtins `taken`,
    every builtin unless they are given, runs the words of the extensions in the module of the engine, and plays the
    life words; the engine is the crate's unless it is given."""

  @property
  def root(self) -> str:
    """The root chain of the life, which is the first act of any record."""

  @property
  def system(self) -> str:
    """The system prompt of every model of the life, which is the text the life runs."""

  @property
  def raised(self) -> BaseException | None:
    """What boot raised, if it raised, as the exception it is, and nothing otherwise."""

  def verb(self, name: str, args: Sequence[object], kwargs: dict[str, object]) -> object:
    """One verb of the engine by its name, said by the operator with these words, and what it gave."""

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

def gate(sheet: str, engine: str | None = None) -> list[tuple[int, str]]:
  """The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker found on the
  sheet, read against the engine the Kernel runs, which is the engine of the crate unless it is given, each error by
  its line, and no warning. It raises when the checker could not read the sheet."""

@final
class Extension:
  """One extension as a host plays it, which the crate reads for every host."""

  @property
  def name(self) -> str:
    """The name of the extension, which is its key in a config."""
  @property
  def root(self) -> str | None:
    """The directory the extension stands in, and nothing for a builtin."""
  @property
  def word(self) -> str | None:
    """The word of its python part, which a host plays once as a rung on each chain without a source."""
  @property
  def life(self) -> str | None:
    """The word it plays in every life on each chain without a source, after the words."""
  @property
  def requires(self) -> list[str]:
    """The names of the extensions it requires, which a host plays before it."""
  @property
  def world_ts(self) -> str | None:
    """The file of its part for a World in TypeScript, if it has one."""
  @property
  def world_py(self) -> str | None:
    """The file of its part for a World in python, if it has one."""
  @property
  def tui(self) -> str | None:
    """The file of its part for the TUI, if it has one."""

def extensions(project: str, refresh: bool = False) -> list[Extension]:
  """The extensions a host plays for a project: the builtins and what the config of the user and the config of the
  project name, fetched into the cache of the user once, and again on a refresh, and ordered by what each requires.
  A config, a fetch or a manifest that fails raises Refused, with what failed."""

def places() -> tuple[str, str]:
  """The config directory and the cache directory of the user, as this process finds them."""

def builtin_extensions() -> list[Extension]:
  """The builtin extensions, files, bash and grant, in the order a host plays them."""

def word_of(source: str) -> str:
  """The word of the python part of an extension, which the module of the engine runs after the engine: the file with
  its line ends made LF and less every top-level import from `furb`, which leaves no line of its own, and nothing else
  changed. A file python cannot parse raises Refused."""

def system_prompt(engine: str, taken: list[str], words: list[str]) -> str:
  """The system prompt of a life: the engine as the host minified it, less the definitions of each builtin that
  `taken` does not name, then the words of the extensions, in their order. An engine python cannot parse raises
  Refused."""

def cut_names(taken: list[str]) -> set[str]:
  """The top-level names of the engine that the builtins `taken` does not name define."""

def pinned(record: object, taken: list[str], words: list[str]) -> tuple[list[str], list[str], bool]:
  """What a life on a record takes and runs, the builtins and the words, and whether the life pins them: what the
  record pins, or every builtin and no word when it pins nothing; on an empty record, what it is given, pinned unless
  it is every builtin and no word."""
