"""replace, the text with one string put for another."""

from furb.engine import Text

ONE = Text("/w/n.txt", "one\ntwo\none\n")


def test_replace_old_new_once_gives_a_new_text_with_the_edit_added() -> None:
  """replace(old, new, once) gives a new text with the edit added."""
  assert ONE.replace("one", "1").content == "1\ntwo\n1\n"
  assert ONE.replace("one", "1", once=True).content == "1\ntwo\none\n"
  assert ONE.replace("one", "1").before is ONE
  assert ONE.content == "one\ntwo\none\n"
