"""shown, the lines of a text that a show picks."""

from conftest import THREE
from furb import engine
from furb.engine import HEAD, HIDDEN, Text, grep, span


def test_the_lines_of_a_text_that_a_show_picks() -> None:
  """The lines of a text that a show picks, as they are, one on each line, each cut at its first 1000 characters and then marked with its length, as ... (1045 chars), so a long line of a command or of a file does not fill the turns and a cut line does not read as the end of the text."""
  wide = Text("/w/w.txt", "a" * 1000 + "\n" + "b" * 1001 + "\n" + "c" * 1045)
  assert engine.shown(wide, HEAD).split("\n") == [
    "a" * 1000,
    "b" * 1000 + "... (1001 chars)",
    "c" * 1000 + "... (1045 chars)",
  ]
  assert engine.shown(THREE, HEAD) == "one\ntwo\nthree"
  assert engine.shown(THREE, span(2, 3)) == "two\nthree"
  assert engine.shown(THREE, grep("^t")) == "two\nthree"
  assert engine.shown(THREE, span(-1, -1)) == "three"
  assert engine.shown(THREE, HIDDEN) == ""
  assert engine.shown(Text("/w/m.txt", "  a\n\n# b\n"), HEAD) == "  a\n\n# b"
