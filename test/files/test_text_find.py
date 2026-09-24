"""find, the numbers of the lines that a pattern matches."""

from furb.engine import Text, grep

ONE = Text("/w/n.txt", "one\ntwo\nthree\n")


def test_find_pattern_gives_the_numbers_of_the_lines_that_the_pattern_matches() -> None:
  """find(pattern) gives the numbers of the lines that the pattern matches."""
  assert ONE.find("^t") == [2, 3]
  assert ONE.find("one") == [1]
  assert ONE.find("four") == []
  assert Text("/w/n.txt").find("one") == []


def test_the_numbers_of_the_lines_the_pattern_matches_which_is_what_grep_picks_of_them() -> None:
  """The numbers of the lines the pattern matches, which is what grep picks of them."""
  assert ONE.find("^t") == grep("^t")(ONE.lines) == [2, 3]
  assert ONE.find("e$") == grep("e$")(ONE.lines) == [1, 3]
