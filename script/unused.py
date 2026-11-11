#!/usr/bin/env python3
"""List each exported name of the TypeScript side and each public item of the crates that no file but a test names.

The first law: no exported name and no public item that nothing but a test names, its own file included. A TypeScript
name is used when a file that is no test, and is not the file that defines it, imports it through a path that leads to
that file, through a barrel or not, or when the README of the package, which says the public door, names it. A Rust
item is used when a file that is no test, and is not the file that defines it, names it, an associated item through its
type or its method call. An item of the door, which a host language calls, is used when a source of that language names
it, and an item that its own file names is used. A name that the declaration of a used name of its own file holds is
used too, since the declaration needs it. The generated protocol.ts is no source. The list is the work: the export
goes, or the item goes. The exit code is 1 when the list is not empty. usage: script/unused.py [--tests]

The second law is not held yet: a helper that its own file names, and that a test names, keeps an export that only
that test uses, so the test reaches into the file past its public door. --tests lists each such helper with the test
files that name it, as the work of that law.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

TS = re.compile(
  r"^export (?:declare )?(?:async )?(?:abstract )?(function\*?|const|let|class|interface|type|enum) (\w+)"
)
RS = re.compile(r"^(\s*)pub (?:const |async |unsafe )*(fn|struct|enum|trait|const|static|type) (\w+)")
IMPL = re.compile(r"^impl(?:<[^>]*>)? (?:[\w:<>, ]+ for )?(\w+)")
FROM = re.compile(r'(import|export)\s+(?:type\s+)?(?:\*\s*(?:as\s+(\w+))?|\{([^}]*)\})\s*from\s*"([^"]+)"')
INLINE = re.compile(r'import\("([^"]+)"\)\.(\w+)')
DOOR = re.compile(r"napi|pyo3|pyclass|pyfunction|pymethods")
PACKAGE = "bind/typescript/src/"
BARRELS = {"furb": PACKAGE + "index.ts", "furb/life": PACKAGE + "life/index.ts"}
BARRELS |= {"furb/protocol": PACKAGE + "protocol.ts", "furb/skill": PACKAGE + "skill.ts"}
SOURCES = ("tui/src/", PACKAGE, "src/", "cli/src/", "host/src/", "build/")
GENERATED = {PACKAGE + "protocol.ts"}
WHOLE = {"interface", "type", "enum", "struct", "trait", "class"}

Used = set[tuple[str, str]]
Item = tuple[int, str, bool]


@dataclass
class World:
  """What the sources say, which decides whether each item is used."""

  imported: Used
  documented: set[str]
  words: dict[str, set[str]]
  calls: dict[str, set[tuple[str, str]]]
  ts: set[str]
  py: set[str]


def tests(path: str) -> bool:
  return bool(re.search(r"(^|/)(test|tests)/|\.test\.(ts|tsx|rs)$|(^|/)test_[^/]*\.py$", path))


def resolve(files: dict[str, str], here: str, spec: str) -> str | None:
  if spec in BARRELS:
    return BARRELS[spec]
  if not spec.startswith("."):
    return None
  base = os.path.normpath(Path(here).parent / re.sub(r"\.(js|ts)$", "", spec))
  return next((x for x in (f"{base}.ts", f"{base}/index.ts") if x in files), None)


def listed(names: str) -> list[str]:
  return [re.sub(r"^type\s+", "", x.strip()).split(" as ")[0].strip() for x in names.split(",") if x.strip()]


def once(text: dict[str, str], path: str, used: Used, stars: dict[str, list[str]], renamed: dict[str, Used]) -> None:
  body = text[path]
  for spec, name in INLINE.findall(body):
    if there := resolve(text, path, spec):
      used.add((there, name))
  for kind, space, names, spec in FROM.findall(body):
    if not (there := resolve(text, path, spec)):
      continue
    if space:
      used.update((there, n) for n in re.findall(rf"\b{space}\.(\w+)", body))
    elif names:
      target = renamed.setdefault(path, set()) if kind == "export" else used
      target.update((there, name) for name in listed(names))
    elif kind == "export":
      stars.setdefault(path, []).append(there)


def imports(text: dict[str, str]) -> Used:
  used: Used = set()
  stars: dict[str, list[str]] = {}
  renamed: dict[str, Used] = {}
  for path in text:
    if path.endswith(".ts") and not tests(path):
      once(text, path, used, stars, renamed)
  while True:
    more: Used = set()
    for there, name in used:
      more.update((source, name) for source in stars.get(there, []))
      more.update((source, name) for source, was in renamed.get(there, set()) if was == name)
    if more <= used:
      return used
    used |= more


def declared(lines: list[str], i: int, *, whole: bool) -> set[str]:
  indent = len(lines[i]) - len(lines[i].lstrip())
  out, depth = [], 0
  for j in range(i, len(lines)):
    line = lines[j]
    ends = line.strip() and len(line) - len(line.lstrip()) <= indent
    if j > i and ends and not line.lstrip().startswith(("}", ")", "]", ">", "|", ".", "&", "+", "?", ":", "=")):
      break
    out.append(line)
    depth += line.count("(") - line.count(")")
    if not whole and depth <= 0:
      break
  return set(re.findall(r"\w+", "\n".join(out)))


def camel(name: str) -> str:
  head, *rest = name.split("_")
  return head + "".join(x.title() for x in rest)


def sources(text: dict[str, str]) -> World:
  words: dict[str, set[str]] = {}
  calls: dict[str, set[tuple[str, str]]] = {}
  for path, body in text.items():
    if not path.endswith(".rs") or tests(path):
      continue
    code = "\n".join(x for x in body.split("\n") if not x.lstrip().startswith("pub use "))
    for word in set(re.findall(r"\w+", code)):
      words.setdefault(word, set()).add(path)
    for owner, name in re.findall(r"\b([A-Z]\w*)::(\w+)", code):
      calls.setdefault(name, set()).add((path, owner))
    for name in re.findall(r"\.(\w+)(?:::<[^(]*>)?\(", code):
      calls.setdefault(name, set()).add((path, ""))
  host = {"ts": set(), "py": set()}
  for path, body in text.items():
    if path.endswith((".ts", ".py")) and not tests(path):
      host[path[-2:] if path.endswith(".py") else "ts"].update(re.findall(r"\w+", body))
  documented = set(re.findall(r"\w+", Path("bind/typescript/README.md").read_text()))
  return World(imports(text), documented, words, calls, host["ts"], host["py"])


def rust_seen(path: str, m: re.Match[str], owner: str, near: str, world: World) -> bool:
  indent, kind, name = m[1], m[2], m[3]
  if DOOR.search(near) and ((camel(name) if kind == "fn" else name) in world.ts or name in world.py):
    return True
  if indent and owner:
    return any(p != path and o in ("", owner) for p, o in world.calls.get(name, ()))
  return bool(world.words.get(name, set()) - {path})


def items_of(path: str, lines: list[str], world: World) -> dict[str, Item]:
  items: dict[str, Item] = {}
  owner = ""
  for n, line in enumerate(lines):
    if path.endswith(".ts"):
      if m := TS.match(line):
        items[m[2]] = (n, m[1], (path, m[2]) in world.imported or m[2] in world.documented)
      continue
    if line.startswith("}"):
      owner = ""
    if im := IMPL.match(line):
      owner = im[1]
    if m := RS.match(line):
      items[m[3]] = (n, m[2], rust_seen(path, m, owner, "\n".join(lines[max(0, n - 12) : n]), world))
  return items


def scan(path: str, body: str, world: World) -> tuple[list[str], list[tuple[str, str]]]:
  lines = body.split("\n")
  items = items_of(path, lines, world)
  code = re.sub(r"//[^\n]*", "", re.sub(r"/\*.*?\*/", "", body, flags=re.S))
  counts = {name: len(re.findall(rf"\b{name}\b", code)) for name in items}
  used = {name for name, (_, _, seen) in items.items() if seen or counts[name] > 1}
  queue = list(used)
  while queue:
    name = queue.pop()
    for other in declared(lines, items[name][0], whole=items[name][1] in WHOLE) & items.keys():
      if other not in used:
        used.add(other)
        queue.append(other)
  found = [f"{path}:{n + 1}: {name}" for name, (n, _, _) in items.items() if name not in used]
  alone = [(path, name) for name, (_, _, seen) in items.items() if not seen and counts[name] > 1]
  return found, alone


def main() -> int:
  files = subprocess.check_output(["git", "ls-files"], text=True).split("\n")  # noqa: S607
  text = {}
  for name in files:
    if name.endswith((".ts", ".rs", ".py")) and not name.endswith(".d.ts") and Path(name).is_file():
      text[name] = Path(name).read_text(errors="replace")
  world = sources(text)
  found: list[str] = []
  alone: list[tuple[str, str]] = []
  for path, body in text.items():
    if not tests(path) and path not in GENERATED and path.startswith(SOURCES) and not path.endswith(".py"):
      more, solo = scan(path, body, world)
      found += more
      alone += solo
  if "--tests" in sys.argv:
    for path, name in sorted(alone):
      named = sorted(x for x in text if tests(x) and re.search(rf"\b{name}\b", text[x]))
      if named:
        print(f"{name} {path} {' '.join(named)}")  # noqa: T201
    return 0
  print("\n".join(found))  # noqa: T201
  return 1 if found else 0


sys.exit(main())
