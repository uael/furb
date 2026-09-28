"""grep, the show of the lines that a pattern matches."""

from conftest import born, settle
from furb import engine


async def test_grep_pattern_is_the_show_of_the_lines_that_the_pattern_matches_each_with_its_number() -> None:
  """grep(pattern) is the show of the lines that the pattern matches."""
  assert engine.grep("^b")(["a", "b", "c", "bb"]) == [2, 4]
  _, _, root = born("read('m.txt', grep('^[bd]'))\nclose(1)", files={"/w/m.txt": "a\nb\nc\nd\n"})
  assert await engine.prompt(int, "pick some", on=root) == 1
  await settle()
  told = "#read1\nread1_path = 'm.txt'\n<s:read1_text>\nb\nd</s:read1_text>"
  assert engine.turns(on=root)[-1][1] == f"{told}\n\n#prompt1 closed\nprompt1_value = 1"
