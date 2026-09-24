"""undo, the text before its last edits."""

from conftest import worded

ONE = "one = Text('/w/n.txt', 'one\\ntwo\\n')\n"
"""The start of a word that binds a text of two lines."""


async def test_undo_n_gives_a_new_text_without_its_last_n_edits() -> None:
  """undo(n) gives a new text without its last n edits."""
  word = ONE + (
    "made = one.append('three').append('four')\n"
    "close([made.content, made.undo().content, made.undo(2) is one, made.undo(2).content])"
  )
  assert await worded(word) == ["one\ntwo\nthree\nfour\n", "one\ntwo\nthree\n", True, "one\ntwo\n"]


async def test_the_text_before_its_last_n_edits_and_the_text_itself_when_it_came_from_none() -> None:
  """The text before its last n edits, and the text itself when it came from none."""
  word = ONE + (
    "close([one.undo() is one, one.undo(9) is one, one.append('three').undo(9) is one, "
    "one.append('three').undo(0).content])"
  )
  assert await worded(word) == [True, True, True, "one\ntwo\nthree\n"]
