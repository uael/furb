"""The Kernel of this interpreter: it gates a word with ty, and it runs the word in the module of its chain.

The engine holds the laws of a chain. To judge a word before it runs, and to run it, are machinery, so they stand
here, behind the Kernel that the contract declares. The gate reads the word on the sheet of `furb.sheet`, with the
ty command line reading it: the names of the engine, bound as a chain binds them, then the ladder of the chain,
then the word, all inside one async body, so that the awaits of the word stand.

A word answers by a close and never by a return: a body of a module takes no return, so a word that holds one is no
python and the gate says so.
"""

import ast
import re
import shutil
import subprocess
import sys
from asyncio import CancelledError
from collections.abc import Generator
from inspect import iscoroutine
from pathlib import Path
from tempfile import TemporaryDirectory
from types import CoroutineType

from furb import engine, sheet
from furb.engine import Act, Refused, modules, outcomes, site, under

type Kernel = Generator[tuple | None, tuple]
"""The Kernel, as engine.pyi declares it: engine.py binds no such name, so this module says the type itself."""

SOURCE = Path(engine.__file__)
"""SOURCE is the engine, which names every name that the globals of a chain hold of it."""
DIAG = re.compile(
  r"\A(?P<path>.+?):(?P<line>\d+):(?P<column>\d+): (?P<sort>error|warning)\[(?P<rule>[^\]]+)\] (?P<why>.*)\Z"
)
"""DIAG reads one finding of ty in its concise form."""
VERSION = ".".join(map(str, sys.version_info[:2]))
"""VERSION is the python that the gate reads a word for, which is the python that runs it."""
NO_TY = "the gate did not run"
"""NO_TY is what the life ends with when ty is not there to read a word, which is no finding against the word."""


def declared(source: Path = SOURCE) -> list[str]:
  """Every name the engine defines at its top, which is what the globals of a chain hold of the engine."""
  said: list[str] = []
  for node in ast.parse(source.read_text(encoding="utf-8")).body:
    match node:
      case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef() | ast.TypeAlias():
        said.append(node.name if not isinstance(node, ast.TypeAlias) else node.name.id)
      case ast.AnnAssign(target=ast.Name(id=name)):
        said.append(name)
      case ast.Assign(targets=[ast.Name(id=name)]):
        said.append(name)
      case ast.Assign(targets=[ast.Tuple(elts=elts)]):
        said.extend(one.id for one in elts if isinstance(one, ast.Name))
  return [name for i, name in enumerate(said) if name not in said[:i]]


NAMES = declared()
"""NAMES are the names of the engine, which the sheet binds before it reads the ladder and the word."""


def checked(text: str) -> list[tuple[int, str]]:
  """What ty finds on one sheet, each finding by its line: the errors, and none of the warnings, since a warning
  refuses no word.

  A tool that did not run has said nothing about the word, which is not the same as having found nothing, so the
  life ends there rather than refuse a word that nobody read.
  """
  ty = shutil.which("ty")
  if ty is None:
    raise RuntimeError(NO_TY)
  with TemporaryDirectory(prefix="furb-gate-") as yard:
    at = Path(yard, "sheet.py")
    at.write_text(text, encoding="utf-8")
    args = [ty, "check", "--python", sys.prefix, "--python-version", VERSION, "--output-format", "concise"]
    # The gate answers while the one that asked waits, so it reads the sheet here and not on the loop.
    try:
      ran = subprocess.run([*args, "--exit-zero", str(at)], capture_output=True, check=False)  # noqa: S603
    except OSError as no:
      why = f"{NO_TY}: {no}"
      raise RuntimeError(why) from no
  if ran.returncode:
    why = f"{NO_TY}: {ran.stderr.decode(errors='replace').strip()}"
    raise RuntimeError(why)
  return [
    (int(found["line"]), f"error[{found['rule']}] {found['why']}")
    for line in ran.stdout.decode(errors="replace").split("\n")
    if (found := DIAG.match(line.strip())) is not None and found["sort"] == "error"
  ]


class Native:
  """The Kernel of the interpreter this process runs in, where the module of a chain is a dict.

  It answers a gate with what ty finds against the word, begins a run by compiling the word in the module of its
  chain, says wants for the act a run waits for, carries the run forward at each sent, says ran with nothing for a
  word that ran to its end and with the exception for one that raised, and drops the frame of a run a cancel is
  over. A frame that is mid step is never closed: the close of a word raises where that word stands, and what
  unwinds out of it is the drop. One of these serves one life, since the frames it holds and the ladders it gates
  against are that life's own, and a ladder is the accepted words of its chain, which grows as the gate accepts.
  """

  def __init__(self) -> None:
    self.frames: dict[str, CoroutineType[object, object, object]] = {}
    self.ladders: dict[str, list[str]] = {}

  def gate(self, word: str, ladder: list[str]) -> list[str]:
    """What the gate finds against a word: the sheet of the engine, read by the ty command line."""
    return sheet.gate(NAMES, ladder, word, checked)

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
      ran = eval(compile(word, name, "exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT), held)  # noqa: S307
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
        case ("gate", qid, _, chain, word):
          # The ladder is the accepted words of the chain, so an accepted word joins it here, before it runs.
          found = self.gate(word, self.ladders.setdefault(chain, []))
          if not found:
            self.ladders[chain].append(word)
          yield "done", qid, found
        case ("cancel" | "close", about, *_):
          # The frame of the word that says the close is mid step, and the CancelledError of close ends that one.
          for one in [x for x in self.frames if under(x, about) and not self.frames[x].cr_running]:
            self.frames[one].close()
            self.ended(one, CancelledError())
