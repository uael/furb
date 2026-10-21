"""web, the doctrine of the extension."""

from pathlib import Path

from extensions.conftest import webbed
from furb import engine


async def test_web_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """web is the doctrine of the extension, a quote that says what web_search and web_fetch do, and that each gives an act of Text to await."""
  root, _ = webbed(tmp_path, {})
  doctrine = await engine.rung("close(web)", on=root)
  assert isinstance(doctrine, str)
  assert [
    part for part in ["web_search(query)", "web_fetch(url)", "act of Text to await"] if part not in doctrine
  ] == []
