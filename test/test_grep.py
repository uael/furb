"""grep, the show of the lines that a pattern matches."""

from conftest import born, settle
from furb import engine


async def test_grep_pattern_is_the_show_of_the_lines_that_the_pattern_matches_each_with_its_number() -> None:
  """grep(pattern) is the show of the lines that the pattern matches, each with its number."""
  assert engine.grep("^b")(["a", "b", "c", "bb"]) == [2, 4]
  _, _, root = born("read('m.txt', grep('^[bd]'))\nclose(1)", files={"/w/m.txt": "a\nb\nc\nd\n"})
  assert await engine.prompt(int, "pick some", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#read m.txt\n# /w/m.txt, 0 known\n# 2 b\n# 4 d\n\n#prompt1 closed 1"
