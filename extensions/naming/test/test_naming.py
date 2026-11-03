"""naming, the doctrine of the extension."""

from pathlib import Path

from conftest import STANDS
from extensions.conftest import naming
from furb import engine


async def test_naming_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """naming is the doctrine of the extension, a quote that says what it does."""
  sand, root = naming(tmp_path, STANDS[0])
  sand.script[root] = ["close(naming)"]
  doctrine = await engine.prompt(str, "doctrine", on=root)
  parts = ["name_session()", "first message", "automatic()", "naming", "first line", "root alone"]
  assert [part for part in parts if part not in doctrine] == []
