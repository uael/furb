"""Each word of the extensions of the project passes the gate on the root chain, after the words that play before it."""

import ast
import inspect
import json
import re
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
