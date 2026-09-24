"""find, the numbers of the lines that a pattern matches."""

from conftest import worded

ONE = "one = Text('/w/n.txt', 'one\\ntwo\\nthree\\n')\n"
"""The start of a word that binds a text of three lines."""


async def test_find_pattern_gives_the_numbers_of_the_lines_that_the_pattern_matches() -> None:
  """find(pattern) gives the numbers of the lines that the pattern matches."""
  word = ONE + "close([one.find('^t'), one.find('one'), one.find('four'), Text('/w/n.txt').find('one')])"
  assert await worded(word) == [[2, 3], [1], [], []]


async def test_the_numbers_of_the_lines_the_pattern_matches_which_is_what_grep_picks_of_them() -> None:
  """The numbers of the lines the pattern matches, which is what grep picks of them."""
  word = ONE + "close([[one.find(x), grep(x)(one.lines)] for x in ('^t', 'e$')])"
  assert await worded(word) == [[[2, 3], [2, 3]], [[1, 3], [1, 3]]]
