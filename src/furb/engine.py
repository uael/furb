from collections.abc import Callable
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template

WINDOW = 200000
OPERATOR = "operator"
WORLD = "world"
TIMEOUT = 600.0
WHY = "the engine is derived from the contract and the suite, and is not written yet"
site = ContextVar("site", default=OPERATOR)
modules, acts, asked, outcomes = {}, {}, {}, {}
type Show = Callable[[list[str]], list[int]]
type Filter = Callable[[list[tuple]], list[tuple]]


def send(kind, about, *words, by=""):
  raise NotImplementedError(WHY)


def ask(kind, on, *words):
  raise NotImplementedError(WHY)


def act(kind, on, life, *words):
  raise NotImplementedError(WHY)


def drive(g, name):
  raise NotImplementedError(WHY)


def span(lo: int, hi: int) -> Show:
  def picks(lines):
    raise NotImplementedError(WHY)

  return picks


def grep(pattern: str) -> Show:
  def picks(lines):
    raise NotImplementedError(WHY)

  return picks


def differs(old: list[str]) -> Show:
  def picks(lines):
    raise NotImplementedError(WHY)

  return picks


HEAD, TAIL, HIDDEN = span(1, 2000), span(-250, -1), span(0, 0)


def take(*ids: str, inside: bool = True) -> Filter:
  def picks(lines):
    raise NotImplementedError(WHY)

  return picks


def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  raise NotImplementedError(WHY)


def write(text: Text, on: str = "") -> Text:
  raise NotImplementedError(WHY)


def peek(at: str, on: str = "") -> object:
  raise NotImplementedError(WHY)


def turns(on: str = "") -> list[tuple]:
  raise NotImplementedError(WHY)


def clock(on: str = "") -> float:
  raise NotImplementedError(WHY)


def chance(on: str = "") -> float:
  raise NotImplementedError(WHY)


def gate(word: str, returns: str = "", on: str = "") -> list[str]:
  raise NotImplementedError(WHY)


def cd(path: str, on: str = "") -> str:
  raise NotImplementedError(WHY)


def cwd(on: str = "") -> str:
  raise NotImplementedError(WHY)


def get(about: str) -> tuple:
  raise NotImplementedError(WHY)


def pause(id: str) -> None:
  raise NotImplementedError(WHY)


def wake(id: str) -> None:
  raise NotImplementedError(WHY)


def cancel(id: str) -> None:
  raise NotImplementedError(WHY)


def close(value: object, id: str = "") -> None:
  raise NotImplementedError(WHY)


def debug(template: Template) -> None:
  raise NotImplementedError(WHY)


def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  raise NotImplementedError(WHY)


def rung(word: str = "", retells: str = "", actor: str = "", returns: str = "", on: str = "") -> Act:
  raise NotImplementedError(WHY)


def prompt[T](shape: type[T] | None, message: str = "", to: str = "", on: str = "") -> Act[T]:
  raise NotImplementedError(WHY)


def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act:
  raise NotImplementedError(WHY)


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  raise NotImplementedError(WHY)


def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  raise NotImplementedError(WHY)


@dataclass
class Text:
  path: str
  content: str = ""
  before: Text | None = None

  @property
  def lines(self) -> list[str]:
    raise NotImplementedError(WHY)

  def grow(self, text: str) -> Text:
    raise NotImplementedError(WHY)

  def edit(self, lo: int, hi: int, lines: list[str]) -> Text:
    raise NotImplementedError(WHY)

  def replace(self, old: str, new: str, once: bool = False) -> Text:
    raise NotImplementedError(WHY)

  def undo(self, n: int = 1) -> Text:
    raise NotImplementedError(WHY)

  def append(self, text: str) -> Text:
    raise NotImplementedError(WHY)

  def insert(self, line: int, text: str) -> Text:
    raise NotImplementedError(WHY)

  def delete(self, lo: int, hi: int) -> Text:
    raise NotImplementedError(WHY)

  def find(self, pattern: str) -> list[int]:
    raise NotImplementedError(WHY)


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Act[T = object](str):
  def __await__(self):
    raise NotImplementedError(WHY)


class Refused(Exception): ...


class Drift(Exception): ...


def lineage(name):
  raise NotImplementedError(WHY)


def under(name, of):
  raise NotImplementedError(WHY)


def acting():
  raise NotImplementedError(WHY)


def question(a):
  raise NotImplementedError(WHY)


def scope(name):
  raise NotImplementedError(WHY)


def tell(name, *attrs, body=None):
  raise NotImplementedError(WHY)


def told(name, id, *attrs, body=None):
  raise NotImplementedError(WHY)


def control(kind, name, id, *words):
  raise NotImplementedError(WHY)


def showing(got, show):
  raise NotImplementedError(WHY)


def shown(pair, seen):
  raise NotImplementedError(WHY)


def turns_of(heard):
  raise NotImplementedError(WHY)


def offered(standing, to):
  raise NotImplementedError(WHY)


def covers(a, id):
  raise NotImplementedError(WHY)


def ended(a, id):
  raise NotImplementedError(WHY)


def idle(id):
  raise NotImplementedError(WHY)


def lives(g, a):
  raise NotImplementedError(WHY)


def pausing(life):
  raise NotImplementedError(WHY)


def ending(life):
  raise NotImplementedError(WHY)


def started(life, to=OPERATOR):
  raise NotImplementedError(WHY)


def boot(record=(), **outside):
  raise NotImplementedError(WHY)
