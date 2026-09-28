"""shown, the lines of a text that a show picks."""

from conftest import THREE
from furb import engine
from furb.engine import HEAD, HIDDEN, Text, grep, span


def test_the_lines_of_a_text_that_a_show_picks() -> None:
  """The lines of a text that a show picks, as they are, one on each line."""
  assert engine.shown(THREE, HEAD) == "one\ntwo\nthree"
  assert engine.shown(THREE, span(2, 3)) == "two\nthree"
  assert engine.shown(THREE, grep("^t")) == "two\nthree"
  assert engine.shown(THREE, span(-1, -1)) == "three"
  assert engine.shown(THREE, HIDDEN) == ""
  assert engine.shown(Text("/w/m.txt", "  a\n\n# b\n"), HEAD) == "  a\n\n# b"
