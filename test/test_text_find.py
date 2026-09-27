"""find, the numbers of the lines that a pattern matches."""

from conftest import THREE
from furb.engine import Text, grep


def test_find_pattern_gives_the_numbers_of_the_lines_that_the_pattern_matches() -> None:
  """find(pattern) gives the numbers of the lines that the pattern matches."""
  assert THREE.find("^t") == [2, 3]
  assert THREE.find("one") == [1]
  assert THREE.find("four") == []
  assert Text("/w/n.txt").find("one") == []


def test_the_numbers_of_the_lines_the_pattern_matches_which_is_what_grep_picks_of_them() -> None:
  """The numbers of the lines the pattern matches, which is what grep picks of them."""
  assert THREE.find("^t") == grep("^t")(THREE.lines) == [2, 3]
  assert THREE.find("e$") == grep("e$")(THREE.lines) == [1, 3]
