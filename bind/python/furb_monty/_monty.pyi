"""The extension: one engine in the sandbox, the ears of the crate, and the gate of the crate, reached from python.

Every value that crosses is what monty carries, made python: none, a truth, a number, a text, a list, a tuple, a
map, an instance of a class of the engine as the instance it is, an exception as the one object it is for the
life, a name of the engine as the name this interpreter holds, and a callable the engine made as one that calls
it back. A generator of this interpreter, or anything stepped as one, crosses as an ear, an ear of the crate as
itself, and a function as one the sandbox calls back.
"""

from collections.abc import Callable, Iterable
from string.templatelib import Template
from typing import final

@final
class NativeEar:
  """An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear."""

  def dispose(self) -> None:
    """The ear is let go before any engine hears it, so what it holds goes: a store lets its record go."""

@final
class Engine:
  """One engine, held on the thread of python. Its verbs are the verbs of the contract, one method each, and none
  given to a word that has a default leaves the default of the engine. A verb that makes an act gives its name."""

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
    """One name of the engine that is no verb, said by its name with these words, and what it gave."""

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

  def say(self, kind: str, about: str, *words: object) -> tuple:
    """The way to say a fact from what is no ear: a word through its verbs, the operator, and the work an ear began,
    which speaks from its own loop; the fact is said to the living, whole as the bus made it, and given back."""
  def act(self, kind: str, on: str, ear: Callable[[str], object] | None, *words: object) -> str:
    """The way to make a question: it takes a name when it is made, it is put to the ears, its ear, when it has one, is
    brought to life under that name and given the name, and the name is given back, which is the act to whoever holds
    it."""
  def drive(self, g: object, name: str) -> None:
    """The other way to speak: a generator is brought to life under a name, and from then it hears every fact that is
    said and says its own."""
  def transcript(self, on: str | None = None) -> list[tuple]:
    """transcript gives the facts on a chain, each of which the life adds when it is said, so a chain reads at once what
    it said itself."""
  def ask(self, kind: str, on: str, *words: object) -> object:
    """The way to put a question that is answered now: it makes the act, and gives back what the act came to."""
  def span(self, lo: int, hi: int) -> object:
    """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that
    span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty."""
  def grep(self, pattern: str) -> object:
    """grep(pattern) is the show of the lines that the pattern matches, each with its number."""
  def differs(self, old: list[str]) -> object:
    """differs(lines) is the show of the lines that differ from the lines it holds, which is what a write shows of what
    came back."""
  def take(self, *ids: str, inside: bool | None = None) -> object:
    """take keeps the acts it names and everything they made."""
  def read(self, path: str, show: object | None = None, on: str | None = None) -> object:
    """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has
    not seen."""
  def write(self, text: object, on: str | None = None) -> object:
    """A write: whoever serves the path of the text takes its content."""
  def peek(self, at: str, waiting: object | None = None) -> object:
    """What the act it is at came to, as the record stands where the call is made, which it gives and never raises."""
  def turns(self, on: str | None = None) -> list[tuple]:
    """The turns of a chain, folded from what it has heard."""
  def module(self, on: str | None = None) -> dict[str, object]:
    """module gives the globals of a chain: the dict that the last module of its transcript carries, in which every rung
    of the chain runs."""
  def program(self, on: str | None = None) -> dict[str, str]:
    """program gives the program of a chain: the word of every rung that runs on it since its last module, as python,
    each under the name of that rung, in order, as the runs of its transcript say."""
  def standing(self) -> object:
    """standing gives what the chains stand on: the answer of the last stand that the transcript of the root holds, and
    an empty standing before the first."""
  def stand(self, on: str | None = None) -> object:
    """stand asks the World what the chains stand on and gives the answer, which boot does on the root at the tip of
    every life."""
  def clock(self, on: str | None = None) -> float:
    """clock gives one reading of the wall clock of the World."""
  def chance(self, on: str | None = None) -> float:
    """chance gives a number that is at least zero and under one."""
  def gate(self, word: str, on: str | None = None) -> list[str]:
    """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when
    the word may run."""
  def cd(self, path: str, on: str | None = None) -> str:
    """A cd: the paths of its chain resolve against its path from then on, and it does nothing else."""
  def cwd(self, on: str | None = None) -> str:
    """The working directory of a chain is the closest cd back in its transcript."""
  def get(self, about: str) -> tuple | None:
    """The act again, from its name: whoever holds the name of an act is given the act the life holds under it, whole as
    it stands."""
  def pause(self, id: str) -> None:  # noqa: A002
    """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake."""
  def wake(self, id: str) -> None:  # noqa: A002
    """A wake: it ends the pause over the same act, and what waited is heard."""
  def cancel(self, id: str) -> None:  # noqa: A002
    """A cancel of that prompt reaches the acts that its rungs made on the chain with a source."""
  def close(self, value: object, id: str | None = None) -> None:  # noqa: A002
    """An act ended from outside, by its name, with a value: it is done with it, and it ends what it made, since a close
    is a cancel that carries what the act it names is done with."""
  def debug(self, template: Template) -> None:
    """What a word tells of itself as it runs: each interpolation of a template, with its expression and its value."""
  def wait(self, seconds: float | None = None, on: str | None = None) -> str:
    """A wait: the World takes it and says it is done when its seconds have passed, and it is over then."""
  def rung(
    self, word: str | None = None, retells: str | None = None, actor: str | None = None, on: str | None = None
  ) -> str:
    """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word,
    a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells
    nothing, since that turn stands as the turn it is."""
  def prompt(self, shape: object, message: str | None = None, to: str | None = None, on: str | None = None) -> str:
    """A prompt: it makes the rung of one turn of its model, makes another while the rung it made gives no value, and is
    done with the value, so a rung whose word is refused and a rung whose word raises are asked again alike."""
  def chain(
    self,
    label: str | None = None,
    source: str | None = None,
    filter: object | None = None,  # noqa: A002
    on: str | None = None,
  ) -> str:
    """chain says what a chain does: how it is opened, what it tells, and what it answers for."""
  def grant(self, usd: float | None = None, share: float | None = None, on: str | None = None) -> str:
    """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger
    of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last
    one filled, and it tells that ledger at each answer of a model, so no turn a reply has sent grows a line after
    it."""
  def bash(
    self,
    command: str,
    fed: bool | None = None,
    timeout: float | None = None,
    show: object | None = None,
    show_err: object | None = None,
    on: str | None = None,
  ) -> str:
    """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to."""

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
