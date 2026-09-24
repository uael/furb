"""showing, what a paragraph shows of what a door answered."""

from conftest import FILES, life, settle, sown
from furb import engine

NUMS = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('nums://'):\n"
  "        yield 'done', qid, [1, 2]\n"
  "\n"
  "act('nums', '', kept)\n"
  "read('nums://a')\n"
  "close(1)\n"
)
"""A word of a rung that opens a door of its own, which answers a read with a list, and reads it."""


async def test_what_a_paragraph_shows_of_what_a_door_answered() -> None:
  """What a paragraph shows of what a door answered: the text by the lines the model has not seen, and anything that is no text as a comment of how python shows it."""
  sand = sown(FILES)
  _, root = life(sand)
  word = (
    "(path, content, show), = showing(Text('/w/n.txt', 'one\\ntwo\\n'), HEAD)\n"
    "close([path, content, show is HEAD, showing([1, 2], HEAD), showing(None, HEAD), showing('a\\n\\nb', HEAD)])"
  )
  assert await engine.rung(word, on=root) == [
    "/w/n.txt",
    "one\ntwo\n",
    True,
    ["# [1, 2]"],
    ["# None"],
    ["# 'a\\n\\nb'"],
  ]
  sand.script[root] = [NUMS]
  assert await engine.prompt(int, "a door of my own", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#read nums://a\n# [1, 2]\n\n#prompt1 closed 1"
