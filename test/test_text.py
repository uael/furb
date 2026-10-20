"""Text, a path, what stands at it, and the text it came from."""

import pytest

from furb.engine import Refused, Text

ONE = Text("/w/n.txt", "one\ntwo\nthree\n")


def test_a_text_its_path_what_stands_at_it_of_which_its_lines_are_the_lines() -> None:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from."""
  assert (ONE.path, ONE.content) == ("/w/n.txt", "one\ntwo\nthree\n")
  assert ONE.lines == ["one", "two", "three"]
  assert ONE.before is None
  after = ONE.delete(2, 2)
  assert (after.path, after.content, after.lines) == ("/w/n.txt", "one\nthree\n", ["one", "three"])
  assert after.before is ONE


def test_the_lines_of_a_text_derive_from_its_content() -> None:
  """The lines of a text derive from its content."""
  assert Text("/w/n.txt").lines == []
  assert Text("/w/n.txt", "one").lines == ["one"]
  assert ONE.lines == ONE.content.splitlines()
  assert ONE.append("four").lines == ["one", "two", "three", "four"]


def test_every_edit_of_it_gives_another_text_which_came_from_this_one() -> None:
  """Every edit of it gives another text, which came from this one."""
  made = ONE.append("four")
  assert made is not ONE and made.before is ONE and ONE.content == "one\ntwo\nthree\n"
  again = made.delete(1, 1)
  assert again.before is made and again.content == "two\nthree\nfour\n"


def test_a_method_of_a_text_that_has_the_name_of_a_method_of_str_gives_what_that_method_of_str_gives() -> None:
  """A method of a text that has the name of a method of str gives what that method of str gives, or is refused."""
  shared = {name for name in dir(Text) if not name.startswith("_")} & set(dir(str))
  assert shared == {"replace"}
  assert ONE.replace("two", "2").content == ONE.content.replace("two", "2") == "one\n2\nthree\n"
  assert ONE.replace("t", "T", 1).content == ONE.content.replace("t", "T", 1) == "one\nTwo\nthree\n"
  with pytest.raises(Refused):
    ONE.replace("four", "4")
