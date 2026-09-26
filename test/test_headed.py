"""headed, the header of a paragraph."""

from conftest import born, paragraphs
from furb import engine


async def test_the_header_of_a_paragraph() -> None:
  """The header of a paragraph: # and the name with no space between, then the text, whose later lines are comments."""
  assert engine.headed("bash1", "exited 0") == "#bash1 exited 0"
  assert engine.headed("read", "a.txt") == "#read a.txt"
  assert engine.headed("rung1") == "#rung1"
  assert engine.headed("prompt1", "count them\n\nall of them") == "#prompt1 count them\n#\n# all of them"
  _, _, root = born()
  asking = engine.prompt(int, "count them\n\nall of them", on=root)
  got = paragraphs(engine.turns(on=root))
  assert got[2] == engine.headed(asking, "count them\n\nall of them") + f"\n{asking}: Act[int] = Act('{asking}')"
  engine.cancel(asking)
