"""Compare the contract of a build with the contract of the last release, and name each change that breaks a caller.

The contract is `src/furb/engine.pyi`, the interface of each official kit, `extensions/<name>/<name>.pyi`, and the
closed set of the nodes of a view in `host/protocol.schema.json`, each node with its fields. The
script reads each one at the last tag, with `git show <tag>:<path>`, and in the working tree. It reads a contract as the
signatures of its top-level definitions, the fields and the methods of each class, its constants and its aliases, and
the sentences of its docstrings. A change is a break, a wording or a minor.

- A break: a definition, a method, a field, a keyword or a constant that goes; a type, an alias or an order that
  changes; a keyword that is more and has no default, or that stands before one that a caller may pass by position; a
  default that changes or goes; and the value of a constant that changes, unless the line of the policy names it.
- A wording: a sentence of a docstring that changes or goes. A change of meaning is a major, so it is a break unless
  the pull request holds the label wording.
- A minor: a name more, a keyword more with a default that no caller passes by position, a default that a keyword
  gains, a sentence more, and the value of a policy that changes.
- Of the nodes of a view, which only grow within a major: a node or a field of a node that goes is a break, and a node
  or a field more is a minor.

The line of the policy is the sentence of the contract that starts `A minor may change the default of each policy:`.
The names of the release rule when it holds the line, so one pull request cannot add a name and change its value;
the names of the working tree rule before the first release that holds the line.

With a title, the breaks are declared when the title holds `!` before its colon, and the script says so and passes.
Without a title, it fails on each break.

With --since REF, a change that stands already at REF is known: a pull request declares its own breaks, and a break that
its base holds, declared by an earlier pull request, fails no later one until the next release.

Usage: python3 script/contract.py [--base TAG] [--since REF] [--title TITLE] [--wording]
"""

import argparse
import ast
import inspect
import json
import re
import shutil
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENGINE = "src/furb/engine.pyi"
SCHEMA = "host/protocol.schema.json"
KIT = re.compile(r"extensions/[^/]+/[^/]+\.pyi")
POLICY = "A minor may change the default of each policy:"
POSITIONAL = ("posonly", "pos")

type Change = tuple[str, str]


@dataclass(frozen=True)
class Param:
  name: str
  kind: str
  annotation: str
  default: str | None


@dataclass(frozen=True)
class Sig:
  asynchronous: bool
  params: tuple[Param, ...]
  returns: str


@dataclass
class Item:
  kind: str
  sigs: list[Sig] = field(default_factory=list)
  decorators: tuple[str, ...] = ()
  bases: tuple[str, ...] = ()
  tparams: str = ""
  annotation: str = ""
  value: str | None = None
  doc: list[str] = field(default_factory=list)


@dataclass
class Contract:
  items: dict[str, Item] = field(default_factory=dict)
  free: list[str] = field(default_factory=list)
  policies: set[str] = field(default_factory=set)
  line: bool = False


@dataclass(frozen=True)
class Finding:
  level: str
  where: str
  what: str

  def __str__(self) -> str:
    return f"{self.level}: {self.where}: {self.what}"


def text_of(node: ast.expr | None) -> str:
  return "" if node is None else ast.unparse(node)


def sentences(text: str | None) -> list[str]:
  return [x.strip() for x in (text or "").splitlines() if x.strip()]


def sig_of(node: ast.FunctionDef | ast.AsyncFunctionDef) -> Sig:
  a = node.args
  positional = [*a.posonlyargs, *a.args]
  defaults = [None] * (len(positional) - len(a.defaults)) + list(a.defaults)
  params = [
    Param(arg.arg, "posonly" if n < len(a.posonlyargs) else "pos", text_of(arg.annotation), text_of(d) if d else None)
    for n, (arg, d) in enumerate(zip(positional, defaults, strict=True))
  ]
  if a.vararg:
    params.append(Param(a.vararg.arg, "*", text_of(a.vararg.annotation), None))
  for arg, d in zip(a.kwonlyargs, a.kw_defaults, strict=True):
    params.append(Param(arg.arg, "kwonly", text_of(arg.annotation), text_of(d) if d else None))
  if a.kwarg:
    params.append(Param(a.kwarg.arg, "**", text_of(a.kwarg.annotation), None))
  return Sig(isinstance(node, ast.AsyncFunctionDef), tuple(params), text_of(node.returns))


def class_of(node: ast.ClassDef) -> Item:
  return Item(
    "class",
    decorators=tuple(ast.unparse(d) for d in node.decorator_list),
    bases=tuple(ast.unparse(b) for b in node.bases),
    tparams=", ".join(ast.unparse(p) for p in node.type_params),
    doc=sentences(ast.get_docstring(node)),
  )


def collect(body: list[ast.stmt], prefix: str, found: Contract) -> None:
  last: Item | None = None
  for node in body:
    made: Item | None = None
    match node:
      case ast.FunctionDef() | ast.AsyncFunctionDef():
        shown = tuple(ast.unparse(d) for d in node.decorator_list)
        kept = tuple(d for d in shown if d.split(".")[-1] != "overload")
        item = found.items.setdefault(prefix + node.name, Item("def", decorators=kept))
        item.sigs.append(sig_of(node))
        item.doc += sentences(ast.get_docstring(node))
      case ast.ClassDef():
        found.items[prefix + node.name] = class_of(node)
        has = ast.get_docstring(node, clean=False) is not None
        collect(node.body[1:] if has else node.body, prefix + node.name + ".", found)
      case ast.TypeAlias():
        params = ", ".join(ast.unparse(p) for p in node.type_params)
        made = Item("alias", tparams=params, value=text_of(node.value))
        found.items[prefix + text_of(node.name)] = made
      case ast.AnnAssign(target=ast.Name(id=name)):
        made = Item("field" if prefix else "const", annotation=text_of(node.annotation))
        made.value = text_of(node.value) if node.value else None
        found.items[prefix + name] = made
      case ast.Assign(targets=[ast.Name(id=name)]):
        made = Item("field" if prefix else "const", value=text_of(node.value))
        found.items[prefix + name] = made
      case ast.Expr(value=ast.Constant(value=str() as text)):
        if last is not None and not last.doc:
          last.doc = sentences(inspect.cleandoc(text))
        else:
          found.free += sentences(inspect.cleandoc(text))
      case _:
        pass
    last = made


def read_contract(text: str) -> Contract:
  found = Contract()
  collect(ast.parse(text).body, "", found)
  kept = []
  for one in found.free:
    if one.startswith(POLICY):
      found.line = True
      found.policies |= set(re.findall(r"[A-Z][A-Z_]*[A-Z]", one.split(":", 1)[1].split(".", 1)[0]))
    else:
      kept.append(one)
  found.free = kept
  return found


def canon(sig: Sig) -> str:
  parts = [
    f"{p.kind} {p.name}: {p.annotation}" + (f" = {p.default}" if p.default is not None else "") for p in sig.params
  ]
  return f"{'async ' if sig.asynchronous else ''}({', '.join(parts)}) -> {sig.returns}"


def param_changes(p: Param, q: Param) -> list[Change]:
  found: list[Change] = []
  if p.kind != q.kind:
    found.append(("break", f"the parameter {p.name} changes from {p.kind} to {q.kind}"))
  if p.annotation != q.annotation:
    found.append(("break", f"the type of {p.name} changes from {p.annotation or 'none'} to {q.annotation or 'none'}"))
  if p.default is not None and q.default is None:
    found.append(("break", f"the default of {p.name} goes"))
  elif p.default is not None and p.default != q.default:
    found.append(("break", f"the default of {p.name} changes from {p.default} to {q.default}"))
  elif p.default is None and q.default is not None:
    found.append(("minor", f"the parameter {p.name} gains the default {q.default}"))
  return found


def added_param(q: Param, n: int, last: int) -> Change:
  if q.kind in ("*", "**"):
    return ("minor", f"the parameter {q.name} is more")
  if q.default is None:
    return ("break", f"the parameter {q.name} is more and has no default")
  if q.kind in POSITIONAL and n < last:
    return ("break", f"the parameter {q.name} stands before a parameter that a caller passes by position")
  return ("minor", f"the parameter {q.name} is more, with the default {q.default}")


def signature(old: Sig, new: Sig) -> list[Change]:
  found: list[Change] = []
  if old.asynchronous != new.asynchronous:
    found.append(("break", "it becomes " + ("async" if new.asynchronous else "sync")))
  if old.returns != new.returns:
    found.append(("break", f"its type of return changes from {old.returns or 'none'} to {new.returns or 'none'}"))
  now = {p.name: p for p in new.params}
  was = {p.name for p in old.params}
  for p in old.params:
    q = now.get(p.name)
    found += [("break", f"the parameter {p.name} goes")] if q is None else param_changes(p, q)
  order = [p.name for p in old.params if p.kind in POSITIONAL and p.name in now]
  after = [p.name for p in new.params if p.kind in POSITIONAL and p.name in was]
  if order != after:
    found.append(("break", f"the order of the positional parameters changes from {order} to {after}"))
  last = max((n for n, p in enumerate(new.params) if p.name in was and p.kind in POSITIONAL), default=-1)
  return found + [added_param(q, n, last) for n, q in enumerate(new.params) if q.name not in was]


def def_changes(a: Item, b: Item) -> list[Change]:
  found: list[Change] = []
  if a.decorators != b.decorators:
    found.append(("break", f"its decorators change from {list(a.decorators)} to {list(b.decorators)}"))
  if len(a.sigs) == 1 == len(b.sigs):
    return found + signature(a.sigs[0], b.sigs[0])
  found += [("break", f"the overload {canon(one)} goes or changes") for one in a.sigs if one not in b.sigs]
  return found + [("minor", f"the overload {canon(one)} is more") for one in b.sigs if one not in a.sigs]


def class_changes(a: Item, b: Item) -> list[Change]:
  found = [("break", f"the base {one} goes") for one in a.bases if one not in b.bases]
  found += [("minor", f"the base {one} is more") for one in b.bases if one not in a.bases]
  if a.decorators != b.decorators:
    found.append(("break", f"its decorators change from {list(a.decorators)} to {list(b.decorators)}"))
  if a.tparams != b.tparams:
    found.append(("break", f"its type parameters change from [{a.tparams}] to [{b.tparams}]"))
  return found


def alias_changes(a: Item, b: Item) -> list[Change]:
  found: list[Change] = []
  if a.tparams != b.tparams:
    found.append(("break", f"its type parameters change from [{a.tparams}] to [{b.tparams}]"))
  if a.value != b.value:
    found.append(("break", f"the alias changes from {a.value} to {b.value}"))
  return found


def value_changes(name: str, a: Item, b: Item, allowed: set[str]) -> list[Change]:
  found: list[Change] = []
  if a.annotation != b.annotation:
    found.append(("break", f"its type changes from {a.annotation or 'none'} to {b.annotation or 'none'}"))
  if a.value != b.value and name in allowed:
    found.append(("minor", f"the policy changes from {a.value} to {b.value}"))
  elif a.value != b.value:
    found.append(("break", f"its value changes from {a.value} to {b.value}"))
  return found


def sentence_changes(old: list[str], new: list[str]) -> list[Change]:
  found = [("wording", f"a sentence changes or goes: {one[:100]}") for one in (Counter(old) - Counter(new)).elements()]
  return found + [("minor", f"a sentence is more: {one[:100]}") for one in (Counter(new) - Counter(old)).elements()]


def item_changes(name: str, a: Item, b: Item, allowed: set[str]) -> list[Change]:
  if a.kind != b.kind:
    return [("break", f"the {a.kind} becomes a {b.kind}")]
  match a.kind:
    case "def":
      found = def_changes(a, b)
    case "class":
      found = class_changes(a, b)
    case "alias":
      found = alias_changes(a, b)
    case _:
      found = value_changes(name, a, b, allowed)
  return found + sentence_changes(a.doc, b.doc)


def added(name: str, b: Item, old: Contract) -> Change:
  parent = old.items.get(name.rpartition(".")[0])
  dataclass = parent is not None and any(d.split("(")[0].split(".")[-1] == "dataclass" for d in parent.decorators)
  needs = b.kind == "field" and b.value is None and dataclass
  return ("break" if needs else "minor", f"the {b.kind} is more" + (" and has no default" if needs else ""))


def compare(where: str, old: Contract, new: Contract, allowed: set[str]) -> list[Finding]:
  def at(name: str, changes: list[Change]) -> list[Finding]:
    return [Finding(level, f"{where} {name}".rstrip(), what) for level, what in changes]

  found: list[Finding] = []
  for name, a in old.items.items():
    b = new.items.get(name)
    found += at(name, [("break", f"the {a.kind} goes")] if b is None else item_changes(name, a, b, allowed))
  for name, b in new.items.items():
    if name not in old.items:
      found += at(name, [added(name, b, old)])
  return found + at("", sentence_changes(old.free, new.free))


def diff(where: str, old: str | None, new: str | None) -> list[Finding]:
  if old is None:
    return [] if new is None else [Finding("minor", where, "the contract is more")]
  if new is None:
    return [Finding("break", where, "the contract goes")]
  before, after = read_contract(old), read_contract(new)
  allowed = before.policies if before.line else after.policies
  return compare(where, before, after, allowed)


def nodes(text: str | None) -> dict[str, set[str]]:
  """Each node of a view that the schema of the protocol gives, with its fields: those of every node and its own."""
  if text is None:
    return {}
  defs = json.loads(text).get("definitions", {})
  node = defs.get("Node", {})
  shared = set(node.get("properties", {}))
  found: dict[str, set[str]] = {}
  for one in node.get("oneOf", []):
    props = one.get("properties", {})
    name = props.get("node", {}).get("const")
    if isinstance(name, str):
      found[name] = shared | (set(props) - {"node"})
  return found


def node_diff(where: str, old: str | None, new: str | None) -> list[Finding]:
  before, after = nodes(old), nodes(new)
  found = [Finding("break", where, f"the node {name} goes") for name in sorted(before.keys() - after.keys())]
  found += [Finding("minor", where, f"the node {name} is more") for name in sorted(after.keys() - before.keys())]
  for name in sorted(before.keys() & after.keys()):
    found += [
      Finding("break", where, f"the field {f} of the node {name} goes") for f in sorted(before[name] - after[name])
    ]
    found += [
      Finding("minor", where, f"the field {f} of the node {name} is more") for f in sorted(after[name] - before[name])
    ]
  return found


def git(root: Path, *args: str) -> tuple[int, str]:
  got = subprocess.run([shutil.which("git") or "git", *args], cwd=root, capture_output=True, text=True, check=False)  # noqa: S603
  return got.returncode, got.stdout


def last_tag(root: Path) -> str:
  _, out = git(root, "tag", "--list", "v[0-9]*", "--sort=-v:refname")
  return next(iter(out.split()), "")


def paths(root: Path, *refs: str) -> list[str]:
  here = {x.relative_to(root).as_posix() for x in root.glob("extensions/*/*.pyi")}
  there: set[str] = set()
  for ref in refs:
    _, out = git(root, "ls-tree", "-r", "--name-only", ref, "--", "extensions")
    there |= {x for x in out.split() if KIT.fullmatch(x)}
  return [ENGINE, *sorted(here | there), SCHEMA]


def declared(title: str) -> bool:
  return re.match(r"[^:\n]*!:", title) is not None


def compared(root: Path, tag: str, files: list[str], ref: str = "") -> list[Finding]:
  found: list[Finding] = []
  for path in files:
    code, old = git(root, "show", f"{tag}:{path}")
    if ref:
      kept, text = git(root, "show", f"{ref}:{path}")
      new = text if kept == 0 else None
    else:
      file = root / path
      new = file.read_text(encoding="utf-8") if file.exists() else None
    found += (node_diff if path == SCHEMA else diff)(path, old if code == 0 else None, new)
  return found


def main(argv: list[str] | None = None, root: Path = ROOT) -> int:
  parser = argparse.ArgumentParser(description="Compare the contract with the one of the last release.")
  parser.add_argument("--base", default="", help="The tag to compare with, the last release when it is empty.")
  parser.add_argument("--since", default="", help="A ref whose changes against the tag are known, as a base.")
  parser.add_argument("--title", default=None, help="The title of the pull request, whose ! declares the breaks.")
  parser.add_argument("--wording", action="store_true", help="The pull request holds the label wording.")
  args = parser.parse_args(argv)
  tag = args.base or last_tag(root)
  if not tag:
    sys.stdout.write("No release to compare with: nothing is a break yet.\n")
    return 0
  files = paths(root, tag, *([args.since] if args.since else []))
  known = {str(x) for x in compared(root, tag, files, args.since)} if args.since else set()
  found = [x for x in compared(root, tag, files) if str(x) not in known]
  for one in found:
    sys.stdout.write(f"{one}\n")
  blocking = [x for x in found if x.level == "break" or (x.level == "wording" and not args.wording)]
  counts = Counter(x.level for x in found)
  stand = f" {len(known)} changes already stand on {args.since}." if args.since else ""
  sys.stdout.write(
    f"Against {tag}: {counts['break']} breaks, {counts['wording']} sentences that change, {counts['minor']} minors.{stand}\n"
  )
  if not blocking:
    return 0
  if args.title is not None and declared(args.title):
    sys.stdout.write(f"The title holds !, so it declares {len(blocking)} breaks.\n")
    return 0
  sys.stderr.write(
    f"{len(blocking)} changes break a caller of the contract, and the title does not hold !: write the title as"
    " type(area)!: What changed, or, for a sentence that changes with no change of meaning, give the pull request the"
    " label wording.\n"
  )
  return 1


if __name__ == "__main__":
  sys.exit(main())
