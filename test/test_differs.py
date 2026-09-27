"""differs, the show of the lines that differ, which is what a write shows."""

from conftest import born, settle
from furb import engine

GROWS = (
  "def kept(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('write', qid, _, _, Text(path=path, content=content)) if path.startswith('note://'):\n"
  "        yield 'done', qid, Text(path, 'one\\n' + content)\n"
  "\n"
  "act('note', '', kept)\n"
  "close(write(Text('note://a', 'two\\n')).content)\n"
)
"""A word of a rung that opens a door of its own, which answers a write with one line more than it was given."""


async def test_differs_lines_is_the_show_of_the_lines_that_differ_from_the_lines_it_holds() -> None:
  """differs(lines) is the show of the lines that differ from the lines it holds, which is what a write shows of what came back."""
  assert engine.differs(["one", "two"])(["one", "new", "two"]) == [2, 3]
  _, _, root = born(GROWS)
  assert await engine.prompt(str, "a door of my own", on=root) == "one\ntwo\n"
  await settle()
  told = "#write note://a\n# note://a, 0 known\n# 1 one\n# 2 two"
  assert engine.turns(on=root)[-1][1] == f"{told}\n\n#prompt1 closed 'one\\ntwo\\n'"
