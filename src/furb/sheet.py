"""The sheet the gate reads a word on, and the findings it reads off it.

The contract says the word of a rung runs only if the gate accepts it, that the gate reads the word after the
program of its chain, and that a response which is not python is a finding like any other. The reading is ty's,
and ty reads a file: this is that file, written the same way by every Kernel, so that a word is judged the same
wherever the engine runs.

The word stands on a sheet of its own::

  import __engine__
  async def __body():
    <each name ty gives a module that a chain in the sandbox does not hold>: object
    <each name of the engine that is not private> = __engine__.<that name>
    actor = ""
    raised: BaseException | None = None
    __acts = (<the name of each act of the chain>, ...)
    try:
      acting()
      <a word of the program of the chain before the word>
    except BaseException:
      pass
    <one try of its own for each other word of the program, in order>
    <the word>

Every line of it is there for a reason. The gate gives the checker the engine once, as the module MODULE, which
the sheet alone imports, so a reading costs the program and the word and not the engine. The body is async, so a
word may await at its top level. ty gives every module names that a chain in the sandbox does not hold, UNBOUND,
so the body declares each of them and never binds it: ty refuses a word that reads it, as the sandbox does, on
every engine. The module of a chain holds every name of the engine, an import of the engine among them, so the
body binds each of them next, with the type the engine gives it. Each is a plain binding, as it is in the module
of a chain, so a word may read a name of the engine and bind it again to any value, as the contract lets it. The
two names a chain binds of its own come next, with the types the contract gives them. The names of the acts of the
chain come next, as ACTS, which the gate reads: the transcript binds the name of each act and each name of a value
that a tell of it binds, as bash12_stdout, so a word that binds one says what no act gave, and the gate refuses it. Each word of the program
stands in a try of its own, since a rung that raised keeps what it bound before the raise and the word after it
runs all the same: ty reads the next word through the except, with every name the words before it bound before
they raised, so a word that raises at its top level hides no name a later word binds, and a rung that never ends
leaves the word reachable the same way. But ty reads a name after a try through its except too, which may come
before any statement of the try, so it gives a name that a later word binds again each type it was bound to. So
the gate deletes each name that a statement at the top of a word binds again before the try of that word, and the
name has the type of its last binding, as python gives it when the words run in order. A call opens each try, since ty reads an except that no statement before
it can reach, and an empty word still has a body. And the program and the word keep their own lines, so a finding
is counted back to the line the model wrote.

The word is read as the body of a module before ty reads what it means: the sheet stands it inside a function,
where a return is legal and the engine takes none, so a word the interpreter will not take as a body is a finding
before the sheet is written. This module runs where the engine runs, so it asks nothing of the interpreter that a
word of a rung does not.
"""

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from collections.abc import Callable, Generator, Sequence

type Checked = Callable[[str], Sequence[tuple[int, str]]]
"""What reads a sheet: ty, in whichever form the Kernel holds it, giving each finding by its line."""
type Ear = Generator[tuple | set[str] | None, tuple | None]
"""An ear, as the engine hears one."""

MODULE = "__engine__"
"""MODULE is the name the checker holds the engine under. The sheet alone imports it, and no word knows it, so a
word that imports the engine by the name of its package is refused, as the run refuses it."""
UNBOUND = ("__file__", "__path__", "__annotations__", "__annotate__", "__builtins__", "__warningregistry__")
"""UNBOUND are the names ty gives every module as globals that a chain in the sandbox does not hold. ty gives them in
module_type_implicit_global_symbol of ty_python_semantic: the names the typeshed declares in the class
types.ModuleType, but __dict__, __init__ and __getattr__, and __builtins__, __debug__ and __warningregistry__, which
ty adds itself. A chain in the sandbox holds __name__, which the engine binds, and __doc__, __package__, __spec__,
__loader__ and __debug__, which the module of the engine holds before the engine runs. A word is judged the same on
every engine, in the vocabulary of the sandbox, so the gate of python refuses these names too, though python runs
some of them."""
ACTS = "__acts"
"""ACTS is the name the sheet binds to the names of the acts of the chain, which the gate reads to refuse a word that
binds a name that the transcript binds."""
BOUND = '  actor = ""\n  raised: BaseException | None = None\n'
"""BOUND binds the two names a chain binds of its own."""
OPENED = "  try:\n    acting()\n"
"""OPENED opens the try of a word of the program."""
CAUGHT = "  except BaseException:\n    pass\n"
"""CAUGHT closes the try of a word of the program."""
CUT = 80
"""CUT is the most characters of a line that a finding quotes."""
PROSE = 3
"""PROSE is the fewest words of a line that starts with a capital letter that a finding reads as a sentence."""
STRAY = "a reply is python and quotes only, with no fence, no markup and no prose"
"""STRAY is what a finding adds when a word that is not python holds a fence, the markup of a tool call or prose."""


def quoted(word: str, n: int, why: str) -> str:
  """A finding: the line of the word it is found on, what it says, and the line itself, cut to CUT characters, so a
  model reads what the gate read."""
  lines = word.split("\n")
  line = lines[n - 1].strip() if 0 < n <= len(lines) else ""
  cut = line if len(line) <= CUT else line[: CUT - 3] + "..."
  return f"line {n}: {why}, in {cut!r}" if cut else f"line {n}: {why}"


def stray(line: str) -> bool:
  """Whether a line is no python a model means: a fence of markdown, the markup of a tool call, or a sentence of
  prose."""
  text = line.strip()
  words = text.split()
  return (
    text.startswith(("```", "~~~"))
    or (text.startswith("<") and not text.startswith("<s:"))
    or (len(words) >= PROSE and words[0][:1].isupper() and all(w.strip(".,:;!?'").isalpha() for w in words))
  )


def python(word: str) -> list[str]:
  """What the word is as python, before ty reads what it means: nothing for a word the interpreter takes as the
  body of a module, and the fault for one it does not."""
  lines = word.split("\n")
  # A quote that has a close mark is a string already, so an open mark that stands as it is has none.
  marks = [(n, one.strip()[3:-1]) for n, one in enumerate(lines, 1) if one.strip()[3:-1].isidentifier()]
  opened = [(n, name) for n, name in marks if lines[n - 1].strip() == f"<s:{name}>"]
  if opened:
    return [
      quoted(word, n, f"the quote <s:{name}> has no close mark </s:{name}> at the end of a line") for n, name in opened
    ]
  try:
    compile(word, "<gate>", "exec", flags=PyCF_ALLOW_TOP_LEVEL_AWAIT)
  except SyntaxError as bad:
    # Which line and what it said is what an interpreter offers of it, and one that offers neither gives the text.
    why = str(getattr(bad, "msg", None) or bad)
    if any(stray(one) for one in lines):
      why += f"; {STRAY}"
    # An interpreter that gives no line number may name the line at the end of its text, as (<string>, line 2).
    tail = str(bad).rpartition(", line ")[2].removesuffix(")")
    return [quoted(word, getattr(bad, "lineno", None) or (int(tail) if tail.isdigit() else 1), why)]
  return []


def laid(text: str, depth: int) -> str:
  """The text as it stands on the sheet, indented into the body, with every line where it was, so a finding keeps
  the line it was found on."""
  return "".join(f"{' ' * depth}{line}\n" if line.strip() else "\n" for line in text.split("\n"))


def sheet(engine: dict[str, object], program: Sequence[str], word: str, acts: Sequence[str] = ()) -> tuple[str, int]:
  """The word on its sheet, laid out as this module says, and how many lines stand above the word, which every
  finding is counted back by."""
  unbound = "".join(f"  {name}: object\n" for name in UNBOUND)
  names = "".join(f"  {name} = {MODULE}.{name}\n" for name in engine if not name.startswith("_"))
  words = "".join(OPENED + laid(one, 4) + CAUGHT for one in program)
  above = f"import {MODULE}\nasync def __body():\n{unbound}{names}{BOUND}  {ACTS} = {tuple(acts)!r}\n{words or OPENED + CAUGHT}"
  return above + laid(word, 2), above.count("\n")


def gate(
  engine: dict[str, object], program: Sequence[str], word: str, checked: Checked, acts: Sequence[str] = ()
) -> list[str]:
  """What the gate finds against a word: the word is read for what the interpreter will take, and then ty reads it
  on its sheet after the program of its chain, and each finding below the word is counted back to its line."""
  if found := python(word):
    return found
  text, above = sheet(engine, program, word, acts)
  return [quoted(word, n - above, why) for n, why in checked(text) if n > above]


def against(engine: dict[str, object], checked: Checked, on: str, word: str) -> list[str]:
  """What the gate finds against a word on the sheet of the engine whose names it is given, after the program of its
  chain as it stands now, with the names of the acts of the chain. It makes no fact, so a host may read it while a
  model still writes the word."""
  program, question, transcript = engine["program"], engine["question"], engine["transcript"]
  assert callable(program)
  assert callable(question)
  assert callable(transcript)
  words, facts = program(on), transcript(on)
  assert isinstance(words, dict)
  assert isinstance(facts, list)
  return gate(engine, [*words.values()], word, checked, [str(x[1]) for x in facts if question(x)])


def gating(engine: dict[str, object], checked: Checked) -> Ear:
  """The gate as the ear of a life: it answers each gate with what it finds against the word, on its chain as the
  chain stands when the gate takes it."""
  gates = {"gate"}
  while True:
    match (yield gates):
      case ("gate", qid, _, on, word):
        yield "done", qid, against(engine, checked, on, word)
