"""The Kernel of this interpreter, which runs a word in the module of its chain, and the gate, which reads it with
the gate of the crate.

The engine holds the laws of a chain. To judge a word before it runs, and to run it, are machinery, so they stand
here, in the Kernel and the gate, two ears that boot is given. The gate reads the word on the sheet of
`furb.sheet`, with the type checker of monty reading it, the one the crate carries and the engine of monty gates
with too, so a word is judged once and the same: every name of the engine, then the program of the chain, then the
word, all inside one async body, so that the awaits of the word stand.

A word answers by a close and never by a return: a body of a module takes no return, so a word that holds one is no
python and the gate says so.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from asyncio import CancelledError
from collections.abc import Generator
from inspect import iscoroutine
from types import CoroutineType

import furb
import furb_monty
from furb import engine, sheet
from furb.engine import Act, Refused, site, under

type Kernel = Generator[tuple | None, tuple]
"""The Kernel, an Ear of engine.pyi: engine.py binds no such name, so this module says the type itself."""

ENGINE = vars(furb.python)
"""ENGINE is the module of the engine this Kernel runs a word with, whose names the module of every chain holds,
so that the sheet binds each of them before the word."""


def checked(text: str) -> list[tuple[int, str]]:
  """What the gate of the crate finds on one sheet, each finding by its line: the errors, and none of the
  warnings, since a warning refuses no word. A gate that could not read the sheet has said nothing about the word,
  which is not the same as having found nothing, so it raises and the life ends there rather than refuse a word
  that nobody read."""
  return furb_monty.gate(text)


class Native:
  """The Kernel of the interpreter this process runs in, where the module of a chain is a dict.

  It takes a run as that run, and begins its word when it hears that it took it, compiled in the module of its
  chain and run as its rung. When the word waits for an act that is not done, it makes a wants as the run, and
  carries the word forward at the done of that wants. It says the run done, as the run, with nothing for a word that
  ran to its end and with the exception for one that raised, and it drops the frame of a word a cancel is over. A
  frame that is mid step is never closed: the close of a word raises where that word stands, and what unwinds out
  of it is the drop. One of these serves one life, since the frames it holds are that life's own. The gate is no
  part of it: an ear of its own, so a word may ask it while the Kernel runs that word.
  """

  def __init__(self) -> None:
    self.frames: dict[str, CoroutineType[object, object, object]] = {}
    self.taken: set[str] = set()
    self.waits: dict[str, str] = {}

  def ended(self, run: str, got: BaseException | None) -> None:
    """The word is over, and the run is done with what the word gave, as that run."""
    self.frames.pop(run, None)
    with site.set(run):
      engine.say("done", run, got)

  def carry(self, run: str, given: object) -> None:
    """The word stepped as its rung with what it waited for, and stepped again while what it waits for is done."""
    with site.set(str(engine.get(run)[4])):
      while True:
        try:
          frame = self.frames[run]
          got = frame.throw(given) if isinstance(given, BaseException) else frame.send(given)
          while not isinstance(got, Act):
            got = frame.throw(Refused(f"a rung awaits an act, and {got!r} is none"))
        except StopIteration:
          return self.ended(run, None)
        except BaseException as raised:
          return self.ended(run, raised)
        if engine.peek(got, ...) is ...:
          with site.set(run):
            self.waits[engine.act("wants", "", None, got)] = run
          return None
        given = engine.peek(got)

  def begin(self, run: str) -> None:
    """A word begun as its rung: it is compiled in the module of its chain, a word that awaits nothing ends here,
    and a rung that is done already begins no word."""
    self.taken.discard(run)
    _, _, _, chain, rung, word, _ = (str(x) for x in engine.get(run))
    if engine.peek(rung, ...) is not ...:
      return None
    try:
      # The compile stands inside, so a word the interpreter will not take is what the run came to and no more.
      with site.set(rung):
        ran = eval(compile(word, rung, "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT), engine.module(chain))  # noqa: S307
    except BaseException as raised:
      return self.ended(run, raised)
    if not iscoroutine(ran):
      return self.ended(run, None)
    self.frames[run] = ran
    return self.carry(run, None)

  def kernel(self) -> Kernel:
    """The Kernel as one generator for one life: it takes each run, begins its word when it hears that it took it,
    and carries the word at the done of each wants it made."""
    while True:
      match (yield):
        case ("run", run, *_):
          # This Kernel runs every word, retold or not, so it reads no donor off the run.
          with site.set(run):
            engine.say("started", run)
          self.taken.add(run)
        case ("started", run, *_) if run in self.taken:
          self.begin(run)
        case ("done", wants, _, value) if wants in self.waits:
          self.carry(self.waits.pop(wants), value)
        case ("cancel" | "close", about, *_):
          # The frame of the word that says the close is mid step, and the CancelledError of close ends that one.
          for one in [x for x in self.frames if under(str(engine.get(x)[4]), about)]:
            if not self.frames[one].cr_running:
              self.frames[one].close()
              self.ended(one, CancelledError())


def gate(word: str, program: list[str]) -> list[str]:
  """What the gate finds against a word: the sheet of the engine, read by the gate of the crate."""
  return sheet.gate(ENGINE, program, word, checked)


def gating() -> Kernel:
  """The gate as the ear of a life, which reads every word on the sheet of the engine with the gate of the crate."""
  return sheet.gating(ENGINE, checked)
