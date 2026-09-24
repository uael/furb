"""delete, the text without the lines lo through hi."""

from conftest import worded


async def test_delete_lo_hi_gives_a_new_text_without_the_lines_lo_through_hi() -> None:
  """delete(lo, hi) gives a new text without the lines lo through hi."""
  word = (
    "one = Text('/w/n.txt', 'one\\ntwo\\nthree\\nfour\\n')\n"
    "close([one.delete(2, 3).content, one.delete(1, 1).lines, one.delete(1, 4).content, one.delete(2, 3).before is one])"
  )
  assert await worded(word) == ["one\nfour\n", ["two", "three", "four"], "", True]
