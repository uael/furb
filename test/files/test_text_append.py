"""append, the text with more text at its end."""

from conftest import worded


async def test_append_text_gives_a_new_text_with_the_text_added_at_its_end() -> None:
  """append(text) gives a new text with the text added at its end."""
  word = (
    "one = Text('/w/n.txt', 'one\\ntwo\\n')\n"
    "close([one.append('three').content, one.append('three\\nfour\\n').lines, "
    "Text('/w/n.txt').append('one').content, one.append('three').before is one])"
  )
  assert await worded(word) == ["one\ntwo\nthree\n", ["one", "two", "three", "four"], "one\n", True]
