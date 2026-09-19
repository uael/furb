"""showing, what a tag shows of what a door answered."""

from conftest import life, settle, sown, tags
from furb import engine
from furb.engine import HEAD, Text

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


async def test_what_a_tag_shows_of_what_a_door_answered() -> None:
  """What a tag shows of what a door answered: the text by the lines the model has not seen, and anything that is no text as python shows it."""
  one = Text("/w/n.txt", "one\ntwo\n")
  assert engine.showing(one, HEAD) == [(one, HEAD)]
  assert engine.showing([1, 2], HEAD) == "[1, 2]"
  assert engine.showing(None, HEAD) == "None"
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [NUMS]
  assert await engine.prompt(int, "a door of my own", on=root) == 1
  await settle()
  assert [tag[2] for tag in tags(engine.turns(on=root), "read")] == ["[1, 2]"]
