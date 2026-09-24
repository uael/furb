"""append, the text with more text at its end."""

from furb.engine import Text

ONE = Text("/w/n.txt", "one\ntwo\n")


def test_append_text_gives_a_new_text_with_the_text_added_at_its_end() -> None:
  """append(text) gives a new text with the text added at its end."""
  assert ONE.append("three").content == "one\ntwo\nthree\n"
  assert ONE.append("three\nfour\n").lines == ["one", "two", "three", "four"]
  assert Text("/w/n.txt").append("one").content == "one\n"
  assert ONE.append("three").before is ONE
