"""picked, the numbers of the lines that a show tells of a text."""

from conftest import born
from furb.engine import HIDDEN, TAIL, grep, picked, span


async def test_picked_gives_the_numbers_of_the_lines_that_a_show_tells_of_a_text() -> None:
  """picked gives the numbers of the lines that a show tells of a text, from the lines of the text: those that a show of the file names, or what a show that is a callable gives."""
  born()
  lines = [f"line {i}" for i in range(1, 301)]
  assert picked(span(-2, -1), lines) == [299, 300] and picked(TAIL, lines) == list(range(51, 301))
  assert picked(HIDDEN, lines) == [] and picked(grep("0$"), lines[:30]) == [10, 20, 30]
  assert picked(lambda got: [len(got)], lines) == [300]
