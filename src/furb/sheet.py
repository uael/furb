"""The sheet the gate reads a word on, and the findings it reads off it.

The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word against the
rungs before it, and that a response which is not python is a finding like any other. The reading is ty's, and ty
reads a file: this is that file, written the same way by every Kernel, so that a word is judged the same wherever
the engine runs.

The word stands on a sheet of its own::

  import furb.engine as __engine
  async def __body():
    send = __engine.send          # every name the globals of a chain hold, one to a line
    ...
    try:
      __engine.lineage('')
      <the ladder of the chain>
    except BaseException:
      pass
    <the word>

Every line of it is there for a reason. The body is async, so a word may await at its top level. A name is bound
on a line of its own rather than imported, since a word may rebind a name and no word may rebind an import. The
ladder stands in a try, so a rung that raised or that never ends leaves the word reachable. And the word and the
ladder keep their own lines, so a finding is counted back to the line the model wrote.

The word is read as the body of a module before ty reads what it means: the sheet stands it inside a function,
where a return is legal and the engine takes none, so a word the interpreter will not take as a body is a finding
before the sheet is written. This module runs where the engine runs, in this interpreter or in the sandbox, so it
asks nothing of the interpreter that a word of a rung does not.
"""

import re
from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Sequence

type Checked = Callable[[str], Sequence[tuple[int, str]]]
"""What reads a sheet: ty, in whichever form the Kernel holds it, giving each finding by its line."""

ENGINE = "__engine"
"""ENGINE is the one name of the sheet: the module it binds the names of the engine from, which no word may say."""
OWN = re.compile(rf"\b{ENGINE}\b")
"""OWN finds the name of the sheet in a word."""
BOUND = ("actor", "raised")
"""BOUND are the two names a chain binds of its own: the actor it stands on, and what the last rung raised."""
OPENED = f"  try:\n    {ENGINE}.lineage('')\n"
"""OPENED opens the ladder on a call, since ty reads an except that no statement before it can reach."""
CAUGHT = "  except BaseException:\n    pass\n"
"""CAUGHT closes the ladder, so that a rung which raised or which never ends leaves the word reachable."""


def python(word: str) -> list[str]:
  """What the word is as python, before ty reads what it means: nothing for a word the interpreter takes as the
  body of a module, and the fault for one it does not, or for one that says the name of the sheet."""
  try:
    compile(word, "<gate>", "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT)
  except SyntaxError as bad:
    # Which line and what it said is what the interpreter offers of it: this one gives both, the sandbox the text.
    return [f"line {getattr(bad, 'lineno', None) or 1}: {getattr(bad, 'msg', None) or bad}"]
  if (hit := OWN.search(word)) is not None:
    return [f"{hit.group()} is a name of the gate"]
  return []


def laid(text: str, depth: int) -> str:
  """The text as it stands on the sheet, indented into the body, with every line where it was, so a finding keeps
  the line it was found on."""
  return "".join(f"{' ' * depth}{line}\n" if line.strip() else "\n" for line in text.split("\n"))


def sheet(names: Sequence[str], ladder: Sequence[str], word: str) -> tuple[str, int]:
  """The word on a sheet of its own, and how many lines stand above the word, which every finding is counted back
  by: the names a chain holds, bound one to a line, then the ladder of the chain, then the word."""
  head = f"import furb.engine as {ENGINE}\nasync def __body():\n" + "".join(
    f"  {x} = {ENGINE}.{x}\n" for x in (*names, *BOUND)
  )
  above = head + OPENED + laid("\n".join(ladder), 4) + CAUGHT
  return above + laid(word, 2), above.count("\n")


def gate(names: Sequence[str], ladder: Sequence[str], word: str, checked: Checked) -> list[str]:
  """What the gate finds against a word: the word is read for what the interpreter will take, and then ty reads it
  on its sheet against the ladder of its chain, and each finding below the word is counted back to its line."""
  if found := python(word):
    return found
  text, above = sheet(names, ladder, word)
  return [f"line {n - above}: {why}" for n, why in checked(text) if n > above]
