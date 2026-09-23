"""commented, a text as comments."""

from conftest import STANDS, Sand, life
from furb import engine


async def test_the_text_as_comments() -> None:
  """The text as comments: # and a space before each line of it, and # alone for an empty line, so no line of it runs."""
  assert engine.commented("a\n\nb") == "# a\n#\n# b"
  assert engine.commented(3) == "# 3"
  assert engine.commented("") == "#"
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  word = engine.commented("raise ValueError('boom')\n\nk = 1")
  assert await engine.rung(word, on=root) is None
  assert "k" not in engine.modules[root]
