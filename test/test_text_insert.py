"""insert, the text with more text put before a line."""

import pytest

from furb.engine import Text

ONE = Text("/w/n.txt", "one\ntwo\n")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_insert_line_text_gives_a_new_text_with_the_text_put_at_that_line() -> None:
  """insert(line, text) gives a new text with the text put at that line."""
  assert ONE.insert(1, "top").content == "top\none\ntwo\n"
  assert ONE.insert(2, "mid").content == "one\nmid\ntwo\n"
  assert ONE.insert(3, "end").content == "one\ntwo\nend\n"
  assert ONE.insert(2, "a\nb\n").lines == ["one", "a", "b", "two"]
  assert ONE.insert(1, "top").before is ONE
