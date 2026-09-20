"""furb for python: one life of the engine, in the sandbox, with a World you write in python.

The crate runs the engine in monty and gives its surface to a host. This is that surface for a host of python:
`Life` is one life, a host writes a World and hands it over, and `Voice` is how the host says into the life the
work that finishes later.

Nothing of the engine crosses. Every value that crosses is plain: none, a truth, a number, a text, a list, a
tuple and a map are the python ones, `Shape` is a shape of the engine such as a text or an exit, `Fault` is an
exception by its name and what it was made with, and `SHOW` is the mark of a show, which carries nothing.
"""

from collections.abc import Mapping, Sequence
from typing import Protocol, final

type Value = None | bool | int | float | str | list[Value] | tuple[Value, ...] | dict[str, Value] | Shape | Fault | Show
"""Value is everything that crosses between a life and its host."""

WORLD: str
"""The name the World hears under, which is the name the engine takes it under."""

@final
class Show:
  """The mark of a show, which carries nothing a host can read."""

SHOW: Show
"""The one mark of a show."""

@final
class Shape:
  """A shape of the engine, by its name and its fields, which a text and an exit are.

  The fields keep the order the shape declares them in, and each one is read by its name: `text.path` and
  `text.content` are the fields of a text, and `shape.fields` is all of them at once.
  """

  def __init__(self, name: str, fields: Mapping[str, Value]) -> None: ...
  @property
  def name(self) -> str:
    """The name of the shape, as the engine holds it."""

  @property
  def fields(self) -> dict[str, Value]:
    """The fields of the shape, by name, in the order the shape declares them."""

  def __getattr__(self, name: str) -> Value:
    """One field of the shape, by its name, which is how a host reads a text."""

@final
class Fault:
  """An exception of the engine, by its name and what it was made with.

  It is a value and not a raise: a fault the engine hands back is a thing a host reads, and only a fault of the
  call a host made is raised in the host.
  """

  def __init__(self, name: str, *args: Value) -> None: ...
  @property
  def name(self) -> str:
    """The name of the exception."""

  @property
  def args(self) -> tuple[Value, ...]:
    """What the exception was made with."""

  def refused(self) -> bool:
    """Whether the engine refused the call, which is the one fault a word of a model makes on purpose."""

@final
class Fact:
  """One fact of the engine: its kind, the act it is about, who said it, and its words."""

  def __init__(self, kind: str, about: str, by: str = "", *words: Value) -> None: ...
  @property
  def kind(self) -> str:
    """The kind of the fact, which is its first slot."""

  @property
  def about(self) -> str:
    """The act the fact is about."""

  @property
  def by(self) -> str:
    """Who said the fact: a rung, the operator, the World or the Kernel."""

  @property
  def words(self) -> tuple[Value, ...]:
    """The words of the fact, which are everything after who said it."""

  @property
  def question(self) -> bool:
    """Whether the fact is a question, which its name says: a question is about itself, under its kind."""

  @property
  def on(self) -> str | None:
    """The chain a question is on, which is its first word, and nothing for a fact that is no question."""

@final
class Ask:
  """A question of the engine, which a World answers a fact with when it must know something first.

  The boundary puts the question and hands the answer back through `World.answered`, so a World never calls into
  a life that stands waiting for it.
  """

  def __init__(self, kind: str, on: str, *words: Value) -> None: ...
  @property
  def kind(self) -> str:
    """The kind of the question, such as `cwd` or `merged`."""

  @property
  def on(self) -> str:
    """The chain the question is on."""

  @property
  def words(self) -> list[Value]:
    """The words of the question."""

type Reply = None | Fact | Sequence[Fact] | Ask
"""Reply is what a World says of a fact: nothing, one fact, the facts to say in that order, or a question."""

class World(Protocol):
  """The interface to the disk, the machine, the actors and the record.

  The World hears every fact. It answers what a chain stands on, a reading of the clock, a number it draws, and a
  read or a write of a path nobody of the engine serves; it starts what it is started to do, which is a command,
  a wait and a prompt of the operator; it answers an ask with the turn of a model; and it keeps what the journal
  says to keep.

  A World that does work which finishes later holds a `Voice` and says the facts of that work into it.
  """

  def hears(self, fact: Fact) -> Reply:
    """One fact, heard. What the World would say of it, or the question it must ask first."""

  def answered(self, got: Value) -> Reply:
    """The answer to the question the World last asked, and what it says now that it holds it."""

class Gate(Protocol):
  """What reads a word before it runs.

  The Kernel runs the word of a rung where the engine runs, so the one thing it cannot answer for itself is
  whether the word may run at all: what a word is read against is the host's to decide.
  """

  def gate(self, word: str, ladder: Sequence[str], shape: str) -> Sequence[str]:
    """What the gate finds against a word. Nothing at all means the word may run."""

class Refused(Exception):
  """What a life could not do: what the engine raised, or what the crate could not read of what it gave."""

@final
class Voice:
  """How a host says into a life when nothing asked it to.

  A Voice may be carried anywhere the host does its work, including another thread, since what it says waits in
  order until the life is ready to hear it. It is heard by one life: the life it is given to.
  """

  def __init__(self) -> None: ...
  def fact(self, one: Fact) -> None:
    """One fact, said into the life."""

  def close(self, act: str, value: Value) -> None:
    """One act, closed with a value, which answers the act that asked for it."""

  def pause(self, chain: str) -> None:
    """One chain, paused, which stops it until the operator wakes it."""

  def waiting(self) -> bool:
    """Whether anything said into this Voice is still waiting to be heard."""

@final
class Life:
  """One life: the engine in the sandbox, and the outside it reaches.

  A host opens one with a World of its own and drives it by calling words on it, the way an operator calls a
  verb. What the host started and has not finished it says through its `Voice`, and the life hears all of it in
  the order it was said.
  """

  def __init__(
    self, world: World, gate: Gate | None = None, record: str | None = None, voice: Voice | None = None
  ) -> None: ...
  @property
  def root(self) -> str:
    """The root chain of the life, which is the first act of any record."""

  @property
  def world(self) -> World:
    """The World of this life, which is the object the host handed over."""

  def word(self, word: str) -> Value:
    """One word of the operator, run on a chain, and what it gave."""

  def heard(self) -> int:
    """Everything the host has said into its Voice, done in the life, in the order it was said."""

  def waits(self, seconds: float) -> bool:
    """Wait until the host says something, or until this long has passed, and say whether anything waits."""

  def came(self, act: str) -> Value:
    """What an act came to, and nothing at all while it waits."""

def line(entry: Value) -> str:
  """The line a World writes for one entry it was told to keep."""
