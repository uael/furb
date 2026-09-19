"""Text, a path, what stands at it, and the text it came from."""

import pytest

from furb.engine import Text

ONE = Text("/w/n.txt", "one\ntwo\nthree\n")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_a_text_its_path_what_stands_at_it_of_which_its_lines_are_the_lines() -> None:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from."""
  assert (ONE.path, ONE.content) == ("/w/n.txt", "one\ntwo\nthree\n")
  assert ONE.lines == ["one", "two", "three"]
  assert ONE.before is None
  after = ONE.delete(2, 2)
  assert (after.path, after.content, after.lines) == ("/w/n.txt", "one\nthree\n", ["one", "three"])
  assert after.before is ONE


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_the_lines_of_a_text_derive_from_its_content() -> None:
  """The lines of a text derive from its content."""
  assert Text("/w/n.txt").lines == []
  assert Text("/w/n.txt", "one").lines == ["one"]
  assert ONE.lines == ONE.content.splitlines()
  assert ONE.append("four").lines == ["one", "two", "three", "four"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_every_edit_of_it_gives_another_text_which_came_from_this_one() -> None:
  """Every edit of it gives another text, which came from this one."""
  made = ONE.append("four")
  assert made is not ONE and made.before is ONE and ONE.content == "one\ntwo\nthree\n"
  again = made.delete(1, 1)
  assert again.before is made and again.content == "two\nthree\nfour\n"
