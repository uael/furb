"""headed, the header of a paragraph."""

from conftest import STANDS, Sand, life, paragraphs
from furb import engine


async def test_the_header_of_a_paragraph() -> None:
  """The header of a paragraph: # and the name with no space between, then the text, whose later lines are comments."""
  assert engine.headed("prompt1", "closed 3") == "#prompt1 closed 3"
  assert engine.headed("clock", "1001.0") == "#clock 1001.0"
  assert engine.headed("rung1") == "#rung1"
  assert engine.headed("prompt1", "count them\n\nall of them") == "#prompt1 count them\n#\n# all of them"
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  asking = engine.prompt(int, "count them\n\nall of them", on=root)
  got = paragraphs(engine.turns(on=root))
  assert got[2] == engine.headed(asking, "count them\n\nall of them") + f"\n{asking}: Act[int] = Act('{asking}')"
  engine.cancel(asking)
