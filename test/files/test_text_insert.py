"""insert, the text with more text put before a line."""

from conftest import worded


async def test_insert_line_text_gives_a_new_text_with_the_text_put_at_that_line() -> None:
  """insert(line, text) gives a new text with the text put at that line."""
  word = (
    "one = Text('/w/n.txt', 'one\\ntwo\\n')\n"
    "close([one.insert(1, 'top').content, one.insert(2, 'mid').content, one.insert(3, 'end').content, "
    "one.insert(2, 'a\\nb\\n').lines, one.insert(1, 'top').before is one])"
  )
  assert await worded(word) == ["top\none\ntwo\n", "one\nmid\ntwo\n", "one\ntwo\nend\n", ["one", "a", "b", "two"], True]
