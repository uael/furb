"""insert, the text with more text put before a line."""

from conftest import TWO


def test_insert_line_text_gives_a_new_text_with_the_text_put_at_that_line() -> None:
  """insert(line, text) gives a new text with the text put at that line."""
  assert TWO.insert(1, "top").content == "top\none\ntwo\n"
  assert TWO.insert(2, "mid").content == "one\nmid\ntwo\n"
  assert TWO.insert(3, "end").content == "one\ntwo\nend\n"
  assert TWO.insert(2, "a\nb\n").lines == ["one", "a", "b", "two"]
  assert TWO.insert(1, "top").before is TWO
