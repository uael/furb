"""delete, the text without the lines lo through hi."""

import pytest

from furb.engine import Text

ONE = Text("/w/n.txt", "one\ntwo\nthree\nfour\n")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
def test_delete_lo_hi_gives_a_new_text_without_the_lines_lo_through_hi() -> None:
  """delete(lo, hi) gives a new text without the lines lo through hi."""
  assert ONE.delete(2, 3).content == "one\nfour\n"
  assert ONE.delete(1, 1).lines == ["two", "three", "four"]
  assert ONE.delete(1, 4).content == ""
  assert ONE.delete(2, 3).before is ONE
