"""undo, the text before its last edits."""

from conftest import TWO


def test_undo_n_gives_a_new_text_without_its_last_n_edits() -> None:
  """undo(n) gives a new text without its last n edits."""
  made = TWO.append("three").append("four")
  assert made.content == "one\ntwo\nthree\nfour\n"
  assert made.undo().content == "one\ntwo\nthree\n"
  assert made.undo(2) is TWO and made.undo(2).content == "one\ntwo\n"


def test_the_text_before_its_last_n_edits_and_the_text_itself_when_it_came_from_none() -> None:
  """The text before its last n edits, and the text itself when it came from none."""
  assert TWO.undo() is TWO
  assert TWO.undo(9) is TWO
  assert TWO.append("three").undo(9) is TWO
  assert TWO.append("three").undo(0).content == "one\ntwo\nthree\n"
