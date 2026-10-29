"""LORE, the doctrine of the extension."""

import ast
from pathlib import Path

from conftest import of
from extensions.conftest import extended
from furb import engine

LORE = next(
  ast.literal_eval(one.value)
  for one in ast.parse((Path(__file__).parents[1] / "ouroboros.py").read_text(encoding="utf-8")).body
  if isinstance(one, ast.Assign) and ast.unparse(one.targets[0]) == "LORE"
)
"""LORE as the word of the extension binds it."""


async def test_lore_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """LORE is the doctrine of the extension, which teaches the three ways to extend furb and when to take each, and which holds no law."""
  _, root = extended("ouroboros", tmp_path)
  await engine.rung("ouroboros()", on=root)
  assert of(engine.turns(on=root), "ouroboros") == [f"#ouroboros1\n<s:ouroboros1_lore>\n{LORE}</s:ouroboros1_lore>"]
