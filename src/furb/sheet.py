"""The sheet the gate reads a word on, and the findings it reads off it.

The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
program of its chain, and that a response which is not python is a finding like any other. The reading is ty's,
and ty reads a file: this is that file, written the same way by every Kernel, so that a word is judged the same
wherever the engine runs.

The word stands on a sheet of its own::

  async def __body():
    <the engine>
    actor = ""
    raised: BaseException | None = None
    try:
      lineage("")
      <the program of the chain before the word>
    except BaseException:
      pass
    <the word>

Every line of it is there for a reason. The body is async, so a word may await at its top level. The engine is
laid first, whole, as the first rung of every chain: the module of a chain is the engine run as a word, so every
name the engine binds, an import of its own among them, is a name the word may say, with the type the engine
gives it. The two names a chain binds of its own come next, with the types the contract gives them. The program
stands in a try, since a rung that raised keeps what it bound before the raise and the word after it runs all the
same: ty reads the word through the except, with every name the program bound before it raised, and a rung that
never ends leaves the word reachable the same way. A call opens the try, since ty reads an except that no
statement before it can reach, and an empty program still has a body. And the engine, the program and the word
keep their own lines, so a finding is counted back to the line the model wrote.

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

BOUND = '  actor = ""\n  raised: BaseException | None = None\n'
"""BOUND binds the two names a chain binds of its own, the actor it stands on and what the last rung raised, with
the types the contract gives them."""
OPENED = '  try:\n    lineage("")\n'
"""OPENED opens the program in a try, on a call, since ty reads an except that no statement before it can reach: a
name the program bound before it raised reaches the word, and an empty program still has a body."""
CAUGHT = "  except BaseException:\n    pass\n"
"""CAUGHT closes the program, so that a rung which raised or which never ends leaves the word reachable."""


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


def sheet(engine: str, program: Sequence[str], word: str) -> tuple[str, int]:
  """The word on a sheet of its own, and how many lines stand above the word, which every finding is counted back
  by: the engine, as the first rung of the chain, then the two names the chain binds, then the program of the chain
  before the word, then the word."""
  above = "async def __body():\n" + laid(engine, 2) + BOUND + OPENED + laid("\n".join(program), 4) + CAUGHT
  return above + laid(word, 2), above.count("\n")


def gate(engine: str, program: Sequence[str], word: str, checked: Checked) -> list[str]:
  """What the gate finds against a word: the word is read for what the interpreter will take, and then ty reads it
  on its sheet after the program of its chain, and each finding below the word is counted back to its line."""
  if found := python(word):
    return found
  text, above = sheet(engine, program, word)
  return [f"line {n - above}: {why}" for n, why in checked(text) if n > above]


def gating(engine: str, checked: Checked) -> Ear:
  """The gate as the ear of a life: it answers each gate with what it finds against the word on the sheet of that
  engine, after the program the gate says."""
  while True:
    match (yield):
      case ("gate", qid, _, _, word, program):
        yield "done", qid, gate(engine, [*program.values()], word, checked)
