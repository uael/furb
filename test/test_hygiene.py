"""The hygiene laws, which CLAUDE.md says: the contract and the suite agree, sentence for sentence."""

import ast
import re
from collections import Counter
from pathlib import Path

import tiktoken

from furb import engine
from furb_monty import _monty
from furb_monty._monty import SYSTEM

PY = Path(engine.__file__)
PYI = PY.with_suffix(".pyi")
TESTS = PYI.parents[2] / "test"
EXTENSIONS = TESTS.parent / "extensions"
# The contract of the engine and the contract of each extension, each with the folder of its suite.
CONTRACTS = {PYI: TESTS, **{one: one.parent / "test" for one in sorted(EXTENSIONS.glob("*/*.pyi"))}}
# What the engine may cost the model that reads it: a wall, and a shape that will not fit under it is a shape not
# found yet. Six thousand, by the owner's word, for the engine alone, and two thousand for the doctrine after it.
BUDGET = 6_000
DOCTRINE = 2_000
ENGINE, _, QUOTE = SYSTEM.partition("\n<s:doctrine>")
DEFS = (ast.FunctionDef, ast.AsyncFunctionDef)
SCOPE = (ast.Module, ast.ClassDef, ast.Lambda, ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp, *DEFS)


def normal(sentence: str) -> str:
  return re.sub(r"\s+", " ", sentence).strip()


def defs(tree: ast.Module) -> dict[str, list[str]]:
  """Every definition by its dotted name, with its sentences; overloads of one name share it."""
  out: dict[str, list[str]] = {}
  for i, node in enumerate(tree.body):
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
      out.setdefault(node.name, []).extend(sentences(ast.get_docstring(node, clean=True)))
    elif isinstance(node, ast.ClassDef):
      out.setdefault(node.name, []).extend(sentences(ast.get_docstring(node, clean=True)))
      for member in node.body:
        if (
          isinstance(member, ast.FunctionDef | ast.AsyncFunctionDef)
          and member.name != "__init__"
          and not property_(member)
        ):
          out.setdefault(f"{node.name}.{member.name}", []).extend(sentences(ast.get_docstring(member, clean=True)))
    elif isinstance(node, ast.AnnAssign | ast.Assign | ast.TypeAlias):
      after = tree.body[i + 1] if i + 1 < len(tree.body) else None
      doc = after.value.value if isinstance(after, ast.Expr) and isinstance(after.value, ast.Constant) else None
      for name in globals_of(node):
        out.setdefault(name, []).extend(sentences(doc if isinstance(doc, str) else None))
  return out


def property_(member: ast.FunctionDef | ast.AsyncFunctionDef) -> bool:
  return any(isinstance(d, ast.Name) and d.id == "property" for d in member.decorator_list)


def globals_of(node: ast.AnnAssign | ast.Assign | ast.TypeAlias) -> list[str]:
  targets = (
    node.targets if isinstance(node, ast.Assign) else [node.target if isinstance(node, ast.AnnAssign) else node.name]
  )
  return [target.id for target in targets if isinstance(target, ast.Name)]


def sentences(doc: str | None) -> list[str]:
  return [normal(line) for line in (doc or "").splitlines() if line.strip()]


def file_of(name: str, taken: set[str], tests: Path) -> Path:
  """test_<name>.py for a function, a class or a global, test_<class>_<method>.py for a method, in lower case, dunders bare.

  A class whose lower-case name is another definition's, Bash beside bash or Head beside HEAD, has test_<class>_shape.py.
  """
  parts = [part.strip("_").lower() for part in name.split(".")]
  if "." not in name and name[0].isupper() and not name.isupper() and parts[0] in taken:
    parts.append("shape")
  return tests / ("test_" + "_".join(parts) + ".py")


def taken_by_others(named: dict[str, list[str]]) -> set[str]:
  """The lower-case names of the top-level definitions that are not classes."""
  return {name.lower() for name in named if "." not in name and not (name[0].isupper() and not name.isupper())}


def carried_by(path: Path) -> list[str]:
  tree = ast.parse(path.read_text(encoding="utf-8"))
  return [
    normal(ast.get_docstring(node, clean=True) or "")
    for node in ast.walk(tree)
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef) and node.name.startswith("test_")
  ]


def suite(tests: Path) -> dict[Path, list[str]]:
  here = Path(__file__)
  return {p: carried_by(p) for p in sorted(tests.glob("test_*.py")) if p != here}


def test_every_sentence_has_one_test_in_the_file_of_its_definition() -> None:
  missing = []
  for contract, tests in CONTRACTS.items():
    carried = suite(tests)
    named = defs(ast.parse(contract.read_text(encoding="utf-8")))
    taken = taken_by_others(named)
    for name, said in named.items():
      counts = Counter(carried.get(file_of(name, taken, tests), []))
      missing.extend((contract.name, name, s, counts[s]) for s in said if counts[s] != 1)
  assert missing == []


def test_every_test_carries_a_sentence_of_the_contract() -> None:
  stray = []
  for contract, tests in CONTRACTS.items():
    named = defs(ast.parse(contract.read_text(encoding="utf-8")))
    taken = taken_by_others(named)
    said = {(file_of(name, taken, tests), s) for name, lines in named.items() for s in lines}
    stray.extend((p.name, s) for p, docs in suite(tests).items() for s in docs if (p, s) not in said)
  assert stray == []


def test_every_definition_has_a_file_and_a_sentence() -> None:
  for contract, tests in CONTRACTS.items():
    named = defs(ast.parse(contract.read_text(encoding="utf-8")))
    taken = taken_by_others(named)
    files = {name: file_of(name, taken, tests) for name in named}
    shared = Counter(files.values())
    assert [name for name, path in files.items() if shared[path] > 1] == []
    assert [name for name, path in files.items() if not path.exists()] == []
    assert [name for name, said in named.items() if not said] == []


def test_the_module_docstring_is_empty() -> None:
  for contract in CONTRACTS:
    assert ast.get_docstring(ast.parse(contract.read_text(encoding="utf-8"))) is None


def test_the_engine_fits_the_window_it_is_meant_to_be_read_in() -> None:
  spent = len(tiktoken.get_encoding("o200k_base").encode(ENGINE))
  assert spent < BUDGET, f"the engine the model reads costs {spent} tokens, over the {BUDGET} budget"


def test_the_doctrine_fits_the_window_it_is_meant_to_be_read_in() -> None:
  spent = len(tiktoken.get_encoding("o200k_base").encode(f"<s:doctrine>{QUOTE}"))
  assert spent <= DOCTRINE, f"the doctrine costs {spent} tokens, over the {DOCTRINE} budget"


def test_the_engine_the_model_reads_is_the_engine_that_runs() -> None:
  """Whatever the minifier takes, it may not be meaning: what the model is handed parses to the program on disk."""
  assert ast.dump(ast.parse(engine.unquoted(SYSTEM))) == ast.dump(ast.parse(PY.read_text(encoding="utf-8")))


def binds(scope: ast.AST) -> set[str]:
  """Every name this scope binds in its own code, which is what shadows an outer name of the same spelling."""
  out: set[str] = set()
  far: set[str] = set()
  if isinstance(scope, (*DEFS, ast.Lambda)):
    a = scope.args
    out.update(x.arg for x in [*a.posonlyargs, *a.args, *a.kwonlyargs, a.vararg, a.kwarg] if x is not None)
  out.update(p.name for p in getattr(scope, "type_params", []))
  if isinstance(scope, ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp):
    for gen in scope.generators:
      out.update(x.id for x in ast.walk(gen.target) if isinstance(x, ast.Name))
    return out

  def visit(n: ast.AST) -> None:
    if isinstance(n, (*DEFS, ast.ClassDef)):
      out.add(n.name)
      return
    if isinstance(n, SCOPE):
      return
    if isinstance(n, ast.Nonlocal | ast.Global):
      far.update(n.names)
    if isinstance(n, ast.Name) and not isinstance(n.ctx, ast.Load):
      out.add(n.id)
    elif isinstance(n, ast.Import | ast.ImportFrom):
      out.update((al.asname or al.name).split(".")[0] for al in n.names)
    elif isinstance(n, ast.ExceptHandler | ast.MatchAs) and n.name:
      out.add(n.name)
    for ch in ast.iter_child_nodes(n):
      visit(ch)

  for st in [scope.body] if isinstance(scope, ast.Lambda) else getattr(scope, "body", []):
    visit(st)
  return out - far


def reaching(tree: ast.Module) -> tuple[dict[int, tuple[ast.AST, ...]], dict[int, set[str]]]:
  """Which scopes enclose every node, and what each of those scopes binds."""
  chain: dict[int, tuple[ast.AST, ...]] = {}

  def walk(n: ast.AST, at: tuple[ast.AST, ...]) -> None:
    chain[id(n)] = at
    for ch in ast.iter_child_nodes(n):
      walk(ch, (*at, ch) if isinstance(ch, SCOPE) else at)

  walk(tree, (tree,))
  held = {id(s): s for at in chain.values() for s in at}
  return chain, {i: binds(s) for i, s in held.items()}


def shadows(text: str) -> list[str]:
  """Every name of a module that a scope binds again beneath a scope that binds it already, with its line."""
  chain, bound = reaching(ast.parse(text))
  dark: list[str] = []
  for at in {id(c[-1]): c for c in chain.values()}.values():
    if isinstance(at[-1], ast.ClassDef):
      continue
    over = set().union(*(bound[id(s)] for s in at[:-1] if not isinstance(s, ast.ClassDef)), set())
    dark.extend(f"{getattr(at[-1], 'lineno', 0)} {name}" for name in sorted(bound[id(at[-1])] & over - {"_"}))
  return dark


def test_no_name_is_bound_again_beneath_itself() -> None:
  """A name bound where an enclosing scope already binds it says two things at once; every word keeps one meaning."""
  assert shadows(PY.read_text(encoding="utf-8")) == [], "these bindings shadow an enclosing one"


def test_the_word_of_an_extension_is_its_file_less_its_imports_of_furb() -> None:
  """The word that a chain runs is the program of the file of the extension, which ruff and ty read."""
  for one in _monty.official():
    name, word = one["name"], one["word"]
    body = ast.parse((EXTENSIONS / name / f"{name}.py").read_text(encoding="utf-8")).body
    kept = [one for one in body if not (isinstance(one, ast.ImportFrom) and (one.module or "").startswith("furb"))]
    assert ast.dump(ast.parse(word)) == ast.dump(ast.Module(body=kept, type_ignores=[]))


def test_the_word_of_an_extension_binds_no_name_of_the_engine_nor_again_beneath_itself() -> None:
  """A word runs in the module of a chain, after the engine, so it keeps the meaning of every name there."""
  held = set(binds(ast.parse(PY.read_text(encoding="utf-8"))))
  for one in _monty.official():
    name, word = one["name"], one["word"]
    assert binds(ast.parse(word)) & held == set(), name
    assert shadows(f"{PY.read_text(encoding='utf-8')}\n{word}") == [], name
