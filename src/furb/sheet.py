"""The sheet the gate reads a word on, and the findings it reads off it.

The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
program of its chain, and that a response which is not python is a finding like any other. The reading is ty's,
and ty reads a file: this is that file, written the same way by every Kernel, so that a word is judged the same
wherever the engine runs.

The word stands on a sheet of its own::

  import __engine__
  async def __body():
    <each name of the engine> = __engine__.<that name>
    actor = ""
    raised: BaseException | None = None
    try:
      acting()
      <a word of the program of the chain before the word>
    except BaseException:
      pass
    <one try of its own for each other word of the program, in order>
    <the word>

Every line of it is there for a reason. The gate gives the checker the engine once, as the module MODULE, which
the sheet alone imports, so a reading costs the program and the word and not the engine. The body is async, so a
word may await at its top level. The module of a chain holds every name of the engine, an import of the engine
among them, so the body binds each of them first, with the type the engine gives it. Each is a plain binding, as
it is in the module of a chain, so a word may read a name of the engine and bind it again to any value, as the
contract lets it. The two names a chain binds of its own come next, with the types the contract gives them. Each
word of the program stands in a try of its own, since a rung that raised keeps what it bound before the raise and
the word after it runs all the same: ty reads the next word through the except, with every name the words before
it bound before they raised, so a word that raises at its top level hides no name a later word binds, and a rung
that never ends leaves the word reachable the same way. A call opens each try, since ty reads an except that no
statement before it can reach, and an empty word still has a body. And the program and the word keep their own
lines, so a finding is counted back to the line the model wrote.

The word is read as the body of a module before ty reads what it means: the sheet stands it inside a function,
where a return is legal and the engine takes none, so a word the interpreter will not take as a body is a finding
before the sheet is written. This module runs where the engine runs, so it asks nothing of the interpreter that a
word of a rung does not.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Generator, Sequence

type Checked = Callable[[str], Sequence[tuple[int, str]]]
"""What reads a sheet: ty, in whichever form the Kernel holds it, giving each finding by its line."""
type Ear = Generator[tuple | None, tuple | None]
"""An ear, as the engine hears one."""

MODULE = "__engine__"
"""MODULE is the name the checker holds the engine under. The sheet alone imports it, and no word knows it, so a
word that imports the engine by the name of its package is refused, as the run refuses it."""
BOUND = '  actor = ""\n  raised: BaseException | None = None\n'
"""BOUND binds the two names a chain binds of its own, the actor it stands on and what the last rung raised, with
the types the contract gives them."""
OPENED = "  try:\n    acting()\n"
"""OPENED opens a word of the program in a try, on a call, since ty reads an except that no statement before it can
reach: a name the word bound before it raised reaches every word after it, and an empty word still has a body."""
CAUGHT = "  except BaseException:\n    pass\n"
"""CAUGHT closes a word of the program, so that a rung which raised or which never ends leaves the words after it
reachable."""


def python(word: str) -> list[str]:
  """What the word is as python, before ty reads what it means: nothing for a word the interpreter takes as the
  body of a module, and the fault for one it does not."""
  try:
    compile(word, "<gate>", "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT)
  except SyntaxError as bad:
    # Which line and what it said is what an interpreter offers of it, and one that offers neither gives the text.
    return [f"line {getattr(bad, 'lineno', None) or 1}: {getattr(bad, 'msg', None) or bad}"]
  return []


def laid(text: str, depth: int) -> str:
  """The text as it stands on the sheet, indented into the body, with every line where it was, so a finding keeps
  the line it was found on."""
  return "".join(f"{' ' * depth}{line}\n" if line.strip() else "\n" for line in text.split("\n"))


def sheet(engine: dict[str, object], program: Sequence[str], word: str) -> tuple[str, int]:
  """The word on a sheet of its own, and how many lines stand above the word, which every finding is counted back
  by: every name of the engine, which is a key of its module that is not private, bound from MODULE; then the two
  names the chain binds; then each word of the program of the chain before the word, in a try of its own; then the
  word."""
  names = "".join(f"  {name} = {MODULE}.{name}\n" for name in engine if not name.startswith("_"))
  words = "".join(OPENED + laid(one, 4) + CAUGHT for one in program)
  above = f"import {MODULE}\nasync def __body():\n{names}{BOUND}{words or OPENED + CAUGHT}"
  return above + laid(word, 2), above.count("\n")


def gate(engine: dict[str, object], program: Sequence[str], word: str, checked: Checked) -> list[str]:
  """What the gate finds against a word: the word is read for what the interpreter will take, and then ty reads it
  on its sheet after the program of its chain, and each finding below the word is counted back to its line."""
  if found := python(word):
    return found
  text, above = sheet(engine, program, word)
  return [f"line {n - above}: {why}" for n, why in checked(text) if n > above]


def gating(engine: dict[str, object], checked: Checked) -> Ear:
  """The gate as the ear of a life: it answers each gate with what it finds against the word on the sheet of the
  engine whose names it is given, after the program the gate says."""
  while True:
    match (yield):
      case ("gate", qid, _, _, word, program):
        yield "done", qid, gate(engine, [*program.values()], word, checked)
