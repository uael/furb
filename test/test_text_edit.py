"""edit, the text with one more edit of its lines."""

import pytest

from conftest import THREE
from furb.engine import Refused, Text


def test_the_text_with_one_more_edit_an_edit_whose_lines_are_not_there_is_refused() -> None:
  """The text with one more edit; an edit whose lines are not there is refused."""
  made = THREE.edit(2, 2, ["TWO"])
  assert made.content == "one\nTWO\nthree\n" and made.before is THREE
  with pytest.raises(Refused, match=r"/w/n\.txt no lines 9:9"):
    THREE.edit(9, 9, ["x"])
  with pytest.raises(Refused, match=r"/w/n\.txt no lines 1:4"):
    THREE.edit(1, 4, ["x"])


def test_the_edits_that_say_themselves_are_one_edit_each() -> None:
  """The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines."""
  assert THREE.replace("one", "1").content == "1\ntwo\nthree\n"
  assert THREE.append("four").content == "one\ntwo\nthree\nfour\n"
  assert THREE.insert(1, "top").content == "top\none\ntwo\nthree\n"
  assert THREE.delete(2, 2).content == "one\nthree\n"
  for made in (THREE.replace("one", "1"), THREE.append("four"), THREE.insert(1, "top"), THREE.delete(2, 2)):
    assert made.before is THREE and made.undo() is THREE


def test_an_edit_ends_the_last_line_of_the_text_with_a_line_break() -> None:
  """An edit of lines, as an append, an insert or a delete, ends the last line of the text with a line break, so what it puts after a text that ends with none starts a line of its own."""
  bare = Text("/w/n.txt", "one\ntwo")
  assert bare.append("three").content == "one\ntwo\nthree\n"
  assert bare.insert(3, "x\ny").lines == ["one", "two", "x", "y"]
  assert bare.edit(1, 1, ["ONE"]).content == "ONE\ntwo\n"
  assert Text("/w/n.txt", "a\r\nb").append("c").content == "a\r\nb\nc\n"
  assert Text("/w/n.txt").append("x").content == "x\n"
  assert bare.replace("two", "2").content == "one\n2"
