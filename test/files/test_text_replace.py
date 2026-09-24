"""replace, the text with one string put for another."""

from conftest import worded


async def test_replace_old_new_once_gives_a_new_text_with_the_edit_added() -> None:
  """replace(old, new, once) gives a new text with the edit added."""
  word = (
    "one = Text('/w/n.txt', 'one\\ntwo\\none\\n')\n"
    "close([one.replace('one', '1').content, one.replace('one', '1', once=True).content, "
    "one.replace('one', '1').before is one, one.content])"
  )
  assert await worded(word) == ["1\ntwo\n1\n", "1\ntwo\none\n", True, "one\ntwo\none\n"]
