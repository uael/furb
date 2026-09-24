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
from collections.abc import Generator, Sequence
from inspect import iscoroutine
from types import CoroutineType

import furb
import furb_monty
from furb import engine, sheet
from furb.engine import Act, Refused, modules, outcomes, site, under

type Kernel = Generator[tuple | None, tuple]
"""The Kernel, an Ear of engine.pyi: engine.py binds no such name, so this module says the type itself."""

ENGINE = vars(furb.python)
"""ENGINE is the module of the engine this Kernel runs a word with, whose names the module of every chain holds,
so that the sheet binds each of them before the word."""


def checked(text: str, source: str | None = None) -> list[tuple[int, str]]:
  """What the gate of the crate finds on one sheet, read against the source of the engine the Kernel runs, which is
  the engine alone unless it is given, each finding by its line: the errors, and none of the warnings, since a
  warning refuses no word. A gate that could not read the sheet has said nothing about the word, which is not the
  same as having found nothing, so it raises and the life ends there rather than refuse a word that nobody read."""
  return furb_monty.gate(text, source)


def extended(words: Sequence[str]) -> str:
  """The module of the engine with the words of the extensions run in it after the engine, as a life of monty runs
  them, so the module of every chain binds their names from its birth; and the source of that engine, which the gate
  reads a word on."""
  for word in words:
    exec(compile(word, "<extension>", "exec"), ENGINE)  # noqa: S102
  return furb_monty.engine_source(list(words))


class Native:
  """The Kernel of the interpreter this process runs in, where the module of a chain is a dict.

  It begins a run by compiling the word in the module of its chain, says wants for the act a run waits for, carries
  the run forward at each sent, says ran with nothing for a word that ran to its end and with the exception for one
  that raised, and drops the frame of a run a cancel is over. A frame that is mid step is never closed: the close
  of a word raises where that word stands, and what unwinds out of it is the drop. One of these serves one life,
  since the frames it holds are that life's own. The gate is no part of it: an ear of its own, so a word may ask it
  while the Kernel runs that word.
  """

  def __init__(self) -> None:
    self.frames: dict[str, CoroutineType[object, object, object]] = {}

  def ended(self, name: str, got: BaseException | None) -> None:
    """The run is over, and what it came to goes to the chain that had it run."""
    self.frames.pop(name, None)
    engine.send("ran", name, got, by=name)

  def carry(self, name: str, sent: object) -> None:
    """The run stepped with what it waited for, and stepped again while what it waits for is over already."""
    token = site.set(name)
    try:
      while True:
        try:
          frame = self.frames[name]
          got = frame.throw(sent) if isinstance(sent, BaseException) else frame.send(sent)
          while not isinstance(got, Act):
            got = frame.throw(Refused(f"a rung awaits an act, and {got!r} is none"))
        except StopIteration:
          return self.ended(name, None)
        except BaseException as raised:
          return self.ended(name, raised)
        if got not in outcomes:
          engine.send("wants", name, got, by=name)
          return None
        sent = outcomes[got]
    finally:
      site.reset(token)

  def begin(self, name: str, word: str, held: dict[str, object]) -> None:
    """A run begun: the word is compiled in the module of its chain, and a word that awaits nothing ends here."""
    token = site.set(name)
    try:
      # The compile stands inside, so a word the interpreter will not take is what the run came to and no more.
      ran = eval(compile(word, name, "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT), held)  # noqa: S307
    except BaseException as raised:
      return self.ended(name, raised)
    finally:
      site.reset(token)
    if not iscoroutine(ran):
      return self.ended(name, None)
    self.frames[name] = ran
    return self.carry(name, None)

  def kernel(self) -> Kernel:
    """The Kernel as one generator for one life, which speaks from the run it steps."""
    while True:
      match (yield):
        case ("run", rung, _, chain, word, _):
          # This Kernel runs every word, retold or not, so it reads no donor off the run.
          self.begin(rung, word, modules[chain])
        case ("sent", rung, _, value) if rung in self.frames:
          self.carry(rung, value)
        case ("cancel" | "close", about, *_):
          # The frame of the word that says the close is mid step, and the CancelledError of close ends that one.
          for one in [x for x in self.frames if under(x, about) and not self.frames[x].cr_running]:
            self.frames[one].close()
            self.ended(one, CancelledError())


def gate(word: str, program: list[str], source: str | None = None) -> list[str]:
  """What the gate finds against a word: the sheet of the engine, read by the gate of the crate against the source
  of the engine."""
  return sheet.gate(ENGINE, program, word, lambda text: checked(text, source))


def gating(source: str | None = None) -> Kernel:
  """The gate as the ear of a life, which reads every word on the sheet of the engine with the gate of the crate,
  against the source of the engine, which holds the words of the extensions after the engine when it is given."""
  return sheet.gating(ENGINE, lambda text: checked(text, source))
