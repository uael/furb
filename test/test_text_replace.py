"""replace, the text with one string put for another."""

import pytest

from furb.engine import Refused, Text

ONE = Text("/w/n.txt", "one\ntwo\none\n")


def test_replace_old_new_count_gives_a_new_text_with_the_edit_added() -> None:
  """replace(old, new, count) gives a new text with the edit added, as str.replace gives with the same count."""
  assert ONE.replace("one", "1").content == "1\ntwo\n1\n"
  assert ONE.replace("one", "1", 1).content == "1\ntwo\none\n"
  assert ONE.replace("one", "1", count=2).content == "1\ntwo\n1\n"
  assert ONE.replace("one", "1").before is ONE
  assert ONE.content == "one\ntwo\none\n"


def test_a_replace_of_a_string_that_the_text_does_not_hold_is_refused() -> None:
  """A replace of a string that the text does not hold is refused, and the refusal names the path and the string, as for an edit of lines that are not there."""
  with pytest.raises(Refused, match=r"^/w/n\.txt no 'three'$"):
    ONE.replace("three", "3")
  with pytest.raises(Refused, match=r"^/w/n\.txt no 'One'$"):
    ONE.replace("One", "1", 1)
