"""showing, what a paragraph shows of what a door answered."""

from conftest import life, settle, sown
from furb import engine
from furb.engine import HEAD, Text

NUMS = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('nums://'):\n"
  "        say('done', qid, [1, 2])\n"
  "\n"
  "act('nums', '', kept)\n"
  "read('nums://a')\n"
  "close(1)\n"
)
"""A word of a rung that opens a door of its own, which answers a read with a list, and reads it."""


async def test_what_a_paragraph_shows_of_what_a_door_answered() -> None:
  """What a paragraph shows of what a door answered: the text by the lines the model has not seen, and anything that is no text as a comment of how python shows it."""
  one = Text("/w/n.txt", "one\ntwo\n")
  assert engine.showing(one, HEAD) == [(one, HEAD)]
  assert engine.showing([1, 2], HEAD) == ["# [1, 2]"]
  assert engine.showing(None, HEAD) == ["# None"]
  assert engine.showing("a\n\nb", HEAD) == ["# 'a\\n\\nb'"]
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [NUMS]
  assert await engine.prompt(int, "a door of my own", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#read nums://a\n# [1, 2]\n\n#prompt1 closed 1"
