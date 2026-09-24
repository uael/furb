"""grep, the show of the lines that a pattern matches."""

from conftest import FILES, STANDS, Sand, life, settle
from furb import engine


async def test_grep_pattern_is_the_show_of_the_lines_that_the_pattern_matches_each_with_its_number() -> None:
  """grep(pattern) is the show of the lines that the pattern matches, each with its number."""
  sand = Sand(files={"/w/m.txt": "a\nb\nc\nd\n"}, stands=STANDS, words=FILES)
  _, root = life(sand)
  assert await engine.rung("close(grep('^b')(['a', 'b', 'c', 'bb']))", on=root) == [2, 4]
  sand.script[root] = ["read('m.txt', grep('^[bd]'))\nclose(1)"]
  assert await engine.prompt(int, "pick some", on=root) == 1
  await settle()
  assert engine.turns(on=root)[-1][1] == "#read m.txt\n# /w/m.txt, 0 known\n# 2 b\n# 4 d\n\n#prompt1 closed 1"
