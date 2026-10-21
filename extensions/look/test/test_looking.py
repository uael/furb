"""LOOKING, the doctrine of the extension."""

import ast
from pathlib import Path

from conftest import of
from extensions.conftest import extended
from furb import engine

LOOKING = next(
  ast.literal_eval(one.value)
  for one in ast.parse((Path(__file__).parents[1] / "look.py").read_text(encoding="utf-8")).body
  if isinstance(one, ast.Assign) and ast.unparse(one.targets[0]) == "LOOKING"
)
"""LOOKING as the word of the extension binds it."""


async def test_looking_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """LOOKING is the doctrine of the extension, which teaches a model to look at the files of its task in a first word and to change them and close in a later word, and which holds no law."""
  _, root = extended("look", tmp_path, lives=True, actor="local:qwen")
  assert of(engine.turns(on=root), "look") == [f"#look1\n<s:look1_lore>\n{LOOKING}</s:look1_lore>"]
