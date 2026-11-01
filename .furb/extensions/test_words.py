"""Each word of the extensions of the project passes the gate on the root chain, after the words that play before it."""

import ast
import inspect
import json
import re
import tomllib
from pathlib import Path

import pytest

import furb
import furb_monty
from furb import sheet
from furb_monty import _monty

PROJECT = Path(__file__).resolve().parent.parent
CONFIG = json.loads((PROJECT / "config.json").read_text(encoding="utf-8"))
NAMES = list(CONFIG["extensions"])
IMPORT = re.compile(r"^(?:from furb(?:\.\w+)* import \([^)]*\)|from furb(?:\.\w+)* import [^\n(]*|import furb[^\n]*)\n", re.M)
BOOTED = [name for node in ast.walk(ast.parse(inspect.getsource(furb.python.boot))) if isinstance(node, ast.Global) for name in node.names]
ENGINE = {**vars(furb.python), **dict.fromkeys(BOOTED)}
"""The names of the engine as a chain holds them: boot binds some of them, so the module alone lacks them."""


def word(name: str) -> str:
  """The word of an extension of the project: its file less the imports of furb, as the crate plays it."""
  folder = PROJECT / CONFIG["extensions"][name]
  manifest = json.loads((folder / "furb.json").read_text(encoding="utf-8"))
  return IMPORT.sub("", (folder / manifest["word"]).read_text(encoding="utf-8"))


@pytest.mark.parametrize("name", NAMES)
def test_word_passes_the_gate(name: str) -> None:
  """The gate refuses a word of the project on every chain, so a stale name there stops the extension in each life."""
  program = {f"rung{n}": one["word"] for n, one in enumerate(_monty.official(), 1)}
  for before in NAMES[: NAMES.index(name)]:
    program[f"rung{len(program) + 1}"] = word(before)
  assert BOOTED
  assert sheet.gate(ENGINE, program, word(name), furb_monty.gate) == []


def defines(text: str) -> set[str]:
  """The names that a word binds at its top: its functions, its classes and its assignments."""
  names: set[str] = set()
  for node in ast.parse(text).body:
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
      names.add(node.name)
    elif isinstance(node, ast.Assign):
      names |= {one.id for one in node.targets if isinstance(one, ast.Name)}
    elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
      names.add(node.target.id)
  return names


def reads(text: str) -> set[str]:
  """The names that a word reads."""
  return {node.id for node in ast.walk(ast.parse(text)) if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load)}


def test_each_builtin_of_ruff_is_a_name_that_a_word_defines_for_a_word_after_it() -> None:
  """Ruff reads a name of another word as a builtin, so a name that no word before the reader defines hides a fault."""
  builtins = tomllib.loads((PROJECT.parent / "pyproject.toml").read_text(encoding="utf-8"))["tool"]["ruff"]["builtins"]
  assert builtins
  for name in builtins:
    defined = [at for at, one in enumerate(NAMES) if name in defines(word(one))]
    read = [at for at, one in enumerate(NAMES) if name in reads(word(one))]
    assert defined, name
    assert any(first < later for first in defined for later in read), name
    assert name not in ENGINE, name


def bound(word: str) -> set[str]:
  """The names that a word binds at its top."""
  names: set[str] = set()
  for node in ast.parse(word).body:
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
      names.add(node.name)
    elif isinstance(node, ast.TypeAlias):
      names.add(node.name.id)
    elif isinstance(node, (ast.Assign, ast.AnnAssign)):
      targets = node.targets if isinstance(node, ast.Assign) else [node.target]
      names |= {one.id for one in targets if isinstance(one, ast.Name)}
  return names


@pytest.mark.parametrize("name", NAMES)
def test_word_binds_no_name_of_an_official_extension(name: str) -> None:
  """A word of the project that binds a name of an official extension changes that extension on its chains, as a BUDGET of text broke the catch-up of the root."""
  official = set().union(*(bound(one["word"]) for one in _monty.official()))
  assert bound(word(name)) & official == set()


def binders(text: str) -> list[tuple[str, bool]]:
  """Each name that a word binds at its top, with whether its binding reads that name, as a word that extends a list does."""
  found: list[tuple[str, bool]] = []
  for node in ast.parse(text).body:
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
      found.append((node.name, False))
    elif isinstance(node, ast.TypeAlias):
      found.append((node.name.id, False))
    elif isinstance(node, (ast.Assign, ast.AnnAssign)) and node.value is not None:
      targets = node.targets if isinstance(node, ast.Assign) else [node.target]
      read = {one.id for one in ast.walk(node.value) if isinstance(one, ast.Name)}
      found += [(one.id, one.id in read) for one in targets if isinstance(one, ast.Name)]
  return found


def test_no_two_words_of_the_project_bind_one_name_but_to_extend_it() -> None:
  """The words of the project load into the module of the root, so a name that two of them bind is one name, and the later word breaks the earlier."""
  homes: dict[str, str] = {}
  clashes: list[str] = []
  for name in NAMES:
    for bound_name, extends in binders(word(name)):
      if bound_name in homes and homes[bound_name] != name and not extends:
        clashes.append(f"{bound_name}: {homes[bound_name]} and {name}")
      homes.setdefault(bound_name, name)
  assert clashes == []


ALLOWED = ("extensions/conftest.py", ".furb/extensions/conftest.py")


def test_no_test_folder_of_an_extension_holds_a_conftest_that_shadows_the_conftest_of_the_suite() -> None:
  """A conftest.py in a test folder of an extension shadows the conftest module that extensions/conftest.py imports by name."""
  root = PROJECT.parent
  found = sorted(
    one.relative_to(root).as_posix()
    for place in (root / "extensions", PROJECT / "extensions")
    for one in place.rglob("conftest.py")
    if "node_modules" not in one.parts
  )
  shadows = [one for one in found if one not in ALLOWED]
  assert not shadows, (
    f"{shadows} shadows the conftest module that extensions/conftest.py imports by name, and breaks the tests of the"
    " main suite with no life: put its fixtures in .furb/extensions/conftest.py, or import its file by a unique name."
  )
