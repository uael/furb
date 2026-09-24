"""edit, the text with one more edit of its lines."""

import pytest

from furb.engine import Refused, Text

ONE = Text("/w/n.txt", "one\ntwo\nthree\n")


def test_the_text_with_one_more_edit_an_edit_whose_lines_are_not_there_is_refused() -> None:
  """The text with one more edit; an edit whose lines are not there is refused."""
  made = ONE.edit(2, 2, ["TWO"])
  assert made.content == "one\nTWO\nthree\n" and made.before is ONE
  with pytest.raises(Refused, match=r"/w/n\.txt no lines 9:9"):
    ONE.edit(9, 9, ["x"])
  with pytest.raises(Refused, match=r"/w/n\.txt no lines 1:4"):
    ONE.edit(1, 4, ["x"])


def test_the_edits_that_say_themselves_are_one_edit_each() -> None:
  """The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines."""
  assert ONE.replace("one", "1").content == "1\ntwo\nthree\n"
  assert ONE.append("four").content == "one\ntwo\nthree\nfour\n"
  assert ONE.insert(1, "top").content == "top\none\ntwo\nthree\n"
  assert ONE.delete(2, 2).content == "one\nthree\n"
  for made in (ONE.replace("one", "1"), ONE.append("four"), ONE.insert(1, "top"), ONE.delete(2, 2)):
    assert made.before is ONE and made.undo() is ONE
