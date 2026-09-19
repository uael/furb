"""The Kernel of this interpreter: it gates a word with ty, and it runs the word in the module of its chain.

The engine holds the laws of a chain. To judge a word before it runs, and to run it, are machinery, so they stand
here, behind the Kernel that the contract declares. The gate reads the word on a sheet: the names of the engine,
bound as a chain binds them, then the ladder of the chain, then the word, all inside one async body, so that the
awaits of the word stand and ty proves the shape of the value the word closes with.

A word answers by a close and never by a return: a body of a module takes no return, so a word that holds one is no
python and the gate says so. The sheet binds close under the shape the word must give, so every close that names no
act is read against it, and one that names another act is read against nothing, since its value answers no prompt
of this word.
"""

import ast
import re
import shutil
import subprocess
import sys
import traceback
from asyncio import CancelledError
from collections.abc import Generator
from inspect import iscoroutine
from pathlib import Path
from tempfile import TemporaryDirectory
from types import CodeType, CoroutineType

from furb import engine
from furb.engine import Act, Refused, modules, outcomes, site, under

type Kernel = Generator[tuple | None, tuple]
"""The Kernel, as engine.pyi declares it: engine.py binds no such name, so this module says the type itself."""

SOURCE = Path(engine.__file__)
"""SOURCE is the engine, which names every name that the globals of a chain hold of it."""
ENGINE = "__engine"
"""ENGINE is the one name of the sheet: the module it binds the names of the engine from."""
OWN = re.compile(rf"\b{ENGINE}\b")
"""OWN finds the name of the sheet in a word, which no word may say."""
CLOSE = "close"
"""CLOSE is the verb a word answers by, which the sheet binds under the shape the word must give."""
ANY = "object"
"""ANY is the shape of a word that is read against no shape, which any value answers."""
DIAG = re.compile(
  r"\A(?P<path>.+?):(?P<line>\d+):(?P<column>\d+): (?P<sort>error|warning)\[(?P<rule>[^\]]+)\] (?P<why>.*)\Z"
)
"""DIAG reads one finding of ty in its concise form."""
VERSION = ".".join(map(str, sys.version_info[:2]))
"""VERSION is the python that the gate reads a word for, which is the python that runs it."""


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


BOUND = ("actor", "raised")
"""BOUND are the two names a chain binds of its own: the actor it stands on, and what the last rung raised."""
NAMES = [*declared(), *BOUND]
"""NAMES are the names the globals of a chain hold, which the sheet binds before it reads the ladder and the word."""
HEAD = f"import typing\nimport furb.engine as {ENGINE}\nasync def __body():\n" + "".join(
  f"  {x} = {ENGINE}.{x}\n" for x in NAMES if x != CLOSE
)
"""HEAD binds each name a chain holds on a line of its own, since an import declares a name that no word may rebind."""
CLOSING = f"""  @typing.overload
  def {CLOSE}(value: %s, id: typing.Literal[""] = "") -> None: ...
  @typing.overload
  def {CLOSE}(value: object, id: str) -> None: ...
  def {CLOSE}(value: object, id: str = "") -> None: ...
"""
"""CLOSING binds close under the shape: a close that names an act takes any value, since it answers no prompt here."""
OPENED = f"  try:\n    {ENGINE}.lineage('')\n"
"""OPENED opens the ladder on a call, since ty reads an except that no statement before it can reach."""
CAUGHT = "  except BaseException:\n    pass\n"
"""CAUGHT closes the ladder, so that a rung which raised or which never ends leaves the word reachable."""


def compiled(word: str, name: str) -> CodeType:
  """The word as the interpreter runs it: a body of a module, with a top level await allowed.

  A body of a module takes no return, so a word that holds one at its top level does not compile, which is what
  makes a return no python here and a finding of the gate like any other.
  """
  return compile(word, name, "exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)


def laid(text: str, depth: int) -> str:
  """The text as it stands on the sheet, indented into the body, with every line where it was, so a finding keeps
  the line it was found on."""
  return "".join(f"{' ' * depth}{line}\n" if line.strip() else "\n" for line in text.split("\n"))


def sheet(ladder: str, word: str, shape: str) -> tuple[str, int]:
  """The word on a sheet of its own, and how many lines stand above it, which every finding is counted back by."""
  above = HEAD + CLOSING % shape + OPENED + laid(ladder, 4) + CAUGHT
  return above + laid(word, 2), above.count("\n")


def checked(text: str) -> list[tuple[int, str]]:
  """What ty finds on one sheet, each finding by its line.

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
    (int(found["line"]), f"[{found['rule']}] {found['why']}")
    for line in ran.stdout.decode(errors="replace").split("\n")
    if (found := DIAG.match(line.strip())) is not None and found["sort"] == "error"
  ]


NO_TY = "the gate did not run"
"""NO_TY is what the life ends with when ty is not there to read a word, which is no finding against the word."""


class Native:
  """The Kernel of the interpreter this process runs in, where the module of a chain is a dict.

  It answers a gate with what ty finds against the word, begins a run by compiling the word in the module of its
  chain, says wants for the act a run waits for, carries the run forward at each sent, says ran with nothing for a
  word that ran to its end and with the exception for one that raised, and drops the frame of a run a cancel is
  over. A frame that is mid step is never closed: the close of a word raises where that word stands, and what
  unwinds out of it is the drop. One of these serves one life, since the frames it holds and the ladders it gates
  against are that life's own.
  """

  def __init__(self) -> None:
    self.frames: dict[str, CoroutineType[object, object, object]] = {}
    self.ladders: dict[str, list[str]] = {}

  def gate(self, word: str, ladder: list[str], shape: str) -> list[str]:
    """What the gate finds against a word: the word is read for what the interpreter will take and for the name of
    the sheet, and then ty reads it against the ladder of its chain and against the shape it must close with.
    """
    try:
      compiled(word, "<gate>")
    except SyntaxError as bad:
      return ["".join(traceback.format_exception_only(bad)).strip()]
    if (hit := OWN.search(word)) is not None:
      return [f"{hit.group()} is a name of the gate"]
    given = ANY if shape in ("", "None") else shape
    try:
      ast.parse(given, mode="eval")
    except SyntaxError:
      return [f"{shape} is no shape"]
    text, above = sheet("\n".join(ladder), word, given)
    return [f"line {n - above}: {why}" for n, why in checked(text) if n > above]

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
      ran = eval(compiled(word, name), held)  # noqa: S307
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
        case ("run", rung, _, chain, word):
          self.ladders.setdefault(chain, []).append(word)
          self.begin(rung, word, modules[chain])
        case ("sent", rung, _, value) if rung in self.frames:
          self.carry(rung, value)
        case ("gate", qid, _, chain, word, returns):
          yield "done", qid, self.gate(word, self.ladders.get(chain, []), returns)
        case ("cancel" | "close", about, *_):
          # The frame of the word that says the close is mid step, and the CancelledError of close ends that one.
          for one in [x for x in self.frames if under(x, about) and not self.frames[x].cr_running]:
            self.frames[one].close()
            self.ended(one, CancelledError())
