"""What the engine needs of the interpreter that runs it, and what the suite needs on top, read off both.

The engine runs in a sandbox, and the preamble runs beside it. Those two files are everything that runs in there,
so what they use is everything the sandbox must have. This reads them and says it: the modules they import and
what they take from each, the builtins they call, the syntax they are written in, and the attributes they reach
for. Nothing of it is a list somebody keeps by hand, so it is right on the day it is read.

It says the same of the suite, which is the second thing that may run in there. A suite that runs where the
engine runs holds the engine to every sentence of the contract in the one place that proof means anything, and
it needs more than the engine does: the names of its own files, which no import of a sandbox finds, so a host
must be able to put a module in there under a name.

    uv run python script/needs.py

It spends nothing and asks no model.
"""

import ast
import builtins
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
"""ROOT is the root of the repository."""
FILES = (ROOT / "src" / "furb" / "engine.py", ROOT / "src" / "preamble.py")
"""FILES are the two files that run in the sandbox for a life, and nothing else runs in there."""
SUITE = (*sorted((ROOT / "test").glob("test_*.py")), ROOT / "test" / "conftest.py")
"""SUITE are the files of the suite, which run in there too when the suite runs where the engine runs."""
APART = "test_hygiene.py"
"""APART is the one file of the suite that is no test of the engine, and that no sandbox ever runs."""
PLAIN = {
  ast.Module,
  ast.Expr,
  ast.Name,
  ast.Load,
  ast.Store,
  ast.Del,
  ast.Constant,
  ast.Attribute,
  ast.Call,
  ast.keyword,
  ast.arg,
  ast.arguments,
  ast.alias,
  ast.Assign,
  ast.AugAssign,
  ast.AnnAssign,
  ast.Return,
  ast.Pass,
  ast.If,
  ast.While,
  ast.For,
  ast.Break,
  ast.Continue,
  ast.Raise,
  ast.Try,
  ast.ExceptHandler,
  ast.With,
  ast.withitem,
  ast.Assert,
  ast.Import,
  ast.ImportFrom,
  ast.Global,
  ast.Nonlocal,
  ast.Lambda,
  ast.BoolOp,
  ast.BinOp,
  ast.UnaryOp,
  ast.Compare,
  ast.IfExp,
  ast.Subscript,
  ast.Slice,
  ast.Tuple,
  ast.List,
  ast.Dict,
  ast.Set,
  ast.Starred,
  ast.FunctionDef,
  ast.ClassDef,
}
"""PLAIN is the syntax every python has had for years, which nothing needs to say it holds."""


def parts(tree: ast.AST) -> dict[str, set[str]]:
  """Everything one file asks of its interpreter, by what it asks for."""
  held: dict[str, set[str]] = {"modules": set(), "builtins": set(), "syntax": set(), "attributes": set()}
  bound = {one.arg for one in ast.walk(tree) if isinstance(one, ast.arg)}
  for node in ast.walk(tree):
    if isinstance(node, ast.Import):
      held["modules"].update(one.name for one in node.names)
    elif isinstance(node, ast.ImportFrom):
      held["modules"].update(f"{node.module}.{one.name}" for one in node.names)
    elif isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load) and hasattr(builtins, node.id):
      if node.id not in bound:
        held["builtins"].add(node.id)
    elif isinstance(node, ast.Attribute):
      held["attributes"].add(node.attr)
    if type(node) not in PLAIN and not isinstance(
      node, (ast.expr_context, ast.boolop, ast.operator, ast.unaryop, ast.cmpop)
    ):
      held["syntax"].add(type(node).__name__)
  return held


def said(name: str, held: set[str]) -> str:
  """One part of what is needed, as one block of lines, with the names in order."""
  each = sorted(held)
  lines = [f"{name} ({len(each)})"]
  while each:
    lines.append("  " + ", ".join(each[:6]))
    each = each[6:]
  return "\n".join(lines)


def whole(files: tuple[Path, ...]) -> dict[str, set[str]]:
  """Everything a group of files asks of its interpreter, said once for the group."""
  held: dict[str, set[str]] = {"modules": set(), "builtins": set(), "syntax": set(), "attributes": set()}
  for at in files:
    for key, one in parts(ast.parse(at.read_text(encoding="utf-8"))).items():
      held[key].update(one)
  return held


def main() -> int:
  """What the files of the sandbox need, and what the suite needs on top of them."""
  engine = whole(FILES)
  sys.stdout.write("What the engine needs of the interpreter that runs it\n\n")
  for key in ("modules", "builtins", "syntax", "attributes"):
    sys.stdout.write(said(key, engine[key]) + "\n\n")
  suite = whole(tuple(at for at in SUITE if at.name != APART))
  sys.stdout.write("What the suite needs on top, to run where the engine runs\n\n")
  for key in ("modules", "builtins", "syntax"):
    sys.stdout.write(said(key, suite[key] - engine[key]) + "\n\n")
  return 0


if __name__ == "__main__":
  sys.exit(main())
