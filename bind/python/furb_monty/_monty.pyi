"""The extension: one engine in the sandbox, the ears of the crate, and the gate of the crate, reached from python.

Every value that crosses is what monty carries, made python: none, a truth, a number, a text, a list, a tuple, a
map, an instance of a class of the engine as the instance it is, an exception as the one object it is for the
life, a name of the engine as the name this interpreter holds, and a callable the engine made as one that calls
it back. A generator of this interpreter crosses as an ear, an ear of the crate as itself, and a function as one
the sandbox calls back.
"""

from collections.abc import Callable, Generator, Iterable
from typing import final

@final
class NativeEar(Generator[tuple | None, tuple]):
  """An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear. The engine of
  monty hears it as itself, and the engine of this interpreter steps it as a generator of its own."""

  def dispose(self) -> None:
    """The ear is let go before any engine hears it, or after the life it heard in, so what it holds goes: a command
    ends, a wait ends, and a store lets its record go."""

  def send(self, value: tuple | None, /) -> tuple | None: ...
  def throw(self, *args: object) -> tuple | None: ...
  def pump(self) -> None:
    """What the work of the ear said since, said into the engine of this interpreter under the name of the ear."""

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

def official() -> list[tuple[str, str, str]]:
  """The official extensions, in the order a life runs them, each as its name, its word and its life word."""

def enabled(root: list[tuple]) -> list[tuple[str, str, str]]:
  """The extensions that a life runs, in the order it enabled them, as the facts of its root say them, each as its
  name, its word and its life word."""

def extensions(given: list[tuple[str, str, str]]) -> NativeEar:
  """The ear of the extensions, given each extension that the life runs as its name, its word and its life word: it
  enables each at the tip of the life, unless the record enables it, and plays each as a rung on each chain."""

def memory(config: str) -> NativeEar:
  """The ear of the memory extension, which finds the memory of a path in its folders and in the config directory."""

def skills(config: str) -> NativeEar:
  """The ear of the skills extension, which finds skills in the folders of a chain and in the config directory."""

def opened(
  directory: str,
  record: str | None = None,
  *,
  keeps: bool = True,
  inspecting: bool = False,
  extensions: bool = True,
  config: str | None = None,
  actor: str | None = None,
  roster: list[str] | None = None,
  answer: Callable[[dict, Callable[..., None]], tuple] | None = None,
  claude: str | None = None,
  stall: float | None = None,
  images: str | None = None,
  stream: Callable[[str, str, str, str], None] | None = None,
) -> tuple[list[list[list[object]]], list[tuple[str, NativeEar]]]:
  """The record a life opens on, and the ears of the crate that it hears after the ears of the host, as every host of
  the crate opens a life: the provider, the extensions, which enable at the tip those that the configs of the user
  and of the directory turn on unless `extensions` is false, each official extension, the files, the commands, time,
  and the store of the record when the life keeps. A life that inspects keeps nothing, enables nothing new, asks no
  model, and its files, commands and time do no work.

  The provider offers the model of `actor` and the models of `roster`, each named `provider:id` or by an id that one
  model alone holds, or the first model the catalog offers when neither is said. `answer`, when given, answers each
  request in place of the models: it is called on a thread of its own with the request, as JSON reads it, and a
  function `write(text="", thinking="")`, and gives the turn. `claude` is the path of the claude command line, whose
  turn is refused when it makes no progress for `stall` seconds. `images` is the directory of the images that a turn
  names. `stream` is told what a model writes as it writes it, on a thread of the models: the rung it writes for, the
  chain of that rung, and what it added to its text and to its thought."""

def answered(shape: str, line: str) -> object:
  """A line of the operator as a value of the shape a prompt wants, by the rules every console of the crate reads a
  line by. It raises Refused for a line that is no value of the shape, and for a shape the operator answers not."""

type Info = tuple[str, list[str], int, bool, list[float] | None]
"""A model of the catalog: its name, its efforts, its window, whether it takes an image, and its price in dollars for a
million tokens read, written, read from the cache and written to it."""

def models(claude: str | None = None) -> list[Info]:
  """The models the catalog of this machine offers, with the claude command line at a path when it is given, each
  named as the catalog names it."""

def model(name: str) -> Info | None:
  """The model the catalog knows by a name, as `provider:id` or as an id that one model alone holds, whether it offers
  that model or not; nothing when it knows none."""

def levels() -> list[str]:
  """The levels of effort, from least to most, which an actor names after its model."""

def files() -> NativeEar:
  """The ear of the files, which reads and writes a path."""

def bash() -> NativeEar:
  """The ear of commands, which runs each in a shell of this machine."""

def time() -> NativeEar:
  """The ear of time, which reads the clock, draws a chance, and ends a wait."""

def store(path: str) -> tuple[list[list[list[object]]], NativeEar]:
  """The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says
  to keep."""

def kept(path: str) -> list[list[list[object]]]:
  """What the store kept at a path, read with no lease and changed in nothing."""

def gate(sheet: str) -> list[tuple[int, str]]:
  """The gate of the crate, for the Kernel of this interpreter to read a sheet with: what the checker found on the
  sheet, each error by its line, and no warning. It raises when the checker could not read the sheet."""

SHAPES: list[str]
"""Every shape the operator answers, by its name."""
SYSTEM: str
"""The system prompt of every model: the engine, minified in layout alone, and nothing else."""
